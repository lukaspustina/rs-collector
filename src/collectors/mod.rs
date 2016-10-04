use bosun::Sample;
use config::Config;


pub enum Error {
    Init,
    Collection,
    Shutdown,
}

pub type Id = String;

pub trait Collector {
    fn init(&self) -> Result<(), Box<Error>>;
    fn id(&self) -> &Id;
    fn collect(&self) -> Sample;
    fn shutdown(&self);
}

pub fn create_collectors(config: &Config) -> Vec<Box<Collector>> {
    let mut collectors = Vec::new();
    let mut galeras = galera::create_instances(config);
    collectors.append(&mut galeras);

    collectors
}

pub mod galera {
    use std::time::Duration;
    use std::thread;

    use bosun::Sample;
    use super::*;
    use super::super::config::Config;

    #[derive(Debug)]
    #[derive(RustcDecodable)]
    #[allow(non_snake_case)]
    pub struct GaleraConfig {
        pub User: String,
        pub Password: String,
        pub URL: String,
    }

    pub struct Galera {
        id: Id,
        user: String,
        password: String,
        url: String,
    }

    pub fn create_instances(config: &Config) -> Vec<Box<Collector>> {
        match config.Galera {
            Some(ref config) => {
                let id = format!("galera-{}@{}", config.User, config.Password);
                info!("Created instance of Galera collector: {}", id);

                let collector = Galera {
                    id: id, user: config.User.clone(), password: config.Password.clone(),
                    url: config.URL.clone()
                };
                vec![Box::new(collector)]
            }
            None => {
                Vec::new()
            }
        }
    }

    impl Collector for Galera {
        fn init(&self) -> Result<(), Box<Error>> {
            // Do init
            Ok(())
        }
        fn id(&self) -> &Id {
            &self.id
        }
        fn collect(&self) -> Sample {
            thread::sleep(Duration::from_secs(3));
            Sample { time: 1 }
        }
        fn shutdown(&self) {}
    }
}
