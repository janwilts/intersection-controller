use serde::Deserialize;

use crate::config::config_file::ConfigFile;

#[derive(Deserialize, Clone)]
pub struct Protocol {
    pub name: String,
    pub port: i32,
}

#[derive(Deserialize, Clone)]
pub struct Protocols {
    pub protocols: Vec<Protocol>,
}

impl<'s> ConfigFile<'s> for Protocols {
    type Output = Protocols;
}
