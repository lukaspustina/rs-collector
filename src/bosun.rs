use crate::collectors::rscollector::RS_COLLECTOR_STATS_SAMPLES_METRICNAME;

use bosun_emitter::{BosunClient, Datum, EmitterResult};
use bosun_emitter;
use chan::Receiver;
use chan;
use std::thread::JoinHandle;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TICK_INTERVAL_SEC: u64 = 15u64;

pub type Tags = bosun_emitter::Tags;

// TODO: Replace with Bosun::Datum
#[derive(Debug)]
pub struct Sample {
    pub time: u64,
    pub metric: String,
    pub value: f64,
    pub tags: Tags,
}

impl Sample {
    pub fn new<T: Into<String>, U: Into<f64>>(metric: T, value: U) -> Self {
        Sample::new_with_tags(metric, value, Tags::new())
    }

    pub fn new_with_tags<T: Into<String>, U: Into<f64>>(metric: T, value: U, tags: Tags) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Sample { time: now, metric: metric.into(), value: value.into(), tags: tags }
    }
}

pub enum Rate {
    Gauge,
    Counter,
    Rate,
}

impl Into<String> for Rate {
    fn into(self) -> String {
        match self {
            Rate::Gauge => "gauge".to_string(),
            Rate::Counter => "counter".to_string(),
            Rate::Rate => "rate".to_string(),
        }
    }
}

// TODO: Replace with Bosun::Metadata
#[derive(Debug)]
pub struct Metadata {
    /// Metric name
    pub metric: String,
    /// Metric rate type: [gauge, counter, rate]
    pub rate: String,
    /// Metric unit
    pub unit: String,
    /// Metric description
    pub description: String,
}

impl Metadata {
    pub fn new<S: Into<String>, T: Into<String>, U: Into<String>>( metric: S, rate: Rate, unit: T, description: U )
        -> Self {
        Metadata { metric: metric.into(), rate: rate.into(), unit: unit.into(), description: description.into() }
    }
}

#[derive(Debug)]
pub enum BosunRequest {
    Sample(Sample),
    Metadata(Metadata),
    Shutdown,
}

pub struct Bosun {
    queue: Vec<Sample>,
    from_main_rx: Receiver<BosunRequest>,
    bosun_client: BosunClient,
    default_tags: Tags,
    hostname: String,
}

impl Bosun {
    pub fn new(host: &str, hostname: &str, default_tags: &Tags, from_main_rx: Receiver<BosunRequest>) -> Bosun {
        let bosun_client = BosunClient::new(host, 3);
        Bosun {
            queue: Vec::new(),
            from_main_rx: from_main_rx,
            bosun_client: bosun_client,
            default_tags: default_tags.clone(),
            hostname: hostname.to_string(),
        }
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        let timer = chan::tick(Duration::from_secs(TICK_INTERVAL_SEC));

        thread::spawn(move || {
            info!("Bosun thread started.");

            let from_main_rx = self.from_main_rx;
            loop {
                chan_select! {
                    timer.recv() => {
                        let queue_len = self.queue.len() as f64 + 1f64;
                        self.queue.push(Sample::new(RS_COLLECTOR_STATS_SAMPLES_METRICNAME, queue_len));
                        debug!("I've been ticked. Current sample queue length is {:#?}. Sending data now.", queue_len);
                        for s in self.queue.drain(..) {
                            match send_sample_to_bosun(s, &self.bosun_client, &self.hostname, &self.default_tags) {
                                Ok(_) => {},
                                Err(err) => {
                                    // TODO: We should not just drop the sample / datum; maybe we need to pass it back
                                    // to a ring buffer
                                    error!("Failed to send datum to Bosun, because {:?}", err);
                                }
                            }
                        }
                    },
                    from_main_rx.recv() -> msg => {
                        match msg {
                            Some(BosunRequest::Metadata(metadata)) => {
                                debug!("Received new metadata '{}'.", &metadata.metric);
                                let m = bosun_emitter::Metadata {
                                    metric: &metadata.metric, rate: &metadata.rate,
                                    unit: &metadata.unit, description: &metadata.description };
                                match self.bosun_client.emit_metadata(&m) {
                                    Ok(_) => {},
                                    Err(err) => {
                                        error!("Failed to send metadata '{:?}' to Bosun, because {:?}", &m, err);
                                    }
                                }
                            }
                            Some(BosunRequest::Sample(sample)) => {
                                debug!("Received new sample '{}'.", sample.time);
                                self.queue.push(sample);
                            }
                            Some(BosunRequest::Shutdown) => {
                                debug!("Received message to shut down.");
                                break
                            }
                            None => {
                                error!("Channel unexpectedly shut down.");
                            }
                        }
                    }
                }
            }

            info!("Bosun thread finished.");
        })
    }
}

fn send_sample_to_bosun(mut s: Sample, bosun_client: &BosunClient, hostname: &str, default_tags: &Tags) -> EmitterResult {
    let value = format!("{}", &s.value);
    s.tags.insert("host".to_string(), hostname.to_string());
    s.tags.extend(default_tags.clone());
    let d = Datum {
        metric: &s.metric, timestamp: s.time as i64, value: &value, tags: &s.tags };
    trace!("Sending datum {:?} to Bosun.", &d);
    bosun_client.emit_datum(&d)
}
