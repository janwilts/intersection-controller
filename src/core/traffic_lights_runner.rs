use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::config::groups::{Group as ConfigGroup, Groups as ConfigGroups};
use crate::intersections::component::{Component, ComponentKind};
use crate::intersections::group::{ArcGroup, GroupKind};
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;

pub struct TrafficLightsRunner {
    intersection: ArcIntersection,

    groups_config: ConfigGroups,
}

impl TrafficLightsRunner {
    pub fn new(intersection: ArcIntersection, groups_config: ConfigGroups) -> Self {
        Self {
            intersection,
            groups_config,
        }
    }

    pub fn run(&self) {
        loop {
            let runnables = self.intersection.read().unwrap().get_runnables().unwrap();

            if runnables.is_empty() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let by_kind = self.runnables_by_group_kind(runnables);
            let all_times = self.get_times(by_kind.keys().map(|k| k.clone()).collect());

            let mut handles = vec![];

            for (kind, runnables) in by_kind {
                let times = all_times.get(&kind).unwrap().clone();

                handles.push(thread::spawn(move || {
                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light.write().unwrap().set_state(LightState::Proceed);
                        }
                    }

                    thread::sleep(times.get(&LightState::Proceed).unwrap().clone());

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light.write().unwrap().set_state(LightState::Transitioning);
                        }
                    }

                    thread::sleep(times.get(&LightState::Transitioning).unwrap().clone());

                    for group in runnables.clone() {
                        for light in group.read().unwrap().lights.values() {
                            light.write().unwrap().set_state(LightState::Prohibit);
                        }

                        group.write().unwrap().reset_score();
                    }
                }));
            }

            for handle in handles {
                handle.join();
            }

            thread::sleep(Duration::from_secs(2));
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
}
