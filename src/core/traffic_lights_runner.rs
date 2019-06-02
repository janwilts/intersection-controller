use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Acquire;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{after, Receiver};

use crate::config::groups::{Group as ConfigGroup, Groups as ConfigGroups};
use crate::intersections::component::Component;
use crate::intersections::group::{ArcGroup, GroupKind};
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;

#[derive(Fail, Debug)]
#[fail(display = "Stopped traffic lights runner")]
pub struct TrafficLightsRunnerStop {}

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
        let state_receiver = self.intersection.read().unwrap().state_receiver.clone();

        loop {
            let runnables = self.intersection.read().unwrap().get_runnables().unwrap();

            if runnables.is_empty() {
                select! {
                    recv(state_receiver) -> _ => {},
                    recv(self.stop_channel) -> _ => {},
                }

                self.break_on_stop()?;

                continue;
            }

            let by_kind = self.runnables_by_group_kind(runnables);
            let all_times = self.get_times(by_kind.keys().map(|k| k.clone()).collect());

            let mut handles = vec![];

            for (kind, runnables) in by_kind {
                let times = all_times.get(&kind).unwrap().clone();

                let stop = Arc::clone(&self.stop);
                let stop_channel = self.stop_channel.clone();

                handles.push(thread::spawn(move || {
                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light.write().unwrap().set_state(LightState::Proceed);
                        }
                    }
                    select! {
                        recv(after(times.get(&LightState::Proceed).unwrap().clone())) -> _ => {},
                        recv(stop_channel) -> _ => {},
                    };

                    if stop.load(Acquire) {
                        return;
                    }

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light.write().unwrap().set_state(LightState::Transitioning);
                        }
                    }

                    select! {
                        recv(after(times.get(&LightState::Transitioning).unwrap().clone())) -> _ => {},
                        recv(stop_channel) -> _ => {},
                    };

                    if stop.load(Acquire) {
                        return;
                    }

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light.write().unwrap().set_state(LightState::Prohibit);
                        }

                        group.write().unwrap().reset_score();
                    }
                }));
            }

            for handle in handles {
                handle.join().expect("Could not join sub threads");
            }

            self.break_on_stop()?;

            select! {
                recv(after(Duration::from_secs(1))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };

            self.break_on_stop()?;
        }
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

            if let None = config {
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

    fn break_on_stop(&self) -> Result<(), failure::Error> {
        if self.stop.load(Acquire) {
            warn!("Stopping traffic lights runner.");

            return Err(TrafficLightsRunnerStop {}.into());
        }

        Ok(())
    }
}
