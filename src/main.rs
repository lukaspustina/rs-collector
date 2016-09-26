//! rs-collector: An scollector compatible telemetry collector written in Rust.

extern crate clap;
extern crate env_logger;
#[macro_use] extern crate log;

use clap::{Arg, ArgMatches, App};
use std::error::Error;
use std::path::Path;

static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static DEFAULT_CONFIG_FILE: &'static str = "/etc/bosun/scollector.conf";

#[derive(Debug)]
struct Config {
    host: String,
    hostname: String,
}

impl Config {
    pub fn default() -> Config {
        Config {
            host: "".to_string(),
            hostname: "".to_string(),
        }
    }
}

fn main() {
    if env_logger::init().is_err() {
        exit_with_error("Could not initialize logger", -1);
    }

    let app = App::new("rs-collector")
        .version(VERSION)
        .after_help("Two modes are supported, i.e., sending a datum with meta data or \
                               sending only meta data. The modes are controlled whether a value \
                               `--value` is passed or not. Please mind that in both cases the \
                               meta data is required.")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .arg(Arg::with_name("show-config")
            .long("show-config")
            .help("Prints config"))
        .arg(Arg::with_name("verbose")
            .long("verbose")
            .help("Enables verbose output"));
    let cli_args = app.get_matches();

    let verbose: bool = cli_args.is_present("verbose");
    let config: Config = match parse_args(&cli_args) {
        Ok(config) => config,
        Err(err) => {
            exit_with_error(&format!("Failed to parse configuration, because {}.", err), -2);
        }
    };
    if cli_args.is_present("show-config") {
        println!("config: {:?}", config);
    }

    run(&config, verbose);
}

fn parse_args(cli_args: &ArgMatches) -> Result<Config, Box<Error>> {
    let bosun_config_file_path = Path::new(cli_args.value_of("config")
        .unwrap_or(DEFAULT_CONFIG_FILE));
    let mut config: Config = Config::default();

    Ok(config)
}


fn run(config: &Config, verbose: bool) {}


fn msg(msg: &str, verbose: bool) {
    if verbose {
        println!("{}", msg);
    }
}

fn exit_with_error(msg: &str, exit_code: i32) -> ! {
    println!("{}", msg);
    std::process::exit(exit_code);
}


#[cfg(test)]
mod tests {
    use super::{Config, parse_tags};

    #[test]
    fn parse_tags_test_okay() {
        let mut config = Config::default();
        let tags = "key1=val1,key2=val2";
        let _ = parse_tags(&mut config, &tags);
        assert_eq!(config.tags.len(), 2);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn parse_tags_test_fails_wrong_kv_separator() {
        let mut config = Config::default();
        let tags = "key1=val1,key2:val2";
        let _ = parse_tags(&mut config, &tags);
        assert_eq!(config.tags.len(), 2);
    }
}
