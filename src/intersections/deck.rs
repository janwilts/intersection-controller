use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use failure::Fail;

use crate::intersections::component::ComponentState;

#[derive(Debug, Fail)]
pub enum DeckStateError {
    #[fail(display = "Could not create Deck State from a \"{}\" value.", value)]
    CouldNotConvert { value: i32 },
}

#[derive(PartialEq, Copy, Clone)]
pub enum DeckState {
    Open,
    Close,
}

impl ComponentState for DeckState {}

impl Default for DeckState {
    fn default() -> Self {
        DeckState::Close
    }
}

impl Display for DeckState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            DeckState::Open => write!(f, "{}", "OPEN"),
            DeckState::Close => write!(f, "{}", "CLOSE"),
        }
    }
}

impl TryFrom<i32> for DeckState {
    type Error = failure::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        match value {
            0 => Ok(DeckState::Open),
            1 => Ok(DeckState::Close),
            _ => Err(DeckStateError::CouldNotConvert { value }.into()),
        }
    }
}

impl Into<i32> for DeckState {
    fn into(self) -> i32 {
        match self {
            DeckState::Open => 0,
            DeckState::Close => 1,
        }
    }
}
