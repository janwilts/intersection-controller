use crate::config::config_file::ConfigFile;

#[derive(Deserialize)]
pub struct Block {
    pub kind: String,
    pub id: i32,
}

#[derive(Deserialize)]
pub struct Group {
    pub blocks: Vec<Block>,
    pub kind: String,
    pub id: i32,
}

#[derive(Deserialize)]
pub struct Blocks {
    pub groups: Vec<Group>,
}

impl<'s> ConfigFile<'s> for Blocks {
    type Output = Blocks;
}
