use bosun::{Metadata, Sample};
use config::Config;

use std::fmt;


#[derive(Debug)]
pub enum Error {
    InitError(String),
    CollectionError(String),
    ShutdownError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InitError(ref msg) => write!(f, "Collectors error: Init failed because {}", msg),
            &Error::CollectionError(ref msg) => write!(f, "Collectors error: Collection failed because {}", msg),
            &Error::ShutdownError(ref msg) => write!(f, "Collectors error: Shutdown failed because {}", msg),
        }
    }
}

pub type Id = String;

pub trait Collector {
    // This method must be re-callable. That means, this method may be called several times during start and failure mitigation.
    fn init(&mut self) -> Result<(), Box<Error>>;
    fn id(&self) -> &Id;
    fn collect(&self) -> Result<Vec<Sample>, Error>;
    fn shutdown(&self);
    fn metadata(&self) -> Vec<Metadata>;
}

pub fn create_collectors(config: &Config) -> Vec<Box<Collector + Send>> {
    let mut collectors = Vec::new();

    // Create Galera collector instances
    let mut galeras = galera::create_instances(config);
    collectors.append(&mut galeras);

    // Create HasIpAddr collector instance
    let mut hasipaddr = hasipaddr::create_instances(config);
    collectors.append(&mut hasipaddr);

    collectors
}

pub mod hasipaddr;
pub mod galera;

