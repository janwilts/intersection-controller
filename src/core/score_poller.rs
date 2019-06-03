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

    pub fn run(&self) -> Result<(), failure::Error> {
        loop {
            for group in self.traffic_lights.read().unwrap().groups() {
                let mut score = group.read().unwrap().score;

                for sensor in group.read().unwrap().sensors.values() {
                    let sensor = sensor.read().unwrap();

                    if sensor.state() != SensorState::High {
                        continue;
                    }

                    if sensor.distance > 0
                        && sensor.triggered_for(Duration::from_secs(3), SensorState::High)
                    {
                        score += sensor.distance;
                    } else {
                        score += 1;
                    }
                }

                group.write().unwrap().set_score(score)?;
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}
