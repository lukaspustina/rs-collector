use bosun::Sample;

pub type Id = String;

pub trait Collector<T> where T: Collector<T> + Send + 'static {
    fn init<U: Into<String>>(id: U) -> T;
    fn id(&self) -> &Id;
    fn collect(&self) -> Sample;
    fn shutdown(&self);
}

pub mod my_collector {
    use std::time::Duration;
    use std::thread;

    use bosun::Sample;
    use super::{Collector, Id};

    pub struct MyCollector {
        id: Id,
    }

    impl Collector<MyCollector> for MyCollector {
        fn init<U: Into<String>>(id: U) -> MyCollector {
            MyCollector { id: id.into() }
        }
        fn id(&self) -> &Id {
            &self.id
        }
        fn collect(&self) -> Sample {
            thread::sleep(Duration::from_secs(3));
            Sample { time: 1 }
        }
        fn shutdown(&self) {}
    }
}
