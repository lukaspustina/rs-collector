// See
// * https://docs.mongodb.com/manual/reference/command/replSetGetStatus/
// * https://www.datadoghq.com/blog/monitoring-mongodb-performance-metrics-mmap/
// * https://blog.serverdensity.com/monitor-mongodb/
// * http://blog.mlab.com/2013/03/replication-lag-the-facts-of-life/
//

use bosun::{Metadata, Rate, Sample, Tags};
use collectors::{Collector, Error, Id};
use config::Config;

use chrono::prelude::*;
use mongodb::{Bson, Document, Client, ClientOptions, CommandType, Error as MongodbError, ThreadedClient};
use mongodb::common::{ReadMode, ReadPreference as MongoReadPreference};
use mongodb::db::{ThreadedDatabase};
use std::error::Error as StdError;
use std::f64;

#[derive(Debug)]
#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct MongoConfig {
    pub Name: String,
    pub Host: String,
    pub Port: u16,
    pub User: Option<String>,
    pub Password: Option<String>,
    pub UseSsl: Option<bool>,
    pub CaCert: Option<String>,
    pub ClientCert: Option<String>,
    pub ClientCertKey: Option<String>,
    pub ReadPreference: Option<ReadPreference>,
}

#[derive(Clone, Debug)]
#[derive(Deserialize)]
pub enum ReadPreference {
    Primary,
    PrimaryPreferred,
    Secondary,
    SecondaryPreferred,
    Nearest,
}

impl From<ReadPreference> for ReadMode {
    fn from(rp: ReadPreference) -> ReadMode {
        match rp {
            ReadPreference::Primary => ReadMode::Primary,
            ReadPreference::PrimaryPreferred => ReadMode::PrimaryPreferred,
            ReadPreference::Secondary => ReadMode::Secondary,
            ReadPreference::SecondaryPreferred => ReadMode::SecondaryPreferred,
            ReadPreference::Nearest => ReadMode::Nearest,
        }
    }
}

#[derive(Clone)]
pub struct Mongo {
    id: Id,
    name: String,
    user: Option<String>,
    password: Option<String>,
    use_ssl: bool,
    ca_cert: Option<String>,
    client_cert: Option<String>,
    client_cert_key: Option<String>,
    ip_or_hostname: String,
    port: u16,
    read_preference: Option<ReadPreference>,
    client: Option<Client>,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    let mut collectors: Vec<Box<Collector + Send>> = Vec::new();
    if let Some(ref mongos) = config.Mongo {
        for m in mongos {
            let id = format!("mongo#{}#{}@{}:{}",
                            m.Name, m.User.as_ref().unwrap_or(&"''".to_string()), m.Host, m.Port);
            info!("Created instance of Mongo collector: {}", id);

            let collector = Mongo {
                id: id.clone(), name: m.Name.clone(), user: m.User.clone(), password: m.Password.clone(),
                use_ssl: m.UseSsl.unwrap_or_else(|| false),
                ca_cert: m.CaCert.clone(), client_cert: m.ClientCert.clone(), client_cert_key: m.ClientCertKey.clone(),
                ip_or_hostname: m.Host.clone(), port: m.Port, client: None,
                read_preference: m.ReadPreference.clone(),
            };

            // TODO: This should be handled by the parser, but that requires serde
            if collector.use_ssl && collector.ca_cert.is_none() {
                error!("Failed to create instance of Mongo collector id='{}', because SSL is activated without CA cert", id);
            } else if collector.client_cert.is_some() && collector.client_cert_key.is_none() {
                error!("Failed to create instance of Mongo collector id='{}', because client cert is set without client key", id);
            } else {
                info!("Created instance of Galera collector: {}", id);
                collectors.push(Box::new(collector));
            }
        }
    }
    collectors
}

impl Collector for Mongo {
    fn init(&mut self) -> Result<(), Box<Error>> {
        use std::error::Error;

        // TODO: client seems to be _always_ valid, i.e, when connection is impossible
        let mut options = match (self.ca_cert.as_ref(), self.client_cert.as_ref(), self.client_cert_key.as_ref()) {
            (Some(ref ca_cert), Some(ref client_cert), Some(ref client_cert_key)) => {
                ClientOptions::with_ssl(Some(ca_cert), client_cert, client_cert_key, true)
            },
            (Some(ref ca_cert), None, None) => {
                ClientOptions::with_unauthenticated_ssl(Some(ca_cert),false)
            },
            _ => { ClientOptions::new() }
        };
        options.read_preference = self.read_preference.clone().map(|x| MongoReadPreference::new(x.into(), None));
        debug!("Set read preference for {} to {:?}", self.id, &options.read_preference);
        let result = Client::connect_with_options(&self.ip_or_hostname, self.port, options);
        match result {
            Ok(client) => {
                self.client = Some(client);
                Ok(())
            },
            Err(err) => Err(Box::new(super::Error::InitError(err.description().to_string())))
        }
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let mut metric_data = Vec::new();

        let mut server_status = try!(self.server_status()).into_iter()
            .map(|mut s| {
                s.tags.insert("name".to_string(), self.name.clone());
                s
            });
        metric_data.extend(&mut server_status);

        let mut rs_status = try!(self.rs_status()).into_iter()
            .map(|mut s| {
                s.tags.insert("name".to_string(), self.name.clone());
                s
            });
        metric_data.extend(&mut rs_status);

        debug!("metric_data = {:#?}", metric_data);
        Ok(metric_data)
    }

    fn shutdown(&mut self) {
        self.client = None;
    }

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new( "mongo.connections.current", Rate::Gauge, "", "The number of incoming connections from clients to the database server . This number includes the current shell session. Consider the value of connections.available to add more context to this datum. The value will include all incoming connections including any shell connections or connections from other servers, such as replica set members or mongos instances." ),
            Metadata::new( "mongo.connections.available", Rate::Gauge, "", "The number of unused incoming connections available. Consider this value in combination with the value of connections.current to understand the connection load on the database, and the UNIX ulimit Settings document for more information about system thresholds on available connections." ),
            Metadata::new( "mongo.connections.totalCreated", Rate::Counter, "", "Count of all incoming connections created to the server. This number includes connections that have since closed." ),
            Metadata::new( "mongo.opcounters.insert", Rate::Gauge, "", "The total number of insert operations received since the mongod instance last started." ),
            Metadata::new( "mongo.opcounters.query", Rate::Gauge, "", "The total number of queries received since the mongod instance last started." ),
            Metadata::new( "mongo.opcounters.update", Rate::Gauge, "", "The total number of update operations received since the mongod instance last started." ),
            Metadata::new( "mongo.opcounters.delete", Rate::Gauge, "", "The total number of delete operations since the mongod instance last started." ),
            Metadata::new( "mongo.opcounters.getmore", Rate::Gauge, "", "The total number of “getmore” operations since the mongod instance last started. This counter can be high even if the query count is low. Secondary nodes send getMore operations as part of the replication process." ),
            Metadata::new( "mongo.opcounters.command", Rate::Gauge, "", "The total number of commands issued to the database since the mongod instance last started. opcounters.command counts all commands except the write commands: insert, update, and delete." ),
            Metadata::new( "mongo.replicasets.members.mystate", Rate::Gauge, "",
                "Show the local replica set state: 0 = startup, 1 = primary, 2 = secondary, 3 = recovering, 5 = startup2, 6 = unknown, 7 = arbiter, 8 = down, 9 = rollback, 10 = removed" ),
            Metadata::new( "mongo.replicasets.oplog_lag.min", Rate::Gauge, "ms",
                "Show the min. oplog replication lag between the primary and its secondaries. This value is measured only on the replica set's primary." ),
            Metadata::new( "mongo.replicasets.oplog_lag.avg", Rate::Gauge, "ms",
                "Show the avg. oplog replication lag between the primary and its secondaries. This value is measured only on the replica set's primary." ),
            Metadata::new( "mongo.replicasets.oplog_lag.max", Rate::Gauge, "ms",
                "Show the max. oplog replication lag between the primary and its secondaries. This value is measured only on the replica set's primary." ),
        ]
    }
}

impl Mongo {
    fn server_status(&self) -> Result<Vec<Sample>, Error> {
        let client = self.client.as_ref().unwrap();
        let document = try!(query_server_status(client, &self.user, &self.password));

        /*
         * "version" : <string> => Tag
         * "process" : <"mongod"|"mongos">, => Tag
         * "connections" : {
                "current" : <num>,
                "available" : <num>,
                "totalCreated" : NumberLong(<num>)
                },
         * "opcounters" : {
                "insert" : <num>,
                "query" : <num>,
                "update" : <num>,
                "delete" : <num>,
                "getmore" : <num>,
                "command" : <num>
                },
         */

        let mut tags = Tags::new();
        let key = "version";
        if let Some(&Bson::String(ref s)) = document.get(key) {
            trace!("{}: {}", key, s);
            tags.insert(key.to_string(), s.to_string());
        }
        let key = "process";
        if let Some(&Bson::String(ref s)) = document.get(key) {
            trace!("{}: {}", key, s);
            tags.insert(key.to_string(), s.to_string());
        }

        let mut samples = Vec::new();
        if let Some(&Bson::Document(ref cons)) = document.get("connections") {
            if let Some(&Bson::I32(v)) = cons.get("current") {
                samples.push(
                    Sample::new_with_tags("mongo.connections.current", v, tags.clone())
                );
            }
            if let Some(&Bson::I32(v)) = cons.get("available") {
                samples.push(
                    Sample::new_with_tags("mongo.connections.available", v, tags.clone())
                );
            }
            match cons.get("totalCreated") {
                Some(&Bson::I32(v)) =>
                    samples.push(
                        Sample::new_with_tags("mongo.connections.totalCreated", v, tags.clone())
                    ),
                Some(&Bson::I64(v)) =>
                    samples.push(
                        // TODO: The conversion from i64 to f32 may fail
                        Sample::new_with_tags("mongo.connections.totalCreated", v as f32, tags.clone())
                    ),
                _ => {},
            }
        }
        if let Some(&Bson::Document(ref cons)) = document.get("opcounters") {
            if let Some(&Bson::I32(v)) = cons.get("insert") {
                samples.push(
                    Sample::new_with_tags("mongo.opcounters.insert", v, tags.clone())
                );
            }
            if let Some(&Bson::I32(v)) = cons.get("query") {
                samples.push(
                    Sample::new_with_tags("mongo.opcounters.query", v, tags.clone())
                );
            }
            if let Some(&Bson::I32(v)) = cons.get("update") {
                samples.push(
                    Sample::new_with_tags("mongo.opcounters.update", v, tags.clone())
                );
            }
            if let Some(&Bson::I32(v)) = cons.get("delete") {
                samples.push(
                    Sample::new_with_tags("mongo.opcounters.delete", v, tags.clone())
                );
            }
            if let Some(&Bson::I32(v)) = cons.get("getmore") {
                samples.push(
                    Sample::new_with_tags("mongo.opcounters.getmore", v, tags.clone())
                );
            }
            if let Some(&Bson::I32(v)) = cons.get("command") {
                samples.push(
                    Sample::new_with_tags("mongo.opcounters.command", v, tags.clone())
                );
            }
        }

        Ok(samples)
    }

    #[allow(non_snake_case)]
    fn rs_status(&self) -> Result<Vec<Sample>, Error> {
        let client = self.client.as_ref().unwrap();
        let document = try!(query_rs_status(client, &self.user, &self.password));

        if document.is_empty() {
            debug!("Received empty document, so no values to report");
            return Ok(Vec::new());
        }

        let replicaset: String = if let Some(&Bson::String(ref set)) = document.get("set") {
            trace!("set: {}", set);
            set.to_string()
        } else {
            let msg = format!("Could not determine replica set for {}", self.id);
            return Err(Error::CollectionError(msg));
        };
        let myState: i32 = if let Some(&Bson::I32(myState)) = document.get("myState") {
            trace!("myState: {}", myState);
            myState
        } else {
            let msg = format!("Could not determine myState for {}", self.id);
            return Err(Error::CollectionError(msg));
        };

        let mut tags = Tags::new();
        tags.insert("replicaset".to_string(), replicaset);
        let mut samples = Vec::new();

        samples.push(
            Sample::new_with_tags("mongo.replicasets.members.mystate", myState, tags.clone())
        );

        // if replicaset primary
        if myState == 1 {
            let oplog_lag_result = calculate_oplog_lag(&document);
            match oplog_lag_result {
                Ok((min, avg, max)) => {
                    samples.push(
                        Sample::new_with_tags("mongo.replicasets.oplog_lag.min", min, tags.clone())
                    );
                    samples.push(
                        Sample::new_with_tags("mongo.replicasets.oplog_lag.avg", avg, tags.clone())
                    );
                    samples.push(
                        Sample::new_with_tags("mongo.replicasets.oplog_lag.max", max, tags.clone())
                    );
                },
                Err(err) => {
                    // Don't error out, because we already have sensible information like myState
                    error!("Could not determine oplog_log for {}, because '{}'", self.id, err);
                },
            }
        }

        Ok(samples)
    }
}

fn query_server_status(client: &Client, user: &Option<String>, password: &Option<String>) -> Result<Document, Error> {
    let db = client.db("admin");
    if let (&Some(ref u), &Some(ref pw)) = (user, password) {
        try!(db.auth(u, pw));
    }
    let cmd = doc! { "serverStatus" => 1 };
    let result = try!(db.command(cmd, CommandType::Suppressed, None));
    trace!("Document: {}", result);

    Ok(result)
}

fn query_rs_status(client: &Client, user: &Option<String>, password: &Option<String>) -> Result<Document, Error> {
    let db = client.db("admin");
    if let (&Some(ref u), &Some(ref pw)) = (user, password) {
        try!(db.auth(u, pw));
    }
    let cmd = doc! { "replSetGetStatus" => 1 };
    let doc = match db.command(cmd, CommandType::Suppressed, None) {
        Ok(res) => res,
        // This happens when the replSetGetStatus is unsuccessful, e.g., replication is not enabled
        Err(MongodbError::OperationError(msg)) => {
            debug!("Mongo Operation Error because '{}'. Swalling this error", msg);
            return Ok(doc!{})
        }
        Err(e) => {
            debug!("Mongo Error: {:#?}", &e);
            return Err(e.into())
        }
    };
    trace!("Document: {}", doc);

    trace!("Ok value = {:#?}", doc.get("ok"));
    match doc.get("ok") {
        Some(&Bson::FloatingPoint(v)) if v == 1.0 => Ok(doc),
        // This happens when the replSetGetStatus call is not supported, e.g., on mongos
        Some(&Bson::FloatingPoint(v)) => Ok(doc!{}),
        _ => Err(Error::CollectionError(format!("replSetGetStatus: unexpected result document '{}'", doc)))
    }
}

impl From<MongodbError> for Error {
    fn from(err: MongodbError) -> Self {
        let msg = format!("Failed to execute MongoDB query, because '{}'.", err.description());
        Error::CollectionError(msg)
    }
}

#[allow(non_snake_case)]
fn calculate_oplog_lag(document: &Document) -> Result<(f64, f64, f64), Error> {
    let members = if let Some(&Bson::Array(ref members)) = document.get("members") {
        members
    } else {
        let msg = format!("Cloud not parse members array.");
        return Err(Error::CollectionError(msg))
    };

    let mut primary_date: Option<&DateTime<Utc>> = None;
    let mut secondary_dates: Vec<&DateTime<Utc>> = Vec::new();
    for m in members {
        let member = if let &Bson::Document(ref member) = m {
            member
        } else {
            let msg = format!("Invalid member format.");
            return Err(Error::CollectionError(msg))
        };
        let state = if let Some(&Bson::I32(state)) = member.get("state") {
            state
        } else {
            let msg = format!("Missing 'state' element in member document.");
            return Err(Error::CollectionError(msg))
        };
        let optimeDate = if let Some(&Bson::UtcDatetime(ref optimeDate)) = member.get("optimeDate") {
            optimeDate
        } else {
            let msg = format!("Missing 'optimeDate' element in member document.");
            return Err(Error::CollectionError(msg))
        };

        // Primary date
        if state == 1 {
            primary_date = Some(optimeDate);
        } else {
            secondary_dates.push(optimeDate);
        }
    }
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut avg = 0f64;

    if primary_date.is_none() {
        let msg = format!("No primary found in members array.");
        return Err(Error::CollectionError(msg))
    }

    for d in secondary_dates.iter() {
        let diff = primary_date.unwrap().signed_duration_since(**d).num_milliseconds() as f64;
        min = min.min(diff);
        max = max.max(diff);
        avg += diff;
    }
    avg /= secondary_dates.len() as f64;

    Ok((min, avg, max))
}
