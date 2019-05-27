extern crate chrono;
extern crate colored;
extern crate config as conf;
extern crate crossbeam_channel;
extern crate dotenv;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate regex;
extern crate rumqtt;
extern crate sentry;
extern crate serde;
extern crate time;

use std::env;
use std::process;

use crossbeam_channel::unbounded;
use dotenv::dotenv;

use crate::config::Config;
use crate::core::controller::Controller;
use crate::intersections::intersection_builder::IntersectionsBuilder;
use crate::io::client_builder::ClientBuilder;
use crate::io::topics::lifecycle_topic::{Device, Handler, LifeCycleTopic};

mod config;
mod core;
mod intersections;
mod io;

fn main() {
    // Load environment variables.
    dotenv().unwrap();

    // Set up logging.
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    log_builder.parse_filters(&format!("{}=debug", env!("CARGO_PKG_NAME")).replace("-", "_"));

    // Connect to sentry.
    let _sentry = sentry::init(env::var("SENTRY_DSN").unwrap());

    // Integrate sentry.
    sentry::integrations::env_logger::init(Some(log_builder.build()), Default::default());
    sentry::integrations::panic::register_panic_handler();

    let cfg = match Config::new("config") {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("{}", err);
            process::exit(1);
        }
    };

    let (notification_sender, notification_receiver) = unbounded();

    let traffic_lights = IntersectionsBuilder::new(notification_sender.clone())
        .with_defs(&cfg.traffic_lights)
        .with_blocks(&cfg.traffic_lights_blocks)
        .finish()
        .unwrap();

    let bridge = IntersectionsBuilder::new(notification_sender.clone())
        .with_defs(&cfg.bridge)
        .finish()
        .unwrap();

    let mut publisher = ClientBuilder::new(&cfg.io.publisher, &cfg.protocols, cfg.general.team_id)
        .finalize()
        .unwrap();

    publisher.set_last_will(
        Box::new(LifeCycleTopic::new(Device::Controller, Handler::Disconnect)),
        vec![],
    );

    let subscriber = ClientBuilder::new(&cfg.io.subscriber, &cfg.protocols, cfg.general.team_id)
        .finalize()
        .unwrap();

    let mut controller = Controller::new(traffic_lights, bridge);
    controller
        .start(publisher, subscriber)
        .unwrap_or_else(|_| error!("Something went wrong."));
}
