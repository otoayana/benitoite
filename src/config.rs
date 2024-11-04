use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base: Base,
    pub accounts: HashMap<String, Account>,
}

#[derive(Debug, Deserialize)]
pub struct Base {
    pub bind: String,
    pub cert: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub pds: String,
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn parse() -> Result<Config, Box<dyn std::error::Error>> {
        let mut file = File::open("config.toml")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        Ok(toml::from_str(&contents)?)
    }
}
