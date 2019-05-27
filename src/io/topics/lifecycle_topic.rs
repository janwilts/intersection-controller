use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use failure;
use regex::Regex;

use crate::io::topics::{NoTeamIdSet, Topic};

#[derive(Debug, Fail)]
enum LifeCycleTopicBuildError {
    #[fail(display = "Lifecycle topic could not be built: Invalid format.")]
    InvalidFormat,
}

#[derive(Debug, Fail)]
#[fail(display = "Unknown device: {}.", device)]
struct UnknownDevice {
    device: String,
}

#[derive(PartialEq)]
pub enum Device {
    Controller,
    Simulator,
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Device::Controller => write!(f, "controller"),
            Device::Simulator => write!(f, "simulator"),
        }
    }
}

impl TryFrom<&str> for Device {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "controller" => Ok(Device::Controller),
            "simulator" => Ok(Device::Simulator),
            _ => Err(UnknownDevice {
                device: String::from(value),
            }
            .into()),
        }
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Unknown handler: {}.", handler)]
struct UnknownHandler {
    handler: String,
}

#[derive(PartialEq)]
pub enum Handler {
    Connect,
    Disconnect,
}

impl Display for Handler {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Handler::Connect => write!(f, "onconnect"),
            Handler::Disconnect => write!(f, "ondisconnect"),
        }
    }
}

impl TryFrom<&str> for Handler {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "onconnect" => Ok(Handler::Connect),
            "ondisconnect" => Ok(Handler::Disconnect),
            _ => Err(UnknownHandler {
                handler: String::from(value),
            }
            .into()),
        }
    }
}

pub struct LifeCycleTopic {
    pub team_id: Option<i32>,
    pub device: Device,
    pub handler: Handler,
}

impl LifeCycleTopic {
    pub fn new(device: Device, handler: Handler) -> Self {
        Self {
            team_id: None,
            device,
            handler,
        }
    }
}

impl Topic for LifeCycleTopic {
    fn team_id(&self) -> Result<i32, failure::Error> {
        match self.team_id {
            Some(team_id) => Ok(team_id),
            None => Err(NoTeamIdSet.into()),
        }
    }

    fn set_team_id(&mut self, team_id: i32) {
        self.team_id = Some(team_id)
    }
}

impl TryFrom<&str> for LifeCycleTopic {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let regex = Regex::new(
            "^\\d+/features/lifecycle/(controller|simulator)/(onconnect|ondisconnect)$",
        )?;

        if !regex.is_match(value) {
            return Err(LifeCycleTopicBuildError::InvalidFormat.into());
        }

        let captures = regex.captures(value).unwrap();

        Ok(Self {
            team_id: Some(4),
            device: Device::try_from(&captures[1])?,
            handler: Handler::try_from(&captures[2])?,
        })
    }
}

impl Display for LifeCycleTopic {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}/features/lifecycle/{}/{}",
            self.team_id.unwrap(),
            self.device,
            self.handler
        )
    }
}
