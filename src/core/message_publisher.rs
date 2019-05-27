use crossbeam_channel::Receiver;

use crate::io::client::Client;
use crate::io::topics::Topic;

pub enum Message {
    Message((Box<dyn Topic>, Vec<u8>)),
    Stop,
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
        loop {
            if let Ok(message) = self.receiver.recv() {
                if let Message::Stop = message {
                    break;
                }

                if let Message::Message((topic, payload)) = message {
                    self.publisher.publish(topic, payload.clone());
                }
            }
        }
    }
}
