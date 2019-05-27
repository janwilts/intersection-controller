use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::intersections::component::{Component, ComponentId, ComponentState, ComponentUid};
use crate::intersections::group::ArcGroup;

pub type ArcActuator<S> = Arc<RwLock<Box<Actuator<S>>>>;

pub struct Actuator<S>
where
    S: ComponentState,
{
    group: ArcGroup,

    id: ComponentId,

    alias: Option<String>,

    state: S,
    initial_state: S,
    timestamp: DateTime<Utc>,

    sender: Sender<ComponentUid>,
    receiver: Receiver<ComponentUid>,
}

impl<S> Actuator<S>
where
    S: ComponentState + Send,
{
    pub fn new(group: ArcGroup, id: ComponentId, alias: Option<String>, initial_state: S) -> Self {
        let (sender, receiver) = unbounded();

        Self {
            group,
            id,
            alias,
            state: initial_state,
            initial_state,
            timestamp: Utc::now(),
            sender,
            receiver,
        }
    }
}

impl<S> Component<S> for Actuator<S>
where
    S: ComponentState + Send,
{
    fn receiver(&self) -> Receiver<ComponentUid> {
        self.receiver.clone()
    }

    fn sender(&self) -> Sender<ComponentUid> {
        self.sender.clone()
    }

    fn group(&self) -> ArcGroup {
        Arc::clone(&self.group)
    }

    fn state(&self) -> S {
        self.state
    }

    fn initial_state(&self) -> S {
        self.initial_state
    }

    fn set_state_internal(&mut self, state: S) {
        self.state = state;
        self.timestamp = Utc::now();
    }

    fn id(&self) -> ComponentId {
        self.id
    }
}
