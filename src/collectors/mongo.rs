// See
// * https://docs.mongodb.com/manual/reference/command/replSetGetStatus/
// * https://www.datadoghq.com/blog/monitoring-mongodb-performance-metrics-mmap/
// * https://blog.serverdensity.com/monitor-mongodb/
// * http://blog.mlab.com/2013/03/replication-lag-the-facts-of-life/
//

use bosun::{Metadata, Rate, Sample};
use collectors::*;
use config::Config;
use utils;

use bson::Bson;
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;

#[derive(Debug)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct MongoConfig {
    pub User: Option<String>,
    pub Password: Option<String>,
    pub Host: String,
    pub Port: i32,
}

#[derive(Clone)]
pub struct Mongo {
    id: Id,
    user: Option<String>,
    password: Option<String>,
    ip_or_hostname: String,
    port: i32,
    client: Option<Client>,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    let mut instances: Vec<Box<Collector + Send>> = Vec::new();
    for m in &config.Mongo {
        let id = format!("mongo#{}@{}:{}",
                         m.User.as_ref().unwrap_or(&"''".to_string()), m.Host, m.Port );
        info!("Created instance of Mongo collector: {}", id);

        let collector = Mongo {
            id: id, user: m.User.clone(), password: m.Password.clone(),
            ip_or_hostname: m.Host.clone(), port: m.Port, client: None,
        };
        instances.push(Box::new(collector));
    }
    instances
}

impl Collector for Mongo {
    fn init(&mut self) -> Result<(), Box<Error>> {
        use std::error::Error;

        /*
        let galera = self.clone();
        let pool = my::Pool::new(galera);
        match pool {
            Ok(pool) => {
                self.pool = Some(pool);
                Ok(())
            },
            // TODO: Simplify
            Err(err) => Err(Box::new(super::Error::InitError(err.description().to_string())))
        }
        */
        Err(Box::new(super::Error::InitError("Not yet implemented".to_string())))

    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        /*
        // TODO: make this safe -> if let / match
        let wsrepstates: Vec<WsrepStatus> = try!(query_wsrep_status(self.pool.as_ref().unwrap()));
        trace!("wsrepstates = {:#?}", wsrepstates);
        let metric_data = wsrepstates.convert_to_metric();
        debug!("metric_data = {:#?}", metric_data);
        */

        let metric_data = Vec::new();
        Ok(metric_data)
    }

    fn shutdown(&self) {}

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new( "mongo.replicaset.mystate", Rate::Gauge, "",
                "Show the local ReplicaSet state: 0 = startup, 1 = primary, 2 = secondary, 3 = recovering, 5 = startup2, 6 = unknown, 7 = arbiter, 8 = down, 9 = rollback, 10 = removed" ),
        ]
    }
}



