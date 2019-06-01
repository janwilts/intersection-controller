extern crate chrono;
extern crate config as conf;
extern crate crossbeam_channel;
extern crate ctrlc;
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

use chrono::Local;
use crossbeam_channel::unbounded;
use log::LevelFilter;
use serde::export::fmt;

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
    set_up_logger().expect("Could not set up logging");

    let config = Config::new("config").expect("Could not read config");

    let (notification_sender, notification_receiver) = unbounded();

    let traffic_lights = IntersectionsBuilder::new(notification_sender.clone())
        .with_defs(&config.traffic_lights)
        .with_blocks(&config.traffic_lights_blocks)
        .finish()
        .expect("Could not construct traffic lights");

    let bridge = IntersectionsBuilder::new(notification_sender.clone())
        .with_defs(&config.bridge)
        .finish()
        .expect("Could not construct bridge");

    let mut publisher = ClientBuilder::new(
        &config.io.publisher,
        &config.protocols,
        config.general.team_id,
    )
    .finalize()
    .unwrap();

    publisher.set_last_will(
        Box::new(LifeCycleTopic::new(Device::Controller, Handler::Disconnect)),
        vec![],
    );

    let subscriber = ClientBuilder::new(
        &config.io.subscriber,
        &config.protocols,
        config.general.team_id,
    )
    .finalize()
    .unwrap();

    let mut controller = Controller::new(traffic_lights, bridge, notification_receiver, config);
    controller.start(publisher, subscriber);
}

fn set_up_logger() -> Result<(), failure::Error> {
    let formatter = |out: fern::FormatCallback, message: &fmt::Arguments, record: &log::Record| {
        out.finish(format_args!(
            "[{}] [{}] [{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target().to_uppercase(),
            message
        ))
    };

    let short_formatter = |out: fern::FormatCallback, message: &fmt::Arguments, _: &log::Record| {
        out.finish(format_args!(
            "[{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            message
        ))
    };

    let mqtt_log = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Off)
        .level_for("mqtt", LevelFilter::Trace)
        .chain(fern::log_file("log/mqtt.log")?);

    let state_log = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Off)
        .level_for("state", LevelFilter::Trace)
        .chain(fern::log_file("log/state.log")?);

    let traffic_lights_log = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Off)
        .level_for("traffic_lights", LevelFilter::Trace);

    let bridge_log = fern::Dispatch::new()
        .format(formatter)
        .level(LevelFilter::Off)
        .level_for("bridge", LevelFilter::Trace);

    let default_log = fern::Dispatch::new()
        .format(short_formatter)
        .level(LevelFilter::Off)
        .level_for("intersection_controller", LevelFilter::Trace)
        .chain(fern::log_file("log/log.log")?)
        .chain(std::io::stdout());

    fern::Dispatch::new()
        .chain(mqtt_log)
        .chain(state_log)
        .chain(traffic_lights_log)
        .chain(bridge_log)
        .chain(default_log)
        .apply()?;

    Ok(())
}
