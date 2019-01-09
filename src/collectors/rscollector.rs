use bosun::{Metadata, Rate, Sample};
use collectors::*;
use config::Config;

use std::num::ParseFloatError;
#[cfg(target_os = "linux")]
use procinfo::pid;

pub static RS_COLLECTOR_STATS_SAMPLES_METRICNAME: &'static str = "rs-collector.stats.samples";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct RsCollectorConfig {}

#[derive(Clone)]
pub struct RsCollector {
    id: Id,
}

pub fn create_instances(_: &Config) -> Vec<Box<Collector + Send>> {
    let id = format!("rscollector");
    info!("Created instance of RsCollector collector: {}", id);

    let collector = RsCollector { id: id };
    vec![Box::new(collector)]
}

impl Collector for RsCollector {
    fn init(&mut self) -> Result<(), Box<Error>> {
        Ok(())
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let samples = try!(collect_internal_metrics());
        debug!("{:#?}", samples);

        Ok(samples)
    }

    fn shutdown(&mut self) {}

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new("rs-collector.version",
                          Rate::Gauge,
                          "",
                          "Shows the version 'x.y.z' of rs-collector as x*1.000.0000 + y*1000 + z."),
            Metadata::new("rs-collector.stats.rss",
                          Rate::Gauge,
                          "KB",
                          "Shows the resident set size (physical memory) in KB consumed by rs-collector; if supported."),
            // This value is actually computed and send in the Bosun module directly.
            Metadata::new(RS_COLLECTOR_STATS_SAMPLES_METRICNAME,
                          Rate::Gauge,
                          "Samples",
                          "Shows the number of transmitted samples."),
        ]
    }
}

fn collect_internal_metrics() -> Result<Vec<Sample>, Error> {
    let mut samples = Vec::new();

    let version = parse_version_string(VERSION).or(Some(-1f64)).unwrap();
    samples.push(Sample::new("rs-collector.version", version));

    let rss = get_rss().or(Some(-1f64)).unwrap();
    samples.push(Sample::new("rs-collector.stats.rss", rss));

    Ok(samples)
}

fn parse_version_string<T: Into<String>>(version: T) -> Option<f64> {
    let version_triple: Vec<Result<f64, ParseFloatError>> = version.into()
        .split('.')
        .map(|s| s.parse::<f64>())
        .filter(|f| f.is_ok())
        .collect();

    if version_triple.len() == 3 {
        let version_value = version_triple[0].as_ref().unwrap() * 1000000f64
            + version_triple[1].as_ref().unwrap() * 1000f64
            + version_triple[2].as_ref().unwrap();

        Some(version_value)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn get_rss() -> Option<f64> {
    let status = pid::status_self();
    status.ok().map(|s| s.vm_rss as f64)
}

#[cfg(not(target_os = "linux"))]
fn get_rss() -> Option<f64> {
    None
}
