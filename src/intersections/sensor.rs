use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use colored::Colorize;
use crossbeam_channel::{unbounded, Receiver, Sender};
use failure::Fail;

use crate::intersections::component::{Component, ComponentId, ComponentState, ComponentUid};
use crate::intersections::group::ArcGroup;

pub type ArcSensor = Arc<RwLock<Box<Sensor>>>;

#[derive(Debug, Fail)]
pub enum SensorStateError {
    #[fail(display = "Could not create Sensor State from a \"{}\" value.", value)]
    CouldNotConvert { value: i32 },
}

#[derive(PartialEq, Copy, Clone)]
pub enum SensorState {
    Low,
    High,
}

impl ComponentState for SensorState {}

impl Display for SensorState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SensorState::Low => write!(f, "{}", "LOW".black()),
            SensorState::High => write!(f, "{}", "HIGH".white()),
        }
    }
}

impl TryFrom<i32> for SensorState {
    type Error = failure::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        match value {
            0 => Ok(SensorState::Low),
            1 => Ok(SensorState::High),
            _ => Err(SensorStateError::CouldNotConvert { value }.into()),
        }
    }
}

impl Into<i32> for SensorState {
    fn into(self) -> i32 {
        match self {
            SensorState::Low => 0,
            SensorState::High => 1,
        }
    }
}

pub struct Sensor {
    group: ArcGroup,

    id: ComponentId,

    alias: Option<String>,

    state: SensorState,
    initial_state: SensorState,
    timestamp: DateTime<Utc>,

    sender: Sender<ComponentUid>,
    receiver: Receiver<ComponentUid>,
}

impl Sensor {
    pub fn new(
        group: ArcGroup,
        id: ComponentId,
        alias: Option<String>,
        initial_state: SensorState,
    ) -> Self {
        let (sender, receiver) = unbounded();

        Self {
            group,
            id,
            alias,
            state: initial_state.clone(),
            initial_state,
            timestamp: Utc::now(),
            sender,
            receiver,
        }
    }
}

impl Component<SensorState> for Sensor {
    fn receiver(&self) -> Receiver<ComponentUid> {
        self.receiver.clone()
    }

    fn sender(&self) -> Sender<ComponentUid> {
        self.sender.clone()
    }

    fn group(&self) -> ArcGroup {
        Arc::clone(&self.group)
    }

    fn state(&self) -> SensorState {
        self.state.clone()
    }

    fn initial_state(&self) -> SensorState {
        self.initial_state
    }

    fn set_state_internal(&mut self, state: SensorState) {
        self.state = state;
        self.timestamp = Utc::now();
    }

    fn id(&self) -> ComponentId {
        self.id
    }
}
