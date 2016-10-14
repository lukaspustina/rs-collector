use bosun_emitter::Tags;
use rustc_serialize::Decodable;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

use collectors::galera::GaleraConfig;
use collectors::hasipaddr::HasIpAddrConfig;

#[derive(Debug)]
#[derive(RustcDecodable)]
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
}

impl Config {
    /// Loads a configuration from an [SCollector](http://bosun.org/scollector/) configuration file.
    pub fn load_from_rs_collector_config(file_path: &Path) -> Result<Config, Box<::std::error::Error>> {
        match Config::load_toml(file_path) {
            Ok(toml) => {
                let mut decoder = toml::Decoder::new(toml);
                let config = try!(Config::decode(&mut decoder));

                Ok(config)
            }
            Err(err) => Err(err),
        }
    }

    fn load_toml(file_path: &Path) -> Result<toml::Value, Box<::std::error::Error>> {
        let mut config_file = try!(File::open(file_path));
        let mut config_content = String::new();
        try!(config_file.read_to_string(&mut config_content));

        let mut parser = toml::Parser::new(&config_content);
        match parser.parse() {
            Some(toml) => Ok(toml::Value::Table(toml)),
            None => Err(From::from(parser.errors.pop().unwrap())),
        }
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
        }
    }
}
