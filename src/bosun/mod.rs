use chan::Receiver;
use chan;

use std::thread::JoinHandle;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TICK_INTERVAL: u64 = 5u64;

#[derive(Debug)]
pub struct Sample {
    pub time: u64,
    pub metric: String,
    pub value: f64,
}

impl Sample {
    pub fn new<T: Into<String>, U: Into<f64>>(metric: T, value: U) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Sample { time: now, metric: metric.into(), value: value.into() }
    }
}

#[derive(Debug)]
pub enum BosunRequest {
    Sample(Sample),
    Shutdown,
}

pub struct Bosun {
    queue: Vec<Sample>,
    from_main_rx: Receiver<BosunRequest>,
}

impl Bosun {
    pub fn new(from_main_rx: Receiver<BosunRequest>) -> Bosun {
        Bosun {
            queue: Vec::new(),
            from_main_rx: from_main_rx,
        }
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        let timer = chan::tick(Duration::from_secs(TICK_INTERVAL));

        thread::spawn(move || {
            info!("Bosun thread started.");

            let from_main_rx = self.from_main_rx;
            loop {
                chan_select! {
                    timer.recv() => {
                        trace!("I've been ticked. Current sample queue length is {:#?}", self.queue.len());
                        debug!("I should send my samples to Bosun but nobody implemented that, yet")
                    },
                    from_main_rx.recv() -> msg => {
                        match msg {
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
