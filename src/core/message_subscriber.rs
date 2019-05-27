use crossbeam_channel::Sender;
use rumqtt::Notification;

use crate::io::client::Client;

pub struct MessageSubscriber {
    subscriber: Client,
    sender: Sender<(String, String)>,
}

impl MessageSubscriber {
    pub fn new(subscriber: Client, sender: Sender<(String, String)>) -> Self {
        Self { subscriber, sender }
    }

    pub fn run(&self) {
        let receiver = self.subscriber.listen().unwrap();

        for message in receiver {
            let message = match message {
                Notification::Publish(msg) => msg,
                _ => continue,
            };

            let topic = message.topic_name;
            let payload = String::from_utf8_lossy(&message.payload);

            debug!(
                "MQTT Client \"{}\" received a message with topic \"{}\" and payload \"{}\".",
                self.subscriber.options.client_id(),
                topic,
                payload,
            );

            self.sender
                .send((topic.clone(), String::from(payload.clone())))
                .unwrap_or_else(|_| {
                    error!(
                        "Could not send message on topic \"{}\" with payload \"{}\".",
                        topic, payload,
                    )
                });
        }
    }
}
