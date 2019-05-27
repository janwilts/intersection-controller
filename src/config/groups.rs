use serde::Deserialize;

use crate::config::config_file::ConfigFile;

#[derive(Deserialize)]
pub struct Group {
    pub kind: String,
    pub min_go_time: i32,
    pub min_transition_time: i32,
}

#[derive(Deserialize)]
pub struct Groups {
    pub groups: Vec<Group>,
}

impl<'s> ConfigFile<'s> for Groups {
    type Output = Groups;
}
