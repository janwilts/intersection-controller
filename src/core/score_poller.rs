use std::thread;
use std::time::Duration;

use crate::intersections::component::Component;
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::sensor::SensorState;

pub struct ScorePoller {
    traffic_lights: ArcIntersection,
}

impl ScorePoller {
    pub fn new(traffic_lights: ArcIntersection) -> Self {
        Self { traffic_lights }
    }

    pub fn run(&self) {
        loop {
            for group in self.traffic_lights.read().unwrap().groups() {
                let mut should_increase = false;

                for sensor in group.read().unwrap().sensors.values() {
                    let sensor = sensor.read().unwrap();

                    if sensor.state() != SensorState::High {
                        continue;
                    }

                    should_increase = true;

                    //if sensor.state().is_high_for_one_second() && sensor.distance > 0 {
                    //    group.score += sensor.distance;
                    //} else {
                    //    group.score += 1;
                    //}
                }

                if should_increase {
                    group.write().unwrap().increase_score(1);
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}
