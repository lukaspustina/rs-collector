use bosun_emitter::Tags;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

use collectors::galera::GaleraConfig;
use collectors::hasipaddr::HasIpAddrConfig;
use collectors::jvm::JvmConfig;
use collectors::postfix::PostfixConfig;
use collectors::mongo::MongoConfig;
use collectors::megaraid::MegaraidConfig;

#[derive(Debug)]
#[derive(Deserialize)]
#[allow(non_snake_case)]
/// Represents connection parameters to reach Bosun as well as default tags to append to each metric
/// datum.
pub struct Config {
    /// Bosun server host name
    pub Host: String,
    /// Local host name
    pub Hostname: String,
    /// Tags to always append to each metric
    pub Tags: Tags,
    /// Galera config; if enabled
    pub Galera: Option<GaleraConfig>,
    /// HasIpAddr config; if enabled
    pub HasIpAddr: Option<HasIpAddrConfig>,
    /// JVM config; if enabled
    pub Jvm: Option<Vec<JvmConfig>>,
    /// Mongo configs; if enabled
    pub Mongo: Option<Vec<MongoConfig>>,
    /// Postfix config; if enabled
    pub Postfix: Option<PostfixConfig>,
    /// Postfix config; if enabled
    pub Megaraid: Option<MegaraidConfig>,
    /// Deactivate Data Transmission to Bosun
    pub DontSend: Option<bool>
}

impl Config {
    /// Loads a configuration from an [SCollector](http://bosun.org/scollector/) configuration file.
    pub fn load_from_rs_collector_config(file_path: &Path) -> Result<Config, Box<::std::error::Error>> {
        match Config::load_file(file_path) {
            Ok(toml) => {
                let config: Config = try!(toml::from_str(&toml));

                Ok(config)
            }
            Err(err) => Err(err),
        }
    }

    fn load_file(file_path: &Path) -> Result<String, Box<::std::error::Error>> {
        let mut config_file = try!(File::open(file_path));
        let mut config_content = String::new();
        try!(config_file.read_to_string(&mut config_content));

        Ok(config_content)
    }
}

impl Default for Config {
    /// Creates a default configuration for `localhost`, port `8070`.
    fn default() -> Config {
        Config {
            Host: "localhost:8070".to_string(),
            Hostname: "localhost".to_string(),
            Tags: Tags::new(),
            Galera: None,
            HasIpAddr: None,
            Jvm: None,
            Mongo: None,
            Postfix: None,
            Megaraid: None,
            DontSend: Some(false),
        }
    }
}
