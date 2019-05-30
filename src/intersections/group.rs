use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::sync::{Arc, RwLock};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::intersections::actuator::ArcActuator;
use crate::intersections::component::{Component, ComponentId, ComponentKind, ComponentUid};
use crate::intersections::deck::DeckState;
use crate::intersections::gate::GateState;
use crate::intersections::intersection::ArcIntersection;
use crate::intersections::light::LightState;
use crate::intersections::sensor::{ArcSensor, SensorState};

pub type ArcGroup = Arc<RwLock<Box<Group>>>;

#[derive(Debug, Fail)]
#[fail(display = "Invalid group kind: {}", group_kind)]
pub struct InvalidGroupKind {
    group_kind: String,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum GroupKind {
    MotorVehicle,
    Cycle,
    Foot,
    Vessel,
    Bridge,
}

impl fmt::Display for GroupKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GroupKind::MotorVehicle => write!(f, "motor_vehicle"),
            GroupKind::Cycle => write!(f, "cycle"),
            GroupKind::Foot => write!(f, "foot"),
            GroupKind::Vessel => write!(f, "vessel"),
            GroupKind::Bridge => write!(f, "bridge"),
        }
    }
}

impl fmt::Debug for GroupKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GroupKind::MotorVehicle => write!(f, "MV"),
            GroupKind::Cycle => write!(f, "C"),
            GroupKind::Foot => write!(f, "F"),
            GroupKind::Vessel => write!(f, "V"),
            GroupKind::Bridge => write!(f, "B"),
        }
    }
}

impl TryFrom<&str> for GroupKind {
    type Error = InvalidGroupKind;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "motor_vehicle" => Ok(GroupKind::MotorVehicle),
            "cycle" => Ok(GroupKind::Cycle),
            "foot" => Ok(GroupKind::Foot),
            "vessel" => Ok(GroupKind::Vessel),
            "bridge" => Ok(GroupKind::Bridge),
            _ => Err(InvalidGroupKind {
                group_kind: String::from(value),
            }),
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct GroupId {
    pub kind: GroupKind,
    pub id: i32,
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.kind, self.id)
    }
}

impl fmt::Debug for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}{}", self.kind, self.id)
    }
}

pub struct Group {
    pub intersection: ArcIntersection,

    pub id: GroupId,

    pub alias: Option<String>,
    pub special: bool,

    pub sensors: HashMap<ComponentId, ArcSensor>,
    pub lights: HashMap<ComponentId, ArcActuator<LightState>>,
    pub gates: HashMap<ComponentId, ArcActuator<GateState>>,
    pub decks: HashMap<ComponentId, ArcActuator<DeckState>>,

    pub score: i32,

    pub blocks: Vec<ArcGroup>,
    pub concurrences: Vec<ArcGroup>,

    pub sensor_receiver: Receiver<ComponentUid>,
    pub light_receiver: Receiver<ComponentUid>,
    pub gate_receiver: Receiver<ComponentUid>,
    pub deck_receiver: Receiver<ComponentUid>,
    pub actuator_receiver: Receiver<ComponentUid>,

    sensor_sender: Sender<ComponentUid>,
    light_sender: Sender<ComponentUid>,
    gate_sender: Sender<ComponentUid>,
    deck_sender: Sender<ComponentUid>,
    actuator_sender: Sender<ComponentUid>,
}

impl Group {
    pub fn new(
        intersection: ArcIntersection,
        id: GroupId,
        alias: Option<String>,
        special: bool,
    ) -> Self {
        let (sensor_sender, sensor_receiver) = unbounded();
        let (light_sender, light_receiver) = unbounded();
        let (gate_sender, gate_receiver) = unbounded();
        let (deck_sender, deck_receiver) = unbounded();

        let (actuator_sender, actuator_receiver) = unbounded();

        Self {
            intersection,
            id,
            alias,
            special,

            sensors: HashMap::new(),
            lights: HashMap::new(),
            gates: HashMap::new(),
            decks: HashMap::new(),

            score: 0,
            blocks: vec![],
            concurrences: vec![],

            sensor_receiver,
            light_receiver,
            actuator_receiver,
            gate_receiver,
            deck_receiver,

            sensor_sender,
            light_sender,
            gate_sender,
            deck_sender,
            actuator_sender,
        }
    }

    pub fn push_block(&mut self, block: ArcGroup) {
        self.blocks.push(block);
    }

    pub fn push_concurrent(&mut self, concurrent: ArcGroup) {
        self.concurrences.push(concurrent);
    }

    pub fn sensors(&self) -> Vec<ArcSensor> {
        self.sensors.values().map(|s| Arc::clone(&s)).collect()
    }

    pub fn find_sensor(&self, id: ComponentId) -> Option<ArcSensor> {
        Some(Arc::clone(self.sensors.get(&id)?))
    }

    pub fn find_light(&self, id: ComponentId) -> Option<ArcActuator<LightState>> {
        Some(Arc::clone(self.lights.get(&id)?))
    }

    pub fn find_gate(&self, id: ComponentId) -> Option<ArcActuator<GateState>> {
        Some(Arc::clone(self.gates.get(&id)?))
    }

    pub fn find_deck(&self, id: ComponentId) -> Option<ArcActuator<DeckState>> {
        Some(Arc::clone(self.decks.get(&id)?))
    }

    pub fn alias(&self) -> Option<String> {
        self.alias.clone()
    }

    pub fn set_score(&mut self, score: i32) {
        self.score = score;
        self.intersection.read().unwrap().send_score(self.id);
    }

    pub fn increase_score(&mut self, score: i32) {
        self.set_score(self.score + score);
    }

    pub fn reset_score(&mut self) {
        self.set_score(0);
    }

    pub fn one_sensor_high(&self) -> bool {
        for sensor in &self.sensors() {
            if sensor.read().unwrap().state() == SensorState::High {
                return true;
            }
        }

        false
    }

    pub fn blocks_group(&self, group: ArcGroup) -> bool {
        for g in &self.blocks {
            if g.read().unwrap().id == group.read().unwrap().id {
                return true;
            }
        }

        false
    }

    pub fn reset_all(&self) {
        for s in self.sensors.values() {
            s.write().unwrap().reset();
        }

        for l in self.lights.values() {
            l.write().unwrap().reset();
        }

        for d in self.decks.values() {
            d.write().unwrap().reset();
        }

        for g in self.gates.values() {
            g.write().unwrap().reset();
        }
    }

    pub fn send(&self, uid: ComponentUid) {
        self.intersection.read().unwrap().send_state(uid);

        match uid.component_id.kind {
            ComponentKind::Sensor => self.sensor_sender.send(uid),
            ComponentKind::Light => self.light_sender.send(uid),
            ComponentKind::Gate => self.gate_sender.send(uid),
            ComponentKind::Deck => self.deck_sender.send(uid),
        };
    }

    pub fn send_actuator(&self, uid: ComponentUid) {
        self.send(uid);
        self.actuator_sender.send(uid);
    }
}
