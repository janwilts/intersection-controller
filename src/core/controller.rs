use std::convert::TryFrom;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Release;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::config::Config;
use crate::core::bridge_runner::BridgeRunner;
use crate::core::message_publisher::{Message, MessagePublisher};
use crate::core::message_subscriber::MessageSubscriber;
use crate::core::score_poller::ScorePoller;
use crate::core::state_publisher::StatePublisher;
use crate::core::traffic_lights_runner::TrafficLightsRunner;
use crate::intersections::component::{Component, ComponentId, ComponentKind};
use crate::intersections::group::{ArcGroup, GroupId, GroupKind};
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
            subscriber
                .subscribe(Box::new(ComponentTopic::from(sensor.read().unwrap().uid())))
                .expect("Could not subscribe");
        }

        for sensor in self.bridge.read().unwrap().sensors() {
            subscriber
                .subscribe(Box::new(ComponentTopic::from(sensor.read().unwrap().uid())))
                .expect("Could not subscribe");
        }

        debug!("Subscribing to lifecycle topics");
        subscriber
            .subscribe(Box::new(LifeCycleTopic::new(
                Device::Simulator,
                Handler::Connect,
            )))
            .expect("Could not subscribe");

        subscriber
            .subscribe(Box::new(LifeCycleTopic::new(
                Device::Simulator,
                Handler::Disconnect,
            )))
            .expect("Could not subscribe");

        // Publisher
        let publisher_receiver = self.publisher_receiver.clone();
        self.message_publisher_handle = Some(thread::spawn(move || {
            let publisher = MessagePublisher::new(publisher, publisher_receiver);
            publisher.run();
        }));

        // Subscriber
        let subscriber_sender = self.subscriber_sender.clone();
        self.message_subscriber_handle = Some(thread::spawn(move || {
            let subscriber = MessageSubscriber::new(subscriber, subscriber_sender);
            subscriber.run();
        }));

        let state_publisher = Arc::clone(&self.state_publisher);
        self.state_publisher_handle = Some(thread::spawn(move || {
            state_publisher
                .run()
                .expect("Something went wrong in the state publisher.");
        }));

        // Score Poller
        let score_poller = Arc::clone(&self.score_poller);
        self.score_poller_handle = Some(thread::spawn(move || {
            score_poller.run();
        }));

        let receiver = self.subscriber_receiver.clone();

        for message in receiver {
            if let Ok(topic) = LifeCycleTopic::try_from(&message.0[..]) {
                self.handle_life_cycle_message(topic);
            }

            if let Ok(topic) = ComponentTopic::try_from(&message.0[..]) {
                self.handle_component_message(topic, message.1);
            }
        }

        Ok(())
    }

    fn handle_life_cycle_message(&mut self, topic: LifeCycleTopic) {
        info!("Received a lifecycle topic");

        if topic.device == Device::Simulator && topic.handler == Handler::Connect {
            info!("Received a connect");

            self.stop_runners();
            self.reset();

            info!("Starting traffic light and bridge threads");
            let traffic_lights_runner = Arc::clone(&self.traffic_lights_runner);
            self.traffic_lights_runner_handle = Some(thread::spawn(move || {
                traffic_lights_runner.run();
            }));

            let bridge_runner = Arc::clone(&self.bridge_runner);
            self.bridge_runner_handle = Some(thread::spawn(move || {
                bridge_runner.run();
            }));
        } else if topic.device == Device::Simulator && topic.handler == Handler::Disconnect {
            warn!("Received a disconnect");

            self.stop_runners();
            self.reset();
        }
    }

    fn handle_component_message(&self, topic: ComponentTopic, payload: String) {
        let payload_int = payload.parse::<i32>().unwrap();
        let state = SensorState::try_from(payload_int).unwrap();

        if let Some(sensor) = self.traffic_lights.read().unwrap().find_sensor(topic.uid) {
            sensor.write().unwrap().set_state(state);

            if sensor.read().unwrap().group().read().unwrap().id
                == (GroupId {
                    kind: GroupKind::MotorVehicle,
                    id: 14,
                })
            {
                self.handle_jam_sensor(Arc::clone(&sensor.read().unwrap().group()));
            }
        }

        if let Some(sensor) = self.bridge.read().unwrap().find_sensor(topic.uid) {
            sensor.write().unwrap().set_state(state);
        }
    }

    fn handle_jam_sensor(&self, group: ArcGroup) {
        let sensor = group
            .read()
            .unwrap()
            .find_sensor(ComponentId {
                kind: ComponentKind::Sensor,
                id: 1,
            })
            .unwrap();

        if sensor.read().unwrap().triggered_for(Duration::from_secs(5)) {
            warn!("Bridge queue too full! Blocking traffic lights");

            for blockable in self.traffic_lights.read().unwrap().blockable_groups() {
                blockable.write().unwrap().block = true;
            }
        } else {
            for blockable in self.traffic_lights.read().unwrap().blockable_groups() {
                blockable.write().unwrap().block = false;
            }
        }
    }

    fn reset(&self) {
        info!("Resetting all states and scores");
        for group in self.traffic_lights.read().unwrap().groups.values() {
            group.read().unwrap().reset_all();
            group.write().unwrap().reset_score();
        }

        for group in self.bridge.read().unwrap().groups.values() {
            group.read().unwrap().reset_all();
            group.write().unwrap().reset_score();
        }
    }

    fn stop_runners(&mut self) {
        if self.traffic_lights_runner_handle.is_some() && self.bridge_runner_handle.is_some() {
            self.stop_runners.store(true, Release);
            self.stop_runners_sender
                .send(())
                .expect("Could not send stop notification");

            self.traffic_lights_runner_handle
                .take()
                .unwrap()
                .join()
                .expect("Could not join traffic lights thread");
            self.bridge_runner_handle
                .take()
                .unwrap()
                .join()
                .expect("Could not join traffic bridge thread");

            self.stop_runners.store(false, Release);
        }
    }
}
