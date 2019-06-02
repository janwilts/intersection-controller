use crate::config::config_file::ConfigFile;

#[derive(Deserialize)]
pub struct Component {
    pub kind: String,
    pub id: i32,
    pub distance: Option<i32>,
    pub initial_state: Option<i32>,
}

#[derive(Deserialize)]
pub struct Group {
    pub kind: String,
    pub id: i32,
    pub special: Option<bool>,
    pub components: Option<Vec<Component>>,
}

#[derive(Deserialize)]
pub struct Definitions {
    pub groups: Vec<Group>,
}

impl<'s> ConfigFile<'s> for Definitions {
    type Output = Definitions;
}
