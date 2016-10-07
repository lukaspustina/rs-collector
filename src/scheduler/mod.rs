use chan::{Sender, Receiver};
use chan;
use chan_signal::Signal;
use chan_signal;

use std::fmt::{Debug, Formatter};
use std::fmt;
use std::thread::JoinHandle;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use collectors::Collector;
use collectors::Id;
use bosun::{Bosun, BosunRequest, Sample};

pub fn run(collectors: Vec<Box<Collector + Send>>) -> () {
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let timer = chan::tick(Duration::from_secs(TICK_INTERVAL));
    info!("Scheduler thread started.");

    let (to_main_tx, from_runners_rx) = chan::async();
    let controllers = create_controllers(collectors, to_main_tx);
    info!("Loaded {} collectors: {:#?}", controllers.len(), controllers);

    let (to_bosun_tx, from_main_rx) = chan::async();
    let bosun = Bosun::new(from_main_rx);
    let bosun_thread = bosun.spawn();

    event_loop(&controllers,
               &signal,
               &timer,
               &from_runners_rx,
               &to_bosun_tx);

    // TODO: Generalize tear_down for all threads / JoinHandles
    tear_down(controllers);
    to_bosun_tx.send(BosunRequest::Shutdown);
    let _ = bosun_thread.join();

    info!("Scheduler thread finished.");
}

static TICK_INTERVAL: u64 = 1u64;

#[derive(Debug)]
enum CollectorRequest {
    Helo,
    Shutdown,
    Sample,
}

#[derive(Debug)]
enum CollectorResponse {
    Id(Id),
    Sample(Sample),
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
    controller_tx: Sender<CollectorResponse>,
    collector: Arc<Mutex<Box<Collector + Send>>>,
}

impl CollectorRunner {
    fn new(id: Id,
           runner_rx: Receiver<CollectorRequest>,
           controller_tx: Sender<CollectorResponse>,
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
                        self.controller_tx.send(CollectorResponse::Id(self.id.clone()));
                    }
                    Some(CollectorRequest::Sample) => {
                        debug!("CollectorRunner {} received 'Sample' message.", &self.id);
                        self.collect_sample();
                    }
                    Some(CollectorRequest::Shutdown) => {
                        debug!("CollectorRunner {} received 'Shutdown' message.", &self.id);
                        let collector = self.collector.clone();
                        let collector = collector.lock().unwrap();
                        collector.shutdown();
                        self.controller_tx.send(CollectorResponse::Id(self.id.clone()));
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }
            info!("CollectorRunner {} thread finished.", self.id);
        })
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
                    let sample = collector.collect();
                    tx.send(CollectorResponse::Sample(sample));
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
    runners_to_main_tx: Sender<CollectorResponse>) -> Vec<CollectorController> {
    let mut controllers: Vec<CollectorController> = Vec::new();

    for c in collectors.into_iter() {
        let (to_runner_tx, from_controller_rx) = chan::async();
        let mut controller = CollectorController::new(c.id().clone(), to_runner_tx);
        let runner = CollectorRunner::new(c.id().clone(),
                                          from_controller_rx,
                                          runners_to_main_tx.clone(),
                                          c);
        let runner_thread = runner.spawn();

        controller.runner_thread = Some(runner_thread);
        controllers.push(controller);
    }

    controllers
}

fn event_loop(threads: &Vec<CollectorController>,
              signal_rx: &Receiver<Signal>,
              timer: &Receiver<Sender<()>>,
              collectors_rx: &Receiver<CollectorResponse>,
              bosun_tx: &Sender<BosunRequest>)
              -> () {
    info!("Scheduler thread entering event loop.");
    loop {
        debug!("Scheduler thread event loop.");
        chan_select! {
            signal_rx.recv() => {
                break
            },
            timer.recv() => {
                trace!("Scheduler: I've been ticked.");
                for cc in threads.iter() {
                    cc.runner_tx.send(CollectorRequest::Sample)
                }
            },
            collectors_rx.recv() -> message => {
                match message {
                    Some(CollectorResponse::Id(id)) => {
                        debug!("Scheduler received 'Helo' from collector {}.", id);
                    }
                    Some(CollectorResponse::Sample(sample)) => {
                        debug!("Scheduler received sample {}.", sample.time);
                        bosun_tx.send(BosunRequest::Sample(sample));
                    }
                    None => {
                        error!("Channel unexpectedly shut down.");
                        break
                    }
                }
            }
        };
    }
}

fn tear_down(threads: Vec<CollectorController>) -> () {
    info!("Scheduler thread shutting down ...");
    for cc in threads.iter() {
        cc.runner_tx.send(CollectorRequest::Shutdown)
    }

    info!("Scheduler thread waiting for collector threads to finish ...");
    for cc in threads.into_iter() {
        let jh = cc.runner_thread.unwrap();
        let _ = jh.join();
    }
}
