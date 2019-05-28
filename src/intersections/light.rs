use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use failure;
use failure::Fail;

use crate::intersections::component::ComponentState;

#[derive(Debug, Fail)]
pub enum LightStateError {
    #[fail(display = "Could not create Light State from a \"{}\" value.", value)]
    CouldNotConvert { value: i32 },
}

#[derive(PartialEq, Copy, Clone)]
pub enum LightState {
    Prohibit,
    Transitioning,
    Proceed,
    OutOfOrder,
}

impl ComponentState for LightState {}

impl Display for LightState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LightState::Prohibit => write!(f, "{}", "PROHIBIT"),
            LightState::Transitioning => write!(f, "{}", "TRANSITIONING"),
            LightState::Proceed => write!(f, "{}", "PROCEED"),
            LightState::OutOfOrder => write!(f, "{}", "OUT_OF_ORDER"),
        }
    }
}

impl TryFrom<i32> for LightState {
    type Error = failure::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        match value {
            0 => Ok(LightState::Prohibit),
            1 => Ok(LightState::Transitioning),
            2 => Ok(LightState::Proceed),
            3 => Ok(LightState::OutOfOrder),
            _ => Err(LightStateError::CouldNotConvert { value }.into()),
        }
    }
}

impl Into<i32> for LightState {
    fn into(self) -> i32 {
        match self {
            LightState::Prohibit => 0,
            LightState::Transitioning => 1,
            LightState::Proceed => 2,
            LightState::OutOfOrder => 3,
        }
    }
}
