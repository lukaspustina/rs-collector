//! rs-collector: An scollector compatible telemetry collector written in Rust.

extern crate clap;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate rs_collector;

use clap::{Arg, ArgMatches, App};
use std::error::Error;
use std::path::Path;

use rs_collector::config::Config;

static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static DEFAULT_CONFIG_FILE: &'static str = "/etc/rs-collector.conf";

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
        .arg(Arg::with_name("configfile")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .arg(Arg::with_name("show-config")
            .long("show-config")
            .help("Prints config"));
    let cli_args = app.get_matches();

    let verbose: bool = cli_args.is_present("verbose");
    let config: Config = match parse_args(&cli_args) {
        Ok(config) => config,
        Err(err) => {
            exit_with_error(&format!("Failed to parse configuration, because {}.", err), -2);
        }
    };
    // TODO a config must have been parsed; if not exit!
    if cli_args.is_present("show-config") {
        println!("config: {:?}", config);
    }

    run(&config, verbose);
}

fn parse_args(cli_args: &ArgMatches) -> Result<Config, Box<Error>> {
    let config_file_path = Path::new(cli_args.value_of("configfile").unwrap_or(DEFAULT_CONFIG_FILE));
    let mut config: Config = if config_file_path.exists() {
        let config = try!(Config::load_from_rs_collector_config(&config_file_path));
        config
    } else {
        Default::default()
    };

    Ok(config)
}

fn run(config: &Config, verbose: bool) {

    let collectors = rs_collector::collectors::create_collectors(config);
    rs_collector::scheduler::run(collectors, config);
}

fn exit_with_error(msg: &str, exit_code: i32) -> ! {
    println!("{}", msg);
    std::process::exit(exit_code);
}


#[cfg(test)]
mod tests {
}
