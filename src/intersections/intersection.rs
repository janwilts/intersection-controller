use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::intersections::actuator::ArcActuator;
use crate::intersections::component::ComponentUid;
use crate::intersections::deck::DeckState;
use crate::intersections::gate::GateState;
use crate::intersections::group::{ArcGroup, GroupId};
use crate::intersections::light::LightState;
use crate::intersections::sensor::ArcSensor;

pub type ArcIntersection = Arc<RwLock<Box<Intersection>>>;

pub enum Notification {
    StateUpdated(ComponentUid),
    ScoreUpdated(GroupId),
}

pub struct Intersection {
    pub alias: Option<String>,
    pub groups: HashMap<GroupId, ArcGroup>,

    pub state_receiver: Receiver<ComponentUid>,
    pub score_receiver: Receiver<GroupId>,

    state_sender: Sender<ComponentUid>,
    score_sender: Sender<GroupId>,
    notification_sender: Sender<Notification>,
}

impl Intersection {
    pub fn new(alias: Option<String>, notification_sender: Sender<Notification>) -> Self {
        let (state_sender, state_receiver) = unbounded();
        let (score_sender, score_receiver) = unbounded();

        Self {
            alias,
            groups: HashMap::new(),

            state_receiver,
            score_receiver,

            state_sender,
            score_sender,
            notification_sender,
        }
    }

    pub fn groups(&self) -> Vec<ArcGroup> {
        self.groups.values().map(|g| Arc::clone(&g)).collect()
    }

    pub fn sensors(&self) -> Vec<ArcSensor> {
        let mut lights: Vec<ArcSensor> = vec![];

        for group in self.groups.values() {
            lights.extend(group.read().unwrap().sensors());
        }

        lights
    }

    pub fn lights(&self) -> Vec<ArcActuator<LightState>> {
        let mut lights: Vec<ArcActuator<LightState>> = vec![];

        for group in self.groups.values() {
            lights.extend(group.read().unwrap().lights());
        }

        lights
    }

    pub fn gates(&self) -> Vec<ArcActuator<GateState>> {
        let mut lights: Vec<ArcActuator<GateState>> = vec![];

        for group in self.groups.values() {
            lights.extend(group.read().unwrap().gates());
        }

        lights
    }

    pub fn decks(&self) -> Vec<ArcActuator<DeckState>> {
        let mut lights: Vec<ArcActuator<DeckState>> = vec![];

        for group in self.groups.values() {
            lights.extend(group.read().unwrap().decks());
        }

        lights
    }

    pub fn find_group(&self, id: GroupId) -> Option<ArcGroup> {
        if let Some(group) = self.groups.get(&id) {
            return Some(Arc::clone(&group));
        }

        None
    }

    pub fn find_sensor(&self, id: ComponentUid) -> Option<ArcSensor> {
        let group = self.find_group(id.group_id)?;
        let sensor = group.read().unwrap().find_sensor(id.component_id)?;
        Some(Arc::clone(&sensor))
    }

    pub fn find_light(&self, id: ComponentUid) -> Option<ArcActuator<LightState>> {
        let group = self.find_group(id.group_id)?;
        let light = group.read().unwrap().find_light(id.component_id)?;
        Some(Arc::clone(&light))
    }

    pub fn find_gate(&self, id: ComponentUid) -> Option<ArcActuator<GateState>> {
        let group = self.find_group(id.group_id)?;
        let gate = group.read().unwrap().find_gate(id.component_id)?;
        Some(Arc::clone(&gate))
    }

    pub fn find_deck(&self, id: ComponentUid) -> Option<ArcActuator<DeckState>> {
        let group = self.find_group(id.group_id)?;
        let deck = group.read().unwrap().find_deck(id.component_id)?;
        Some(Arc::clone(&deck))
    }

    fn highest_scoring_group(groups: &Vec<ArcGroup>) -> ArcGroup {
        let mut score = -1;
        let mut highest = Arc::clone(groups.first().unwrap());

        for group in groups {
            if group.read().unwrap().score > score {
                score = group.read().unwrap().score;
                highest = Arc::clone(group);
            }
        }

        highest
    }

    pub fn get_runnables(&self) -> Result<Vec<ArcGroup>, failure::Error> {
        let mut groups: Vec<ArcGroup> = vec![];

        let highest_scoring = Self::highest_scoring_group(&self.groups());

        if highest_scoring.read().unwrap().score == 0 {
            return Ok(groups);
        }

        for group in &highest_scoring.read().unwrap().concurrences {
            if group.read().unwrap().score <= 0 {
                continue;
            }

            let mut can_fit = true;

            for block in &group.read().unwrap().blocks {
                for existing_group in &groups {
                    if existing_group.read().unwrap().id == block.read().unwrap().id {
                        can_fit = false;
                    }
                }
            }

            if !can_fit {
                continue;
            }

            groups.push(Arc::clone(&group));
        }

        Ok(groups)
    }

    pub fn send_state(&self, id: ComponentUid) {
        self.state_sender.send(id);
        self.notification_sender
            .send(Notification::StateUpdated(id));
    }

    pub fn send_score(&self, id: GroupId) {
        self.score_sender.send(id);
        self.notification_sender
            .send(Notification::ScoreUpdated(id));
    }
}
