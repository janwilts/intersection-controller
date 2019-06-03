use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Acquire;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{after, Receiver};

use crate::config::groups::{Group as ConfigGroup, Groups as ConfigGroups};
use crate::intersections::component::{Component, ComponentKind, ComponentUid};
use crate::intersections::group::{ArcGroup, GroupKind};
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;
use crate::intersections::sensor::SensorState;

pub struct TrafficLightsRunner {
    intersection: ArcIntersection,
    groups_config: ConfigGroups,

    stop: Arc<AtomicBool>,
    stop_channel: Receiver<()>,
}

impl TrafficLightsRunner {
    pub fn new(
        intersection: ArcIntersection,
        groups_config: ConfigGroups,
        stop: Arc<AtomicBool>,
        stop_channel: Receiver<()>,
    ) -> Self {
        Self {
            intersection,
            groups_config,
            stop,
            stop_channel,
        }
    }

    pub fn run(&self) -> Result<(), failure::Error> {
        info!("Running traffic lights");

        let state_receiver = self.intersection.read().unwrap().state_receiver.clone();
        let jam_sensor = self
            .intersection
            .read()
            .unwrap()
            .find_sensor(ComponentUid::new(
                GroupKind::MotorVehicle,
                14,
                ComponentKind::Sensor,
                1,
            ))
            .unwrap();

        loop {
            select! {
                recv(self.stop_channel) -> _ => {},
                recv(state_receiver) -> _ => {},
                recv(after(Duration::from_millis(100))) -> _ => {},
            }

            if self.stop.load(Acquire) {
                break;
            }

            if jam_sensor
                .read()
                .unwrap()
                .triggered_for(Duration::from_secs(3), SensorState::High)
            {
                warn!("A wild traffic jam appeared, blocking other traffic.");

                for group in self.intersection.read().unwrap().blockable_groups() {
                    group.write().unwrap().block = true;
                }
            } else {
                for group in self.intersection.read().unwrap().blockable_groups() {
                    group.write().unwrap().block = false;
                }
            }

            let runnables = self.intersection.read().unwrap().get_runnables().unwrap();

            if runnables.is_empty() {
                continue;
            }

            info!("Starting a traffic lights phase");

            let by_kind = self.runnables_by_group_kind(runnables);
            let all_times = self.get_times(by_kind.keys().cloned().collect());

            let mut handles = vec![];

            for (kind, runnables) in by_kind {
                let times = all_times[&kind].clone();

                let stop = Arc::clone(&self.stop);
                let stop_channel = self.stop_channel.clone();

                handles.push(thread::spawn(move || {
                    info!("Phase {} for kind {}", LightState::Proceed, kind);

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light
                                .write()
                                .unwrap()
                                .set_state(LightState::Proceed)
                                .unwrap_or_else(|e| error!("{}", e));
                        }
                    }
                    select! {
                        recv(after(times[&LightState::Proceed])) -> _ => {},
                        recv(stop_channel) -> _ => {},
                    };

                    if stop.load(Acquire) {
                        return;
                    }

                    info!("Phase {} for kind {}", LightState::Transitioning, kind);

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light
                                .write()
                                .unwrap()
                                .set_state(LightState::Transitioning)
                                .unwrap_or_else(|e| error!("{}", e));
                        }
                    }

                    select! {
                        recv(after(times[&LightState::Transitioning])) -> _ => {},
                        recv(stop_channel) -> _ => {},
                    };

                    if stop.load(Acquire) {
                        return;
                    }

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light
                                .write()
                                .unwrap()
                                .set_state(LightState::Prohibit)
                                .unwrap_or_else(|e| error!("{}", e));
                        }

                        group
                            .write()
                            .unwrap()
                            .reset_score()
                            .unwrap_or_else(|e| error!("{}", e));
                    }
                }));
            }

            for handle in handles {
                handle.join().expect("Could not join sub threads");
            }

            if self.stop.load(Acquire) {
                break;
            }

            select! {
                recv(after(Duration::from_secs(1))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };

            if self.stop.load(Acquire) {
                break;
            }
        }

        warn!("Stopping traffic lights runner");

        Ok(())
    }

    fn runnables_by_group_kind(
        &self,
        runnables: Vec<ArcGroup>,
    ) -> HashMap<GroupKind, Vec<ArcGroup>> {
        let mut map: HashMap<GroupKind, Vec<ArcGroup>> = HashMap::new();

        for runnable in runnables {
            let kind = runnable.read().unwrap().id.kind;

            if let Some(runnables) = map.get_mut(&kind) {
                runnables.push(Arc::clone(&runnable));
            } else {
                map.insert(kind, vec![Arc::clone(&runnable)]);
            }
        }

        map
    }

    fn get_times(
        &self,
        kinds: Vec<GroupKind>,
    ) -> HashMap<GroupKind, HashMap<LightState, Duration>> {
        let largest_total_time = self.largest_total_time(kinds.clone());

        let mut map: HashMap<GroupKind, HashMap<LightState, Duration>> = HashMap::new();

        for kind in kinds {
            let config = self.group_config(kind);

            if config.is_none() {
                continue;
            }

            let config = config.unwrap();
            let total_time = self.total_time(kind);

            let mut state_map: HashMap<LightState, Duration> = HashMap::new();

            state_map.insert(
                LightState::Proceed,
                Duration::from_millis(config.min_go_time as u64)
                    + (largest_total_time - total_time),
            );
            state_map.insert(
                LightState::Transitioning,
                Duration::from_millis(config.min_transition_time as u64),
            );

            map.insert(kind, state_map);
        }

        map
    }

    fn largest_total_time(&self, kinds: Vec<GroupKind>) -> Duration {
        let mut largest: Duration = Duration::from_millis(0);

        for kind in kinds {
            if self.total_time(kind) > largest {
                largest = self.total_time(kind);
            }
        }

        largest
    }

    fn total_time(&self, kind: GroupKind) -> Duration {
        let config = self.group_config(kind).unwrap();

        Duration::from_millis(config.min_go_time as u64 + config.min_transition_time as u64)
    }

    fn group_config(&self, kind: GroupKind) -> Option<&ConfigGroup> {
        for group in &self.groups_config.groups {
            if group.kind == kind.to_string() {
                return Some(group);
            }
        }

        None
    }
}
