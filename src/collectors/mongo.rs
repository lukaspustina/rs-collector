// See
// * https://docs.mongodb.com/manual/reference/command/replSetGetStatus/
// * https://www.datadoghq.com/blog/monitoring-mongodb-performance-metrics-mmap/
// * https://blog.serverdensity.com/monitor-mongodb/
// * http://blog.mlab.com/2013/03/replication-lag-the-facts-of-life/
//

use bosun::{Metadata, Rate, Sample};
use collectors::{Collector, Error, Id};
use config::Config;

use bson::{Bson, Document};
use mongodb::{Client, CommandType, Error as MongodbError, ThreadedClient};
use mongodb::db::{ThreadedDatabase};
use std::error::Error as StdError;
use std::num::ParseIntError;

#[derive(Debug)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct MongoConfig {
    pub Name: String,
    pub User: Option<String>,
    pub Password: Option<String>,
    pub Host: String,
    pub Port: u16,
}

#[derive(Clone)]
pub struct Mongo {
    id: Id,
    name: String,
    user: Option<String>,
    password: Option<String>,
    ip_or_hostname: String,
    port: u16,
    client: Option<Client>,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    let mut names: Vec<Box<Collector + Send>> = Vec::new();
    for m in &config.Mongo {
        let id = format!("mongo#{}#{}@{}:{}",
                         m.Name, m.User.as_ref().unwrap_or(&"''".to_string()), m.Host, m.Port );
        info!("Created name of Mongo collector: {}", id);

        let collector = Mongo {
            id: id, name: m.Name.clone(), user: m.User.clone(), password: m.Password.clone(),
            ip_or_hostname: m.Host.clone(), port: m.Port, client: None,
        };
        names.push(Box::new(collector));
    }
    names
}

impl Collector for Mongo {
    fn init(&mut self) -> Result<(), Box<Error>> {
        use std::error::Error;

        let result = Client::connect(&self.ip_or_hostname, self.port);
        match result {
            Ok(client) => {
                self.client = Some(client);
                Ok(())
            },
            // TODO: Simplify
            Err(err) => Err(Box::new(super::Error::InitError(err.description().to_string())))
        }
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        // TODO: make this safe -> if let / match
        let client = self.client.as_ref().unwrap();
        let mut metric_data = Vec::new();

        let mut rs_status = try!(rs_status(client)).into_iter()
            .map( |mut s| {s.tags.insert("name".to_string(), self.name.clone()); s });
        trace!("rs_status = {:#?}", rs_status);
        metric_data.extend(&mut rs_status);

        debug!("metric_data = {:#?}", metric_data);
        Ok(metric_data)
    }

    fn shutdown(&mut self) {
        self.client = None;
    }

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new( "mongo.replicaset.mystate", Rate::Gauge, "",
                "Show the local ReplicaSet state: 0 = startup, 1 = primary, 2 = secondary, 3 = recovering, 5 = startup2, 6 = unknown, 7 = arbiter, 8 = down, 9 = rollback, 10 = removed" ),
        ]
    }
}

#[allow(non_snake_case)]
fn rs_status(client: &Client) -> Result<Vec<Sample>, Error> {
    let result = try!(query_rs_status(client));

    let mut samples = Vec::new();
    if let Some(&Bson::I32(myState)) = result.get("myState") {
        trace!("myState: {}", myState);
        samples.push(
            Sample::new("mongo.replicaset.mystate", myState)
        );
    }

    Ok(samples)
}

fn query_rs_status(client: &Client) -> Result<Document, Error> {
    let db = client.db("admin");
    let cmd = doc! { "replSetGetStatus" => 1 };
    let result = try!(db.command(cmd, CommandType::Suppressed, None));
    trace!("Document: {}", result);

    Ok(result)
}

impl From<MongodbError> for Error {
    fn from(err: MongodbError) -> Self {
        let msg = format!("Failed to execute MongoDB query, because '{}'.", err.description());
        Error::CollectionError(msg)
    }
}

