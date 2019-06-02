use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Acquire;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{after, Receiver};

use crate::intersections::component::{Component, ComponentKind, ComponentUid as Uid};
use crate::intersections::deck::DeckState;
use crate::intersections::gate::GateState;
use crate::intersections::group::{ArcGroup, GroupId, GroupKind};
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;
use crate::intersections::sensor::SensorState;

pub struct BridgeRunner {
    intersection: ArcIntersection,
    stop: Arc<AtomicBool>,
    stop_channel: Receiver<()>,
}

impl BridgeRunner {
    pub fn new(
        intersection: ArcIntersection,
        stop: Arc<AtomicBool>,
        stop_channel: Receiver<()>,
    ) -> Self {
        Self {
            intersection,
            stop,
            stop_channel,
        }
    }

    pub fn run(&self) {
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

        let top_vessel = self
            .intersection
            .read()
            .unwrap()
            .find_group(GroupId {
                kind: GroupKind::Vessel,
                id: 1,
            })
            .unwrap();

        let bottom_vessel = self
            .intersection
            .read()
            .unwrap()
            .find_group(GroupId {
                kind: GroupKind::Vessel,
                id: 2,
            })
            .unwrap();

        let front_vessel_channel = top_vessel.read().unwrap().sensor_receiver.clone();
        let bottom_vessel_channel = bottom_vessel.read().unwrap().sensor_receiver.clone();

        loop {
            if !self.one_vessel_high() {
                select! {
                    recv(front_vessel_channel) -> _ => {},
                    recv(bottom_vessel_channel) -> _ => {},
                    recv(self.stop_channel) -> _ => {},
                };

                if self.stop.load(Acquire) {
                    break;
                }

                continue;
            }

            light.write().unwrap().set_state(LightState::Transitioning);

            select! {
                recv(after(Duration::from_secs(4))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }

            light.write().unwrap().set_state(LightState::Prohibit);

            select! {
                recv(after(Duration::from_secs(6))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }

            // Wait for all vehicles to leave the deck.
            while above_deck_sensor.read().unwrap().state() == SensorState::High {
                let channel = above_deck_sensor.read().unwrap().receiver.clone();

                select! {
                    recv(channel) -> _ => {},
                    recv(self.stop_channel) -> _ => {},
                };
                if self.stop.load(Acquire) {
                    break;
                }
            }

            front_gate.write().unwrap().set_state(GateState::Close);
            back_gate.write().unwrap().set_state(GateState::Close);

            select! {
                recv(after(Duration::from_secs(4))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }

            deck.write().unwrap().set_state(DeckState::Open);

            select! {
                recv(after(Duration::from_secs(10))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }

            while self.one_vessel_high() {
                for vessel in self.main_vessels() {
                    if !vessel.read().unwrap().one_sensor_high() {
                        continue;
                    }

                    for light in vessel.read().unwrap().lights.values() {
                        light.write().unwrap().set_state(LightState::Proceed);
                    }

                    let channel = below_deck_sensor.read().unwrap().receiver.clone();

                    while below_deck_sensor.read().unwrap().state() == SensorState::Low {
                        select! {
                            recv(channel) -> _ => {},
                            recv(self.stop_channel) -> _ => {},
                        };
                        if self.stop.load(Acquire) {
                            return;
                        }
                    }

                    while below_deck_sensor.read().unwrap().state() == SensorState::High {
                        select! {
                            recv(channel) -> _ => {},
                            recv(self.stop_channel) -> _ => {},
                        };
                        if self.stop.load(Acquire) {
                            return;
                        }
                    }

                    for light in vessel.read().unwrap().lights.values() {
                        light.write().unwrap().set_state(LightState::Prohibit);
                    }
                }
            }

            deck.write().unwrap().set_state(DeckState::Close);

            select! {
                recv(after(Duration::from_secs(10))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }

            front_gate.write().unwrap().set_state(GateState::Open);
            back_gate.write().unwrap().set_state(GateState::Open);

            select! {
                recv(after(Duration::from_secs(4))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }

            light.write().unwrap().set_state(LightState::Proceed);

            select! {
                recv(after(Duration::from_secs(30))) -> _ => {},
                recv(self.stop_channel) -> _ => {},
            };
            if self.stop.load(Acquire) {
                break;
            }
        }

        warn!("Stopping bridge runner");
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
