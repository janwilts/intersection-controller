use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::intersections::component::{Component, ComponentKind, ComponentUid as Uid};
use crate::intersections::deck::DeckState;
use crate::intersections::gate::GateState;
use crate::intersections::group::{ArcGroup, GroupKind};
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;
use crate::intersections::sensor::SensorState;

pub struct BridgeRunner {
    intersection: ArcIntersection,
}

impl BridgeRunner {
    pub fn new(intersection: ArcIntersection) -> Self {
        Self { intersection }
    }

    pub fn run(&self) -> Result<(), failure::Error> {
        let above_deck_sensor = self
            .intersection
            .read()
            .unwrap()
            .find_sensor(Uid::new(GroupKind::Bridge, 1, ComponentKind::Sensor, 1))
            .unwrap();
        let below_deck_sensor = self
            .intersection
            .read()
            .unwrap()
            .find_sensor(Uid::new(GroupKind::Vessel, 3, ComponentKind::Sensor, 1))
            .unwrap();
        let light = self
            .intersection
            .read()
            .unwrap()
            .find_light(Uid::new(GroupKind::Bridge, 1, ComponentKind::Light, 1))
            .unwrap();
        let front_gate = self
            .intersection
            .read()
            .unwrap()
            .find_gate(Uid::new(GroupKind::Bridge, 1, ComponentKind::Gate, 1))
            .unwrap();
        let back_gate = self
            .intersection
            .read()
            .unwrap()
            .find_gate(Uid::new(GroupKind::Bridge, 1, ComponentKind::Gate, 2))
            .unwrap();
        let deck = self
            .intersection
            .read()
            .unwrap()
            .find_deck(Uid::new(GroupKind::Bridge, 1, ComponentKind::Deck, 1))
            .unwrap();

        loop {
            if !self.one_vessel_high() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            light.write().unwrap().set_state(LightState::Transitioning);

            thread::sleep(Duration::from_secs(4));

            light.write().unwrap().set_state(LightState::Prohibit);

            thread::sleep(Duration::from_secs(6));

            // Wait for all vehicles to leave the deck.
            while above_deck_sensor.read().unwrap().state() == SensorState::High {
                thread::sleep(Duration::from_millis(100));
            }

            front_gate.write().unwrap().set_state(GateState::Close);
            back_gate.write().unwrap().set_state(GateState::Close);

            thread::sleep(Duration::from_secs(4));

            deck.write().unwrap().set_state(DeckState::Open);

            thread::sleep(Duration::from_secs(10));

            while self.one_vessel_high() {
                for vessel in self.main_vessels() {
                    if !vessel.read().unwrap().one_sensor_high() {
                        continue;
                    }

                    for light in vessel.read().unwrap().lights.values() {
                        light.write().unwrap().set_state(LightState::Proceed);
                    }

                    while below_deck_sensor.read().unwrap().state() == SensorState::Low {
                        thread::sleep(Duration::from_millis(100));
                    }

                    while below_deck_sensor.read().unwrap().state() == SensorState::High {
                        thread::sleep(Duration::from_millis(100));
                    }

                    for light in vessel.read().unwrap().lights.values() {
                        light.write().unwrap().set_state(LightState::Prohibit);
                    }
                }
            }

            deck.write().unwrap().set_state(DeckState::Close);

            thread::sleep(Duration::from_secs(10));

            front_gate.write().unwrap().set_state(GateState::Open);
            back_gate.write().unwrap().set_state(GateState::Open);

            thread::sleep(Duration::from_secs(4));

            light.write().unwrap().set_state(LightState::Proceed);

            thread::sleep(Duration::from_secs(30));
        }
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
