use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;

use crate::core::message_publisher::Message;
use crate::intersections::component::Component;
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;
use crate::io::topics::component_topic::ComponentTopic;

pub struct TrafficLightsRunner {
    intersection: ArcIntersection,
    sender: Sender<Message>,
    stop: bool,
}

impl TrafficLightsRunner {
    pub fn new(intersection: ArcIntersection, sender: Sender<Message>) -> Self {
        Self {
            intersection,
            sender,
            stop: false,
        }
    }

    pub fn run(&self) {
        loop {
            if self.stop {
                return;
            }

            let runnables = self.intersection.read().unwrap().get_runnables().unwrap();

            if runnables.is_empty() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            for group in runnables.clone() {
                for light in group.read().unwrap().lights.values() {
                    light.write().unwrap().set_state(LightState::Proceed);

                    self.sender.send(Message::Message((
                        Box::new(ComponentTopic {
                            team_id: Some(4),
                            uid: light.read().unwrap().uid(),
                        }),
                        Vec::from(String::from("2").as_bytes()),
                    )));
                }
            }

            thread::sleep(Duration::from_secs(10));

            for group in runnables.clone() {
                for light in group.read().unwrap().lights.values() {
                    light.write().unwrap().set_state(LightState::Transitioning);

                    self.sender.send(Message::Message((
                        Box::new(ComponentTopic {
                            team_id: Some(4),
                            uid: light.read().unwrap().uid(),
                        }),
                        Vec::from(String::from("1").as_bytes()),
                    )));
                }
            }

            thread::sleep(Duration::from_secs(4));

            for group in runnables.clone() {
                for light in group.read().unwrap().lights.values() {
                    light.write().unwrap().set_state(LightState::Prohibit);

                    self.sender.send(Message::Message((
                        Box::new(ComponentTopic {
                            team_id: Some(4),
                            uid: light.read().unwrap().uid(),
                        }),
                        Vec::from(String::from("0").as_bytes()),
                    )));
                }

                group.write().unwrap().reset_score();
            }

            thread::sleep(Duration::from_secs(2));
        }
    }

    pub fn stop(&mut self) {
        self.stop = false;
    }
}
