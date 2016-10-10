use bosun::{Metadata, Sample};
use config::Config;


#[derive(Debug)]
pub enum Error {
    InitError(String),
    CollectionError(String),
    ShutdownError(String),
}

pub type Id = String;

pub trait Collector {
    fn init(&mut self) -> Result<(), Box<Error>>;
    fn id(&self) -> &Id;
    fn collect(&self) -> Vec<Sample>;
    fn shutdown(&self);
    fn metadata(&self) -> Vec<Metadata>;
}

pub fn create_collectors(config: &Config) -> Vec<Box<Collector + Send>> {
    let mut collectors = Vec::new();

    // Create Galera collector instances
    let mut galeras = galera::create_instances(config);
    collectors.append(&mut galeras);

    collectors
}

pub mod galera;

