use std::convert::TryFrom;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Release;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::config::Config;
use crate::core::bridge_runner::BridgeRunner;
use crate::core::message_publisher::{Message, MessagePublisher};
use crate::core::message_subscriber::MessageSubscriber;
use crate::core::score_poller::ScorePoller;
use crate::core::state_publisher::StatePublisher;
use crate::core::traffic_lights_runner::TrafficLightsRunner;
use crate::intersections::component::Component;
use crate::intersections::intersection::{ArcIntersection, Notification};
use crate::intersections::sensor::SensorState;
use crate::io::client::Client;
use crate::io::topics::component_topic::ComponentTopic;
use crate::io::topics::lifecycle_topic::{Device, Handler, LifeCycleTopic};

pub struct Controller {
    traffic_lights: ArcIntersection,
    bridge: ArcIntersection,

    publisher_receiver: Receiver<Message>,

    subscriber_sender: Sender<(String, String)>,
    subscriber_receiver: Receiver<(String, String)>,

    message_publisher_handle: Option<JoinHandle<()>>,
    message_subscriber_handle: Option<JoinHandle<()>>,

    state_publisher_handle: Option<JoinHandle<()>>,
    state_publisher: Arc<StatePublisher>,

    score_poller_handle: Option<JoinHandle<()>>,
    score_poller: Arc<ScorePoller>,

    traffic_lights_runner_handle: Option<JoinHandle<()>>,
    traffic_lights_runner: Arc<TrafficLightsRunner>,

    bridge_runner_handle: Option<JoinHandle<()>>,
    bridge_runner: Arc<BridgeRunner>,

    stop_runners: Arc<AtomicBool>,
    stop_runners_sender: Sender<()>,
}

impl Controller {
    pub fn new(
        traffic_lights: ArcIntersection,
        bridge: ArcIntersection,
        notification_receiver: Receiver<Notification>,
        config: Config,
    ) -> Self {
        let (publisher_sender, publisher_receiver) = unbounded();
        let (subscriber_sender, subscriber_receiver) = unbounded();
        let (stop_runners_sender, stop_runners_receiver) = unbounded();

        let stop_runners = Arc::new(AtomicBool::new(false));

        Self {
            traffic_lights: Arc::clone(&traffic_lights),
            bridge: Arc::clone(&bridge),

            publisher_receiver,

            subscriber_sender,
            subscriber_receiver,

            message_publisher_handle: None,
            message_subscriber_handle: None,

            state_publisher_handle: None,
            state_publisher: Arc::new(StatePublisher::new(
                notification_receiver.clone(),
                publisher_sender.clone(),
                Arc::clone(&traffic_lights),
                Arc::clone(&bridge),
            )),

            score_poller_handle: None,
            score_poller: Arc::new(ScorePoller::new(Arc::clone(&traffic_lights))),

            traffic_lights_runner_handle: None,
            traffic_lights_runner: Arc::new(TrafficLightsRunner::new(
                Arc::clone(&traffic_lights),
                config.groups,
                Arc::clone(&stop_runners),
                stop_runners_receiver.clone(),
            )),

            bridge_runner_handle: None,
            bridge_runner: Arc::new(BridgeRunner::new(
                Arc::clone(&bridge),
                Arc::clone(&stop_runners),
                stop_runners_receiver.clone(),
            )),

            stop_runners: Arc::clone(&stop_runners),
            stop_runners_sender,
        }
    }

    pub fn start(
        &mut self,
        mut publisher: Client,
        mut subscriber: Client,
    ) -> Result<(), failure::Error> {
        debug!("Starting controller");

        debug!("Starting publisher");
        publisher.start()?;
        debug!("Starting subscriber");
        subscriber.start()?;

        debug!("Subscribing to sensor topics");
        for sensor in self.traffic_lights.read().unwrap().sensors() {
            subscriber.subscribe(Box::new(ComponentTopic::from(sensor.read().unwrap().uid())))?;
        }

        for sensor in self.bridge.read().unwrap().sensors() {
            subscriber.subscribe(Box::new(ComponentTopic::from(sensor.read().unwrap().uid())))?;
        }

        debug!("Subscribing to lifecycle topics");
        subscriber.subscribe(Box::new(LifeCycleTopic::new(
            Device::Simulator,
            Handler::Connect,
        )))?;

        subscriber.subscribe(Box::new(LifeCycleTopic::new(
            Device::Simulator,
            Handler::Disconnect,
        )))?;

        // Publisher
        let publisher_receiver = self.publisher_receiver.clone();
        self.message_publisher_handle = Some(thread::spawn(move || {
            let publisher = MessagePublisher::new(publisher, publisher_receiver);
            publisher.run().unwrap_or_else(|e| error!("{}", e));;
        }));

        // Subscriber
        let subscriber_sender = self.subscriber_sender.clone();
        self.message_subscriber_handle = Some(thread::spawn(move || {
            let subscriber = MessageSubscriber::new(subscriber, subscriber_sender);
            subscriber.run();
        }));

        let state_publisher = Arc::clone(&self.state_publisher);
        self.state_publisher_handle = Some(thread::spawn(move || {
            state_publisher.run().unwrap_or_else(|e| error!("{}", e));
        }));

        // Score Poller
        let score_poller = Arc::clone(&self.score_poller);
        self.score_poller_handle = Some(thread::spawn(move || {
            score_poller.run().unwrap_or_else(|e| error!("{}", e));;
        }));

        let receiver = self.subscriber_receiver.clone();

        for message in receiver {
            if let Ok(topic) = LifeCycleTopic::try_from(&message.0[..]) {
                self.handle_life_cycle_message(topic).unwrap_or_else(|_| {
                    error!("Could not properly handle lifecycle message, skipping.")
                });;
            }

            if let Ok(topic) = ComponentTopic::try_from(&message.0[..]) {
                self.handle_component_message(topic, message.1)
                    .unwrap_or_else(|_| {
                        error!("Could not properly handle component message, skipping.")
                    });
            }
        }

        Ok(())
    }

    fn handle_life_cycle_message(&mut self, topic: LifeCycleTopic) -> Result<(), failure::Error> {
        info!("Received a lifecycle topic");

        if topic.device == Device::Simulator && topic.handler == Handler::Connect {
            info!("Received a connect");

            self.stop_runners()?;
            self.reset()?;

            info!("Starting traffic light and bridge threads");
            let traffic_lights_runner = Arc::clone(&self.traffic_lights_runner);
            self.traffic_lights_runner_handle = Some(thread::spawn(move || {
                traffic_lights_runner
                    .run()
                    .unwrap_or_else(|e| error!("{}", e));
            }));

            let bridge_runner = Arc::clone(&self.bridge_runner);
            self.bridge_runner_handle = Some(thread::spawn(move || {
                bridge_runner.run().unwrap_or_else(|e| error!("{}", e));
            }));
        } else if topic.device == Device::Simulator && topic.handler == Handler::Disconnect {
            warn!("Received a disconnect");

            self.stop_runners()?;
            self.reset()?;
        }

        Ok(())
    }

    fn handle_component_message(
        &self,
        topic: ComponentTopic,
        payload: String,
    ) -> Result<(), failure::Error> {
        let payload_int = payload.parse::<i32>()?;
        let state = SensorState::try_from(payload_int)?;

        if let Some(sensor) = self.traffic_lights.read().unwrap().find_sensor(topic.uid) {
            sensor.write().unwrap().set_state(state)?;
        }

        if let Some(sensor) = self.bridge.read().unwrap().find_sensor(topic.uid) {
            sensor.write().unwrap().set_state(state)?;
        }

        Ok(())
    }

    fn reset(&self) -> Result<(), failure::Error> {
        info!("Resetting all states and scores");
        for group in self.traffic_lights.read().unwrap().groups.values() {
            group.read().unwrap().reset_all()?;
            group.write().unwrap().reset_score()?;
        }

        for group in self.bridge.read().unwrap().groups.values() {
            group.read().unwrap().reset_all()?;
            group.write().unwrap().reset_score()?;
        }

        Ok(())
    }

    fn stop_runners(&mut self) -> Result<(), failure::Error> {
        if self.traffic_lights_runner_handle.is_some() && self.bridge_runner_handle.is_some() {
            self.stop_runners.store(true, Release);
            self.stop_runners_sender.send(())?;

            self.traffic_lights_runner_handle
                .take()
                .unwrap()
                .join()
                .unwrap_or_else(|_| error!("Could not join traffic lights thread"));

            self.bridge_runner_handle
                .take()
                .unwrap()
                .join()
                .unwrap_or_else(|_| error!("Could not join traffic lights thread"));

            self.stop_runners.store(false, Release);
        }

        Ok(())
    }
}
