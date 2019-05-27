use serde::Deserialize;

use crate::config::config_file::ConfigFile;

#[derive(Deserialize)]
pub struct MqConnection {
    pub client_id: String,
    pub host: String,
    pub protocol: String,
    pub qos: i32,
}

#[derive(Deserialize)]
pub struct Io {
    pub publisher: MqConnection,
    pub subscriber: MqConnection,
}

impl<'s> ConfigFile<'s> for Io {
    type Output = Io;
}
