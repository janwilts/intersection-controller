extern crate chrono;
extern crate config as conf;
#[macro_use]
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

use chrono::Local;
use crossbeam_channel::unbounded;

use crate::config::Config;
use crate::core::controller::Controller;
use crate::intersections::intersection_builder::IntersectionsBuilder;
use crate::io::client_builder::ClientBuilder;
use crate::io::topics::lifecycle_topic::{Device, Handler, LifeCycleTopic};
use colored::Color;
use fern::colors::ColoredLevelConfig;
use log::LevelFilter;

mod config;
mod core;
mod intersections;
mod io;

fn main() -> Result<(), failure::Error> {
    // Set up logging.
    set_up_logger()?;

    std::panic::set_hook(Box::new(|info| {
        println!("{}", info);
    }));

    let config = Config::new("config").expect("Could not read config");

    let (notification_sender, notification_receiver) = unbounded();

    let traffic_lights = IntersectionsBuilder::new(notification_sender.clone())
        .with_defs(&config.traffic_lights)
        .with_blocks(&config.traffic_lights_blocks)
        .finish()?;

    let bridge = IntersectionsBuilder::new(notification_sender.clone())
        .with_defs(&config.bridge)
        .finish()?;

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
    controller.start(publisher, subscriber)?;

    Ok(())
}

fn set_up_logger() -> Result<(), failure::Error> {
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::White)
        .debug(Color::BrightBlue)
        .trace(Color::Magenta);

    let mqtt_log = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target().to_uppercase(),
                message
            ))
        })
        .level(LevelFilter::Off)
        .level_for("mqtt", LevelFilter::Trace)
        .chain(fern::log_file("log/mqtt.log")?);

    let log = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                colors.color(record.level()),
                message
            ))
        })
        .level(LevelFilter::Off)
        .level_for("intersection_controller", LevelFilter::Trace)
        .chain(std::io::stdout());

    fern::Dispatch::new().chain(mqtt_log).chain(log).apply()?;

    Ok(())
}
