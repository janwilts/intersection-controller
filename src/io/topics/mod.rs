use failure::Fail;
use std::fmt::Display;

pub mod component_topic;
pub mod lifecycle_topic;

#[derive(Debug, Fail)]
#[fail(display = "No team ID has been set.")]
pub struct NoTeamIdSet;

pub trait Topic: Display + Send {
    fn team_id(&self) -> Result<i32, failure::Error>;
    fn set_team_id(&mut self, team_id: i32);
}
