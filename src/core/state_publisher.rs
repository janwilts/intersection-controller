use crossbeam_channel::{Receiver, Sender};
use failure::Fail;

use crate::core::message_publisher::Message;
use crate::intersections::component::{Component, ComponentKind, ComponentUid};
use crate::intersections::intersection::{ArcIntersection, Notification};
use crate::io::topics::component_topic::ComponentTopic;

#[derive(Debug, Fail)]
#[fail(display = "Component with id {} was not found", id)]
pub struct ComponentNotFound {
    id: ComponentUid,
}

pub struct StatePublisher {
    notification_receiver: Receiver<Notification>,
    sender: Sender<Message>,

    traffic_light: ArcIntersection,
    bridge: ArcIntersection,
}

impl StatePublisher {
    pub fn new(
        notification_receiver: Receiver<Notification>,
        sender: Sender<Message>,
        traffic_light: ArcIntersection,
        bridge: ArcIntersection,
    ) -> Self {
        Self {
            notification_receiver,
            sender,
            traffic_light,
            bridge,
        }
    }

    pub fn run(&self) -> Result<(), failure::Error> {
        for notification in &self.notification_receiver {
            if let Notification::StateUpdated(id) = notification {
                if id.component_id.kind == ComponentKind::Sensor {
                    continue;
                }

                self.sender.send(Message {
                    topic: Box::new(ComponentTopic::from(id)),
                    payload: self.get_payload(id)?.to_string().into_bytes(),
                });
            }
        }

        Ok(())
    }

    fn get_payload(&self, id: ComponentUid) -> Result<i32, failure::Error> {
        if let Some(sensor) = self.traffic_light.read().unwrap().find_sensor(id) {
            return Ok(sensor.read().unwrap().state() as i32);
        }

        if let Some(sensor) = self.bridge.read().unwrap().find_sensor(id) {
            return Ok(sensor.read().unwrap().state() as i32);
        }

        if let Some(light) = self.traffic_light.read().unwrap().find_light(id) {
            return Ok(light.read().unwrap().state() as i32);
        }

        if let Some(light) = self.bridge.read().unwrap().find_light(id) {
            return Ok(light.read().unwrap().state() as i32);
        }

        if let Some(gate) = self.traffic_light.read().unwrap().find_gate(id) {
            return Ok(gate.read().unwrap().state() as i32);
        }

        if let Some(gate) = self.bridge.read().unwrap().find_gate(id) {
            return Ok(gate.read().unwrap().state() as i32);
        }

        if let Some(deck) = self.traffic_light.read().unwrap().find_deck(id) {
            return Ok(deck.read().unwrap().state() as i32);
        }

        if let Some(deck) = self.bridge.read().unwrap().find_deck(id) {
            return Ok(deck.read().unwrap().state() as i32);
        }

        Err(ComponentNotFound { id }.into())
    }
}
