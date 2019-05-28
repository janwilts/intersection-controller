use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;

use crate::core::message_publisher::Message;
use crate::intersections::component::{Component, ComponentKind, ComponentUid};
use crate::intersections::deck::DeckState;
use crate::intersections::gate::GateState;
use crate::intersections::group::{ArcGroup, GroupKind};
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;
use crate::intersections::sensor::SensorState;
use crate::io::topics::component_topic::ComponentTopic;

pub struct BridgeRunner {
    intersection: ArcIntersection,
    sender: Sender<Message>,
    stop: bool,
}

impl BridgeRunner {
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

            if !self.one_vessel_high() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let intersection = self.intersection.read().unwrap();

            let bridge_light_id = ComponentUid::new(GroupKind::Bridge, 1, ComponentKind::Light, 1);
            let front_gate_id = ComponentUid::new(GroupKind::Bridge, 1, ComponentKind::Gate, 1);
            let back_gate_id = ComponentUid::new(GroupKind::Bridge, 1, ComponentKind::Gate, 1);
            let deck_sensor_id = ComponentUid::new(GroupKind::Bridge, 1, ComponentKind::Sensor, 1);
            let deck_id = ComponentUid::new(GroupKind::Bridge, 1, ComponentKind::Deck, 1);
            let ww_sensor_id = ComponentUid::new(GroupKind::Vessel, 3, ComponentKind::Sensor, 1);

            let bridge_light = intersection.find_light(bridge_light_id).unwrap();
            let front_gate = intersection.find_gate(front_gate_id).unwrap();
            let back_gate = intersection.find_gate(back_gate_id).unwrap();
            let deck_sensor = intersection.find_sensor(deck_sensor_id).unwrap();
            let deck = intersection.find_deck(deck_id).unwrap();
            let waterway_sensor = intersection.find_sensor(ww_sensor_id).unwrap();

            bridge_light
                .write()
                .unwrap()
                .set_state(LightState::Prohibit);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(bridge_light.read().unwrap().uid())),
                Vec::from(String::from("0").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(6));

            front_gate.write().unwrap().set_state(GateState::Close);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(front_gate.read().unwrap().uid())),
                Vec::from(String::from("1").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(4));

            while deck_sensor.read().unwrap().state() == SensorState::High {
                deck_sensor
                    .read()
                    .unwrap()
                    .receiver()
                    .recv_timeout(Duration::from_secs(20));
            }

            back_gate.write().unwrap().set_state(GateState::Close);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(back_gate.read().unwrap().uid())),
                Vec::from(String::from("1").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(4));

            deck.write().unwrap().set_state(DeckState::Open);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(deck.read().unwrap().uid())),
                Vec::from(String::from("0").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(10));

            while self.one_vessel_high() {
                for vessel in self.main_vessels() {
                    if !vessel.read().unwrap().one_sensor_high() {
                        continue;
                    }

                    for light in vessel.read().unwrap().lights.values() {
                        light.write().unwrap().set_state(LightState::Proceed);
                        self.sender.send(Message::Message((
                            Box::new(ComponentTopic::from(light.read().unwrap().uid())),
                            Vec::from(String::from("2").as_bytes()),
                        )));
                    }

                    thread::sleep(Duration::from_secs(10));

                    for light in vessel.read().unwrap().lights.values() {
                        light.write().unwrap().set_state(LightState::Prohibit);
                        self.sender.send(Message::Message((
                            Box::new(ComponentTopic::from(light.read().unwrap().uid())),
                            Vec::from(String::from("0").as_bytes()),
                        )));
                    }
                }
            }

            deck.write().unwrap().set_state(DeckState::Close);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(deck.read().unwrap().uid())),
                Vec::from(String::from("1").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(10));

            front_gate.write().unwrap().set_state(GateState::Open);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(front_gate.read().unwrap().uid())),
                Vec::from(String::from("0").as_bytes()),
            )));

            back_gate.write().unwrap().set_state(GateState::Open);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(back_gate.read().unwrap().uid())),
                Vec::from(String::from("0").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(4));

            bridge_light.write().unwrap().set_state(LightState::Proceed);
            self.sender.send(Message::Message((
                Box::new(ComponentTopic::from(bridge_light.read().unwrap().uid())),
                Vec::from(String::from("2").as_bytes()),
            )));

            thread::sleep(Duration::from_secs(10));
        }
    }

    pub fn stop(&mut self) {
        self.stop = false;
    }

    fn one_vessel_high(&self) -> bool {
        for group in self.intersection.read().unwrap().groups.values() {
            if group.read().unwrap().id.kind != GroupKind::Vessel {
                continue;
            }

            if group.read().unwrap().one_sensor_high() {
                return true;
            }
        }

        false
    }

    fn main_vessels(&self) -> Vec<ArcGroup> {
        let mut groups: Vec<ArcGroup> = vec![];

        for group in self.intersection.read().unwrap().groups.values() {
            if group.read().unwrap().id.kind != GroupKind::Vessel {
                continue;
            }

            if group.read().unwrap().id.id == 1 || group.read().unwrap().id.id == 2 {
                groups.push(Arc::clone(&group));
            }
        }

        groups
    }
}
