use std::thread;
use std::time::Duration;

use crate::intersections::component::Component;
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;

pub struct TrafficLightsRunner {
    intersection: ArcIntersection,
}

impl TrafficLightsRunner {
    pub fn new(intersection: ArcIntersection) -> Self {
        Self { intersection }
    }

    pub fn run(&self) {
        loop {
            let runnables = self.intersection.read().unwrap().get_runnables().unwrap();

            if runnables.is_empty() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            for group in runnables.clone() {
                for light in group.read().unwrap().lights.values() {
                    light.write().unwrap().set_state(LightState::Proceed);
                }
            }

            thread::sleep(Duration::from_secs(10));

            for group in runnables.clone() {
                for light in group.read().unwrap().lights.values() {
                    light.write().unwrap().set_state(LightState::Transitioning);
                }
            }

            thread::sleep(Duration::from_secs(4));

            for group in runnables.clone() {
                for light in group.read().unwrap().lights.values() {
                    light.write().unwrap().set_state(LightState::Prohibit);
                }

                group.write().unwrap().reset_score();
            }

            thread::sleep(Duration::from_secs(2));
        }
    }
}
