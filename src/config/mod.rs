use conf::ConfigError;

use crate::config::blocks::Blocks;
use crate::config::config_file::ConfigFile;
use crate::config::definitions::Definitions;
use crate::config::general::General;
use crate::config::groups::Groups;
use crate::config::io::Io;
use crate::config::protocols::Protocols;

pub mod blocks;
mod config_file;
pub mod definitions;
pub mod general;
pub mod groups;
pub mod io;
pub mod protocols;

pub struct Config {
    pub traffic_lights_blocks: Blocks,
    pub traffic_lights: Definitions,
    pub bridge: Definitions,
    pub general: General,
    pub groups: Groups,
    pub io: Io,
    pub protocols: Protocols,
}

impl Config {
    pub fn new(dir: &str) -> Result<Self, ConfigError> {
        Ok(Self {
            traffic_lights_blocks: Blocks::new(dir, "blocks.toml")?,
            traffic_lights: Definitions::new(dir, "traffic_lights.toml")?,
            bridge: Definitions::new(dir, "bridge.toml")?,
            general: General::new(dir, "general.toml")?,
            groups: Groups::new(dir, "groups.toml")?,
            io: Io::new(dir, "io.toml")?,
            protocols: Protocols::new(dir, "protocols.toml")?,
        })
    }
}
