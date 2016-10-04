use bosun::Sample;

pub type Id = String;

pub trait Collector<T> where T: Collector<T> + Send + 'static {
    fn init<U: Into<String>>(id: U) -> T;
    fn id(&self) -> &Id;
    fn collect(&self) -> Sample;
    fn shutdown(&self);
}

pub mod Galera {
    use std::time::Duration;
    use std::thread;

    use bosun::Sample;
    use super::{Collector, Id};

    pub struct Galera {
        id: Id,
    }

    impl Collector<Galera> for Galera {
        fn init<U: Into<String>>(id: U) -> Galera {
            Galera { id: id.into() }
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
