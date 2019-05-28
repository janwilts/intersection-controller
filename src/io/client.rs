use crossbeam_channel::Receiver;
use failure::Fail;
use rumqtt::{LastWill, MqttClient, MqttOptions, Notification, QoS};

use crate::io::topics::Topic;

#[derive(Debug, Fail)]
pub enum ClientError {
    /// The error returned when a method is called on the Client whilst the run method has not yet
    /// been called.
    #[fail(display = "Client \"{}\" has not been started yet", client_id)]
    NotYetStarted { client_id: String },
}

pub struct Client {
    pub options: MqttOptions,
    client: Option<MqttClient>,
    receiver: Option<Receiver<Notification>>,

    /// Client preferred QoS, will be used in `subscribe()` and `publish()`.
    qos: QoS,

    team_id: i32,
}

impl Client {
    pub fn new(client_id: String, host: String, port: u16, qos: QoS, team_id: i32) -> Self {
        info!(target: "mqtt", "Creating new MQTT client with client ID \"{}\"", client_id);

        Self {
            options: MqttOptions::new(client_id, host, port),
            client: None,
            receiver: None,
            qos,
            team_id,
        }
    }

    pub fn set_last_will(&mut self, mut topic: Box<dyn Topic>, payload: Vec<u8>) {
        topic.set_team_id(self.team_id);

        self.options = self.options.clone().set_last_will(LastWill {
            topic: format!("{}", topic),
            message: String::from_utf8(payload).unwrap(),
            qos: self.qos,
            retain: false,
        });
    }

    pub fn start(&mut self) -> Result<(), failure::Error> {
        info!(
            target: "mqtt",
            "MQTT client \"{}\" connecting to \"{}:{}\"",
            self.options.client_id(),
            self.options.broker_address().0,
            self.options.broker_address().1,
        );

        let (client, receiver) = MqttClient::start(self.options.clone())?;

        info!(
            target: "mqtt",
            "MQTT client \"{}\" successfully connected to \"{}:{}\"",
            self.options.client_id(),
            self.options.broker_address().0,
            self.options.broker_address().1
        );

        self.client = Some(client);
        self.receiver = Some(receiver);

        Ok(())
    }

    pub fn subscribe(&mut self, mut topic: Box<dyn Topic>) -> Result<(), failure::Error> {
        topic.set_team_id(self.team_id);

        if let Some(client) = &mut self.client {
            client.subscribe(format!("{}", topic), self.qos)?;

            info!(
                target: "mqtt",
                "MQTT client \"{}\" subscribed to topic \"{}\" on QoS {}",
                self.options.client_id(),
                topic,
                self.qos.to_u8()
            );

            return Ok(());
        }

        Err(ClientError::NotYetStarted {
            client_id: self.options.client_id(),
        }
            .into())
    }

    pub fn unsubscribe(&mut self, mut topic: Box<dyn Topic>) -> Result<(), failure::Error> {
        topic.set_team_id(self.team_id);

        if let Some(client) = &mut self.client {
            client.unsubscribe(format!("{}", topic))?;

            info!(
                "MQTT client \"{}\" unsubscribed from topic \"{}\"",
                self.options.client_id(),
                topic,
            );

            return Ok(());
        }

        Err(ClientError::NotYetStarted {
            client_id: self.options.client_id(),
        }
            .into())
    }

    pub fn publish<P>(
        &mut self,
        mut topic: Box<dyn Topic>,
        payload: P,
    ) -> Result<(), failure::Error>
        where
            P: Into<Vec<u8>>,
    {
        topic.set_team_id(self.team_id);

        if let Some(client) = &mut self.client {
            client.publish(format!("{}", topic), self.qos, false, payload)?;

            info!(
                "MQTT client \"{}\" published with topic \"{}\"",
                self.options.client_id(),
                topic,
            );

            return Ok(());
        }

        Err(ClientError::NotYetStarted {
            client_id: self.options.client_id(),
        }
            .into())
    }

    pub fn listen(&self) -> Result<Receiver<Notification>, failure::Error> {
        if let Some(receiver) = &self.receiver {
            return Ok(receiver.clone());
        }

        Err(ClientError::NotYetStarted {
            client_id: self.options.client_id(),
        }
            .into())
    }
}
