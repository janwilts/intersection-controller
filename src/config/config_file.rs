use conf::{Config, ConfigError, File};
use serde::Deserialize;

pub trait ConfigFile<'s>
where
    Self::Output: Deserialize<'s>,
{
    type Output;

    fn new(dir: &str, file: &str) -> Result<Self::Output, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name(&format!("{}/{}", dir, file)))?;
        s.try_into()
    }
}
