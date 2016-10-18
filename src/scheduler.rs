use chan::{Sender, Receiver};
use chan;
use chan_signal::Signal;
use chan_signal;

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::fmt;
use std::thread::JoinHandle;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use Msg;
use config::Config;
use collectors::{Collector, Error};
use collectors::Id;
use bosun::{Bosun, BosunRequest, Metadata, Sample};

pub fn run(collectors: Vec<Box<Collector + Send>>, config: &Config) -> () {
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let timer = chan::tick(Duration::from_secs(TICK_INTERVAL_SEC));
    info!("Scheduler thread started.");

    let (to_main_tx, from_runners_rx) = chan::async();
    let controllers = create_controllers(collectors, to_main_tx);
    info!("Loaded {} collectors: {:#?}", controllers.len(), controllers);

    let (to_bosun_tx, from_main_rx) = chan::async();
    let bosun_thread = if config.DontSend.or(Some(false)).unwrap() {
        None
    } else {
        let bosun = Bosun::new(&config.Host, &config.Hostname, &config.Tags, from_main_rx);
        Some(bosun.spawn())
    };

    event_loop(&controllers,
               &signal,
               &timer,
               &from_runners_rx,
               &to_bosun_tx);

    // TODO: Generalize tear_down for all threads / JoinHandles
    tear_down(controllers);
    to_bosun_tx.send(BosunRequest::Shutdown);
    if let Some(thread) = bosun_thread {
        let _ = thread.join();
    }

    info!("Scheduler thread finished.");
}

static TICK_INTERVAL_SEC: u64 = 15u64;

#[derive(Debug)]
enum CollectorRequest {
    Helo,
    Init,
    Metadata,
    Sample,
    Shutdown,
}

#[derive(Debug)]
enum CollectorResponse {
    Id(Id),
    Metadata(Metadata),
    Sample(Sample),
    CollectionError(Error)
}

/**
* Handle for Main Thread to communicate with CollectorRunner
**/
struct CollectorController {
    id: Id,
    runner_tx: Sender<CollectorRequest>,
    runner_thread: Option<JoinHandle<()>>,
}

impl Debug for CollectorController {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "CollectorController {{ id: {:#?} }}", self.id)
    }
}

impl CollectorController {
    fn new(id: Id, runner_tx: Sender<CollectorRequest>) -> CollectorController {
        CollectorController {
            id: id,
            runner_tx: runner_tx,
            runner_thread: None,
        }
    }
}

/**
* Periodically runs the collector <T>
**/
struct CollectorRunner {
    id: Id,
    runner_rx: Receiver<CollectorRequest>,
    controller_tx: Sender<Msg<CollectorResponse>>,
    collector: Arc<Mutex<Box<Collector + Send>>>,
}

impl CollectorRunner {
    fn new(id: Id,
           runner_rx: Receiver<CollectorRequest>,
           controller_tx: Sender<Msg<CollectorResponse>>,
           collector: Box<Collector + Send>)
           -> CollectorRunner {
        CollectorRunner {
            id: id,
            runner_rx: runner_rx,
            controller_tx: controller_tx,
            collector: Arc::new(Mutex::new(collector)),
        }
    }

    fn spawn(mut self) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("CollectorRunner {} thread started.", self.id);
            loop {
                let message = self.runner_rx.recv();
                match message {
                    Some(CollectorRequest::Helo) => {
                        debug!("CollectorRunner {} received 'Helo' message.", &self.id);
                        self.controller_tx.send(
                            Msg::Collector(self.id.clone(),CollectorResponse::Id(self.id.clone())));
                    },
                    Some(CollectorRequest::Init) => {
                        // TODO: Add failure management exp backoff timer to wait before reconnecting.
                        let exp_backoff = 10;
                        debug!("CollectorRunner {} received 'Init' message. Waiting {} sec.", &self.id, exp_backoff);
                        thread::sleep(Duration::from_secs(exp_backoff));
                        let collector = self.collector.clone();
                        let mut collector = collector.lock().unwrap();
                        match collector.init() {
                            Ok(_) => {
                                info!("CollectorRunner {} successfully re-initialized collector.", &self.id);
                            },
                            Err(_) => {
                                error!("CollectorRunner {} failed to re-initialize collector. Shutting collector down.", &self.id);
                                collector.shutdown();
                                break;
                            }
                        }
                    },
                    Some(CollectorRequest::Metadata) => {
                        debug!("CollectorRunner {} received 'Metadata' message.", &self.id);
                        self.collect_metadata();
                    },
                    Some(CollectorRequest::Sample) => {
                        debug!("CollectorRunner {} received 'Sample' message.", &self.id);
                        self.collect_sample();
                    },
                    Some(CollectorRequest::Shutdown) => {
                        debug!("CollectorRunner {} received 'Shutdown' message.", &self.id);
                        let collector = self.collector.clone();
                        let collector = collector.lock().unwrap();
                        collector.shutdown();
                        self.controller_tx.send(
                            Msg::Collector(self.id.clone(),CollectorResponse::Id(self.id.clone())));
                        break;
                    },
                    None => {
                        break;
                    },
                }
            }
            info!("CollectorRunner {} thread finished.", self.id);
        })
    }

    fn collect_metadata(&mut self) {
        let collector = self.collector.clone();
        let lock = collector.try_lock();
        match lock {
            Ok(_) => {
                let id = self.id.clone();
                let tx = self.controller_tx.clone();
                let collector = self.collector.clone();
                thread::spawn(move || {
                    debug!("CollectorRunner {} spawned metadata thread.", &id);
                    let ref collector = *collector.lock().unwrap();
                    let metadata = collector.metadata();
                    for m in metadata.into_iter() {
                        tx.send(
                            Msg::Collector(id.clone(), CollectorResponse::Metadata(m)));
                    }
                    debug!("CollectorRunner {} finished metadata thread.", &id);
                });
            }
            Err(_) => {
                trace!("CollectorRunner {} metadata already running ...", &self.id);
            }
        }
    }

    fn collect_sample(&mut self) {
        let collector = self.collector.clone();
        let lock = collector.try_lock();
        match lock {
            Ok(_) => {
                let id = self.id.clone();
                let tx = self.controller_tx.clone();
                let collector = self.collector.clone();
                thread::spawn(move || {
                    debug!("CollectorRunner {} spawned sample thread.", &id);
                    let ref collector = *collector.lock().unwrap();
                    match collector.collect() {
                        Ok(samples) => {
                            for s in samples.into_iter() {
                                tx.send(
                                    Msg::Collector(id.clone(), CollectorResponse::Sample(s)));
                            }
                        },
                        Err(error) => {
                            warn!("CollectorResponse {} received collection error {}", &id, error);
                            tx.send(
                                Msg::Collector(id.clone(), CollectorResponse::CollectionError(error)));
                        }

                    }
                    debug!("CollectorRunner {} finished sample thread.", &id);
                });
            }
            Err(_) => {
                trace!("CollectorRunner {} sampling already running ...", &self.id);
            }
        }
    }
}

fn create_controllers(
    collectors: Vec<Box<Collector + Send>>,
    runners_to_main_tx: Sender<Msg<CollectorResponse>>)
    -> HashMap<String, CollectorController> {

    let mut controllers: HashMap<String, CollectorController> = HashMap::new();

    for mut c in collectors.into_iter() {
        // Initialization might be moved to collector threads?
        match c.init() {
            Ok(_) => {
                let (to_runner_tx, from_controller_rx) = chan::async();
                let id = c.id().clone();
                let mut controller = CollectorController::new(id.clone(), to_runner_tx);
                let runner = CollectorRunner::new(id.clone(),
                                                  from_controller_rx,
                                                  runners_to_main_tx.clone(),
                                                  c);
                let runner_thread = runner.spawn();

                controller.runner_thread = Some(runner_thread);
                controllers.insert(id, controller);
            },
            Err(err) => {
                error!("Failed to initialize collector {}: {:?}", c.id(), err);
            }
        }
    }

    controllers
}

fn event_loop(threads: &HashMap<String, CollectorController>,
              signal_rx: &Receiver<Signal>,
              timer: &Receiver<Sender<()>>,
              collectors_rx: &Receiver<Msg<CollectorResponse>>,
              bosun_tx: &Sender<BosunRequest>)
              -> () {
    info!("Scheduler thread entering event loop.");

    // TODO: This should not be here.
    // Transmit metadata once.
    for cc in threads.values() {
        cc.runner_tx.send(CollectorRequest::Metadata)
    }

    loop {
        debug!("Scheduler thread event loop.");
        chan_select! {
            signal_rx.recv() => {
                break
            },
            timer.recv() => {
                trace!("Scheduler: I've been ticked.");
                for cc in threads.values() {
                    cc.runner_tx.send(CollectorRequest::Sample)
                }
            },
            collectors_rx.recv() -> message => {
                match message {
                    Some(Msg::Collector(_, CollectorResponse::Id(id))) => {
                        debug!("Scheduler received 'Helo' from collector {}.", id);
                    }
                    Some(Msg::Collector(id, CollectorResponse::Metadata(metadata))) => {
                        debug!("Scheduler received metadata from '{}' for '{}'.", &id, &metadata.metric );
                        bosun_tx.send(BosunRequest::Metadata(metadata));
                    }
                    Some(Msg::Collector(id, CollectorResponse::Sample(sample))) => {
                        debug!("Scheduler received sample from '{}' for '{}'.", &id, &sample.time );
                        bosun_tx.send(BosunRequest::Sample(sample));
                    },
                    Some(Msg::Collector(id, CollectorResponse::CollectionError(error))) => {
                        debug!("Scheduler received collection error from {} '{}'.", &id, &error);
                        // TODO: Take care of failure case
                        if let Some(cc) = threads.get(&id) {
                            cc.runner_tx.send(CollectorRequest::Init);
                        }
                    },
                    None => {
                        error!("Channel unexpectedly shut down.");
                        break
                    }
                }
            }
        };
    }
}

fn tear_down(mut threads: HashMap<String, CollectorController>) -> () {
    info!("Scheduler thread shutting down ...");
    for cc in threads.values() {
        cc.runner_tx.send(CollectorRequest::Shutdown)
    }

    info!("Scheduler thread waiting for collector threads to finish ...");
    for (_, cc) in threads.drain() {
        let jh = cc.runner_thread.unwrap();
        let _ = jh.join();
    }
}
