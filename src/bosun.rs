use bosun_emitter::{BosunClient, Datum, Tags};
use bosun_emitter;
use chan::Receiver;
use chan;
use std::thread::JoinHandle;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TICK_INTERVAL_SEC: u64 = 15u64;

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
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Sample { time: now, metric: metric.into(), value: value.into(), tags: Tags::new() }
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
    pub fn new<T: Into<String>>( metric: T, rate: Rate, unit: T, description: T ) -> Self {
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
        let bosun_client = BosunClient::new(host);
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
                        trace!("I've been ticked. Current sample queue length is {:#?}", &self.queue.len());
                        for mut s in self.queue.drain(..) {
                            let value = format!("{}", &s.value);
                            s.tags.insert("host".to_string(), self.hostname.clone());
                            s.tags.extend(self.default_tags.clone());
                            let d = Datum {
                                metric: &s.metric, timestamp: s.time as i64, value: &value, tags: &s.tags };
                            debug!("Sending datum {:?} to Bosun.", &d);
                            match self.bosun_client.emit_datum(&d) {
                                Ok(_) => {},
                                Err(err) => {
                                    error!("Failed to send datum to Bosun, because {:?}", err);
                                }
                                // TODO: We should not just drop the sample / datum; maybe we need to pass it back to
                                // ring buffer
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
