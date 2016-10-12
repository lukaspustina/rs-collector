#[deny(missing_docs)]

extern crate bosun_emitter;
#[macro_use]
extern crate chan;
extern crate chan_signal;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate mysql;
extern crate rustc_serialize;
extern crate toml;


pub mod bosun;
pub mod collectors;
pub mod config;
pub mod scheduler;
pub mod utils;

enum Msg<T> {
    Collector(String, T)
}
