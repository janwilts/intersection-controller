use crossbeam_channel::Receiver;

use crate::io::client::Client;
use crate::io::topics::Topic;

pub struct Message {
    pub topic: Box<dyn Topic>,
    pub payload: Vec<u8>,
}

pub struct MessagePublisher {
    publisher: Client,
    receiver: Receiver<Message>,
}

impl MessagePublisher {
    pub fn new(publisher: Client, receiver: Receiver<Message>) -> Self {
        Self {
            publisher,
            receiver,
        }
    }

    pub fn run(mut self) {
        for message in &self.receiver {
            self.publisher.publish(message.topic, message.payload);
        }
    }
}
