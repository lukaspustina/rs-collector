//! rs-collector: An scollector compatible telemetry collector written in Rust.

extern crate clap;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate rs_collector;
extern crate time;

use clap::{Arg, ArgMatches, App};
use log::SetLoggerError;
use std::env;
use std::error::Error;
use std::path::Path;

use rs_collector::config::Config;

static VERSION: &'static str = env!("CARGO_PKG_VERSION");
static DEFAULT_CONFIG_FILE: &'static str = "/etc/rs-collector.conf";

fn main() {
    if init_logger().is_err() {
        exit_with_error("Could not initialize logger", -1);
    }

    let app = App::new("rs-collector")
        .version(VERSION)
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

    std::panic::set_hook(Box::new(print_panic_and_abort));

    run(&config);
}

fn print_panic_and_abort(info: &std::panic::PanicInfo<'_>) {
    if let Some(loc) = info.location() {
        println!("Application panicked at {}", loc);
    } else {
        println!("Application panicked");
    }

    if let Some(msg) = info.payload().downcast_ref::<&str>() {
        println!("Reason: {}", msg);
    }

    std::process::abort()
}

fn init_logger() -> Result<(), SetLoggerError> {
    use log::{LogRecord, LogLevelFilter};
    use env_logger::LogBuilder;

    let format = |record: &LogRecord| {
        let t = time::now();
        format!("{},{:03} - {} - {} - {}:{}",
                time::strftime("%Y-%m-%d %H:%M:%S", &t).unwrap(),
                t.tm_nsec / 1000_000,
                record.level(),
                record.args(),
                record.location().file(),
                record.location().line(),
        )
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init()
}

fn parse_args(cli_args: &ArgMatches) -> Result<Config, Box<dyn Error>> {
    let config_file_path = Path::new(cli_args.value_of("configfile").unwrap_or(DEFAULT_CONFIG_FILE));
    let config: Config = if config_file_path.exists() {
        let config = r#try!(Config::load_from_rs_collector_config(&config_file_path));
        config
    } else {
        Default::default()
    };

    Ok(config)
}

fn run(config: &Config) {
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
