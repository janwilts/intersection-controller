extern crate chrono;
extern crate config as conf;
extern crate crossbeam_channel;
#[macro_use]
extern crate failure;
extern crate fern;
#[macro_use]
extern crate log;
extern crate regex;
extern crate rumqtt;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate time;

use std::process;

use crossbeam_channel::unbounded;

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
    // Set up logging.
    let mqtt = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target().to_uppercase(),
                message
            ))
        })
        .level(log::LevelFilter::Off)
        // but accept Info if we explicitly mention it
        .level_for("mqtt", log::LevelFilter::Trace)
        .chain(fern::log_file("log/mqtt.log").unwrap());

    let test = fern::Dispatch::new()
        .level(log::LevelFilter::Off)
        // but accept Info if we explicitly mention it
        .level_for("test", log::LevelFilter::Trace)
        .chain(std::io::stdout());


    let logger = fern::Dispatch::new().chain(mqtt).chain(test).apply().unwrap();

    warn!(target: "mqtt", "Test");

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
