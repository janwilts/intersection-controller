use failure::Fail;
use rumqtt::QoS;

use crate::config::io::MqConnection;
use crate::config::protocols::{Protocol, Protocols};
use crate::io::client::Client;

#[derive(Debug, Fail)]
pub enum ClientBuildError {
    #[fail(display = "Invalid protocol: {}", protocol)]
    InvalidProtocol { protocol: String },
}

pub struct ClientBuilder<'a> {
    io_config: &'a MqConnection,
    protocols_config: &'a Protocols,
    team_id: i32,
}

impl<'a> ClientBuilder<'a> {
    pub fn new(io_config: &'a MqConnection, protocols_config: &'a Protocols, team_id: i32) -> Self {
        Self {
            io_config,
            protocols_config,
            team_id,
        }
    }

    pub fn finalize(&self) -> Result<Client, failure::Error> {
        let protocol = self.find_protocol()?;

        let client = Client::new(
            self.io_config.client_id.clone(),
            self.io_config.host.clone(),
            protocol.port as u16,
            QoS::from_u8(self.io_config.qos as u8)?,
            self.team_id,
        );

        Ok(client)
    }

    fn find_protocol(&self) -> Result<&Protocol, failure::Error> {
        let protocol = self.io_config.protocol.clone();

        let filtered: Vec<&Protocol> = self
            .protocols_config
            .protocols
            .iter()
            .filter(|p| p.name == protocol)
            .collect();

        if let Some(protocol) = filtered.first() {
            return Ok(protocol);
        }

        Err(ClientBuildError::InvalidProtocol { protocol }.into())
    }
}
