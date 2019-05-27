use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use colored::Colorize;
use failure::Fail;

use crate::intersections::component::ComponentState;

#[derive(Debug, Fail)]
pub enum GateStateError {
    #[fail(display = "Could not create Gate State from a \"{}\" value.", value)]
    CouldNotConvert { value: i32 },
}

#[derive(PartialEq, Copy, Clone)]
pub enum GateState {
    Open,
    Close,
}

impl ComponentState for GateState {}

impl Display for GateState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            GateState::Open => write!(f, "{}", "OPEN".green()),
            GateState::Close => write!(f, "{}", "CLOSE".red()),
        }
    }
}

impl TryFrom<i32> for GateState {
    type Error = failure::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        match value {
            0 => Ok(GateState::Open),
            1 => Ok(GateState::Close),
            _ => Err(GateStateError::CouldNotConvert { value }.into()),
        }
    }
}

impl Into<i32> for GateState {
    fn into(self) -> i32 {
        match self {
            GateState::Open => 0,
            GateState::Close => 1,
        }
    }
}
