use crate::config::config_file::ConfigFile;

#[derive(Deserialize)]
pub struct General {
    pub team_id: i32,
}

impl<'s> ConfigFile<'s> for General {
    type Output = General;
}
