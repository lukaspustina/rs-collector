#[deny(missing_docs)]

extern crate bosun_emitter;
#[macro_use]
extern crate chan;
extern crate chan_signal;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate get_if_addrs;
extern crate is_executable;
extern crate itertools;
#[macro_use]
extern crate mongodb;
extern crate mysql;
#[cfg(target_os = "linux")]
extern crate procinfo;
extern crate regex;
extern crate rustc_serialize;
extern crate toml;

pub mod bosun;
extern crate chrono;
pub mod collectors;
pub mod config;
pub mod scheduler;
pub mod utils;

enum Msg<T> {
    Collector(String, T)
}
