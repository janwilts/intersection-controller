use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

use chrono::{DateTime, Utc};
use colored::{Color, Colorize};
use crossbeam_channel::{Receiver, Sender};
use failure;

use crate::intersections::group::{ArcGroup, GroupId, GroupKind};

#[derive(Debug, Fail)]
#[fail(display = "Could not build component kind; unknown kind: {}", kind)]
pub struct ComponentKindBuildError {
    kind: String,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub enum ComponentKind {
    Light,
    Sensor,
    Gate,
    Deck,
}

impl Display for ComponentKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ComponentKind::Light => write!(f, "light"),
            ComponentKind::Sensor => write!(f, "sensor"),
            ComponentKind::Gate => write!(f, "gate"),
            ComponentKind::Deck => write!(f, "deck"),
        }
    }
}

impl Debug for ComponentKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ComponentKind::Light => write!(f, "{}", "L".color(Color::Yellow)),
            ComponentKind::Sensor => write!(f, "{}", "S".color(Color::Green)),
            ComponentKind::Gate => write!(f, "{}", "G".color(Color::Magenta)),
            ComponentKind::Deck => write!(f, "{}", "D".color(Color::Blue)),
        }
    }
}

impl TryFrom<&str> for ComponentKind {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "light" => Ok(ComponentKind::Light),
            "sensor" => Ok(ComponentKind::Sensor),
            "gate" => Ok(ComponentKind::Gate),
            "deck" => Ok(ComponentKind::Deck),
            _ => Err(ComponentKindBuildError {
                kind: String::from(value),
            }
            .into()),
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct ComponentId {
    pub kind: ComponentKind,
    pub id: i32,
}

impl Display for ComponentId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.kind, self.id)
    }
}

impl Debug for ComponentId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}{}", self.kind, self.id)
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct ComponentUid {
    pub group_id: GroupId,
    pub component_id: ComponentId,
}

impl ComponentUid {
    pub fn new(
        group_kind: GroupKind,
        group_id: i32,
        component_kind: ComponentKind,
        component_id: i32,
    ) -> Self {
        Self {
            group_id: GroupId {
                kind: group_kind,
                id: group_id,
            },
            component_id: ComponentId {
                kind: component_kind,
                id: component_id,
            },
        }
    }
}

impl Display for ComponentUid {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.group_id, self.component_id)
    }
}

impl Debug for ComponentUid {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}{:?}", self.group_id, self.component_id)
    }
}

pub trait ComponentState:
    Clone + Copy + Default + Display + PartialEq + Into<i32> + TryFrom<i32>
{
}

pub trait Component<S>: Send
where
    S: ComponentState,
{
    fn receiver(&self) -> Receiver<ComponentUid>;
    fn sender(&self) -> Sender<ComponentUid>;

    fn group(&self) -> ArcGroup;

    fn state(&self) -> S;
    fn initial_state(&self) -> S;
    fn set_state_internal(&mut self, state: S);
    fn timestamp(&self) -> DateTime<Utc>;

    fn id(&self) -> ComponentId;

    fn set_state(&mut self, state: S) -> Result<(), failure::Error> {
        debug!("Setting state on {:?} to {}", self.uid(), state);

        self.set_state_internal(state);
        self.sender().send(self.uid())?;
        self.group().read().unwrap().send_actuator(self.uid())?;

        Ok(())
    }

    fn uid(&self) -> ComponentUid {
        let group_id = self.group().read().unwrap().id;

        ComponentUid {
            group_id,
            component_id: self.id(),
        }
    }

    fn reset(&mut self) -> Result<(), failure::Error> {
        self.set_state(self.initial_state())?;

        Ok(())
    }

    fn triggered_for(&self, duration: Duration, state: S) -> bool {
        self.state() == state && (Utc::now() - self.timestamp()).to_std().unwrap() >= duration
    }
}
