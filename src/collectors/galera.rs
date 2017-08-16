// See http://galeracluster.com/documentation-webpages/monitoringthecluster.html

use bosun::{Metadata, Rate, Sample};
use collectors::*;
use config::Config;
use utils;

use mysql as my;
use std::path::PathBuf;

#[derive(Debug)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct GaleraConfig {
    pub User: Option<String>,
    pub Password: Option<String>,
    pub Socket: Option<String>,
    pub Host: Option<String>,
    pub UseSsl: Option<bool>,
    pub CaCert: Option<String>,
    pub ClientCert: Option<String>,
    pub ClientCertKey: Option<String>,
}

#[derive(Clone)]
pub struct Galera {
    id: Id,
    user: Option<String>,
    password: Option<String>,
    socket: Option<String>,
    ip_or_hostname: Option<String>,
    use_ssl: bool,
    ca_cert: Option<PathBuf>,
    client_cert: Option<PathBuf>,
    client_cert_key: Option<PathBuf>,
    pool: Option<my::Pool>,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    match config.Galera {
        Some(ref config) => {
            let id = format!("galera#{}@{}{}",
                             config.User.as_ref().unwrap_or(&"''".to_string()),
                             config.Socket.as_ref().unwrap_or(&"".to_string()),
                             config.Host.as_ref().unwrap_or(&"".to_string()),
                            );

            let collector = Galera {
                id: id.clone(),
                user: config.User.clone(),
                password: config.Password.clone(),
                socket: config.Socket.clone(),
                ip_or_hostname: config.Host.clone(),
                use_ssl: config.UseSsl.unwrap_or_else(|| false),
                ca_cert: config.CaCert.as_ref().map(|s| s.into()),
                client_cert: config.ClientCert.as_ref().map(|s| s.into()),
                client_cert_key: config.ClientCertKey.as_ref().map(|s| s.into()),
                pool: None,
            };

            // TODO: This should be handled by the parser, but that requires serde
            if collector.use_ssl && collector.ca_cert.is_none() {
                error!("Failed to create instance of Galera collector id='{}', because SSL is activated without CA cert", id);
                Vec::new()
            } else if collector.client_cert.is_some() && collector.client_cert_key.is_none() {
                error!("Failed to create instance of Galera collector id='{}', because client cert is set without client key", id);
                Vec::new()
            } else {
                info!("Created instance of Galera collector: {}", id);
                vec![Box::new(collector)]
            }
        }
        None => {
            Vec::new()
        }
    }
}


#[cfg(target_os = "linux")]
impl From<Galera> for my::Opts {
    fn from(config: Galera) -> Self {
        let mut optsbuilder: my::OptsBuilder = my::OptsBuilder::new();
        // prefer_socket is set by default; but we make sure it is set anyway.
        if config.socket.is_some() {
            optsbuilder.prefer_socket(true);
        }
        optsbuilder
            .ip_or_hostname(config.ip_or_hostname)
            .socket(config.socket)
            .user(config.user)
            .pass(config.password);

        if config.use_ssl {
            let ssl_config = match (config.ca_cert, config.client_cert, config.client_cert_key) {
                (Some(ca), Some(client), Some(key)) => Some((ca, Some((client, key)))),
                (Some(ca), _, _) => Some((ca, None)),
                _ => None,
            };
            optsbuilder.ssl_opts(ssl_config);
        }

        my::Opts::from(optsbuilder)
    }
}

#[cfg(not(target_os = "linux"))]
impl From<Galera> for my::Opts {
    fn from(config: Galera) -> Self {
        let mut optsbuilder: my::OptsBuilder = my::OptsBuilder::new();
        // prefer_socket is set by default; but we make sure it is set anyway.
        if config.socket.is_some() {
            optsbuilder.prefer_socket(true);
        }
        optsbuilder
            .ip_or_hostname(config.ip_or_hostname)
            .socket(config.socket)
            .user(config.user)
            .pass(config.password);

        my::Opts::from(optsbuilder)
    }
}

impl Collector for Galera {
    fn init(&mut self) -> Result<(), Box<Error>> {
        use std::error::Error;

        let galera = self.clone();
        if galera.use_ssl {
            info!("Using SSL for instance of Galera collector: {}", self.id);
        }
        let pool = my::Pool::new(galera);
        match pool {
            Ok(pool) => {
                self.pool = Some(pool);
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
        let wsrepstates: Vec<WsrepStatus> = try!(query_wsrep_status(self.pool.as_ref().unwrap()));
        trace!("wsrepstates = {:#?}", wsrepstates);
        let metric_data = wsrepstates.convert_to_metric();
        debug!("metric_data = {:#?}", metric_data);

        Ok(metric_data)
    }

    fn shutdown(&mut self) {
        self.pool = None;
    }

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new( "galera.wsrep.local.state.uuid", Rate::Gauge, "",
                "Shows the cluster state UUID, which you can use to determine whether the node is part of the cluster." ),
            Metadata::new( "galera.wsrep.protocol.version", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.last.committed", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.replicated", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.replicated.bytes", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.repl.keys", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.repl.keys.bytes", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.repl.data.bytes", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.repl.other.bytes", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.received", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.received.bytes", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.local.commits", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.local.cert.failures", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.local.replays", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.local.send.queue", Rate::Gauge, "Length",
                "Show the send queue length." ),
            Metadata::new( "galera.wsrep.local.send.queue.max", Rate::Gauge, "Length",
                "Show the max queue length since the last status query." ),
            Metadata::new( "galera.wsrep.local.send.queue.min", Rate::Gauge, "Length",
               "Show the min queue length since the last status query." ),
            Metadata::new( "galera.wsrep.local.send.queue.avg", Rate::Gauge, "Length",
                "Show an average for the send queue length since the last status query." ),
            Metadata::new( "galera.wsrep.local.recv.queue", Rate::Gauge, "Length",
                "Shows the size of the local received queue since the last status query." ),
            Metadata::new( "galera.wsrep.local.recv.queue.max", Rate::Gauge, "Length",
                "Shows the max size of the local received queue since the last status query." ),
            Metadata::new( "galera.wsrep.local.recv.queue.min", Rate::Gauge, "Length",
                "Shows the min size of the local received queue since the last status query." ),
            Metadata::new( "galera.wsrep.local.recv.queue.avg", Rate::Gauge, "Length",
                "Shows the average size of the local received queue since the last status query." ),
            Metadata::new( "galera.wsrep.local.cached.downto", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.flow.control.paused.ns", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.flow.control.paused", Rate::Gauge, "Fraction",
                "Shows the fraction of the time, since the status variable was last called, that the node paused due to Flow Control." ),
            Metadata::new( "galera.wsrep.flow.control.sent", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.flow.control.recv", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.cert.deps.distance", Rate::Gauge, "Distance",
                "Shows the average distance between the lowest and highest sequence number, or seqno, values that the node can possibly apply in parallel." ),
            Metadata::new( "galera.wsrep.apply.oooe", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.apply.oool", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.apply.window", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.commit.oooe", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.commit.oool", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.commit.window", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.local.state", Rate::Gauge, "State",
                "Shows the node state; the desired state is 'synced'. [1 = Joining (requesting/receiving State Transfer) - node is joining the cluster, 2 = Donor/Desynced - node is the donor to the node joining the cluster, 3 = Joined - node has joined the cluster, 4 = Synced - node is synced with the cluster]" ),
            Metadata::new( "galera.wsrep.cert.index.size", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.cert.bucket.count", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.gcache.pool.size", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.causal.reads", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.cert.interval", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.desync.count", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.evs.state", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.gcomm.uuid", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.cluster.conf.id", Rate::Gauge, "changes",
                "Shows the total number of cluster changes that have happened, which you can use to determine whether or not the node is a part of the Primary Component." ),
            Metadata::new( "galera.wsrep.cluster.size", Rate::Gauge, "nodes",
                "Shows the number of nodes in the cluster, which you can use to determine if any are missing." ),
            Metadata::new( "galera.wsrep.cluster.state.uuid", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.cluster.status", Rate::Gauge, "Status",
                "Shows the primary status of the cluster component that the node is in, which you can use in determining whether your cluster is experiencing a partition. Possible values are [0 = Primary, 1 = Node is part of a nonoperational component.]" ),
            Metadata::new( "galera.wsrep.connected", Rate::Gauge, "",
                "Shows whether the node has network connectivity with any other nodes. [0 = On, 1 = Off]" ),
            Metadata::new( "galera.wsrep.local.bf.aborts", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.local.index", Rate::Gauge, "", "" ),
            Metadata::new( "galera.wsrep.ready", Rate::Gauge, "",
                "Shows whether the node can accept queries. [0 = On, 1 = Off]" ),
        ]
    }
}


#[derive(Debug)]
struct WsrepStatus {
    name: String,
    value: String,
}

impl WsrepStatus {
    pub fn new<T: Into<String>>(name: T, value: T) -> WsrepStatus {
        WsrepStatus {
            name: name.into(),
            value: value.into(),
        }
    }
}

fn value_number_to_f64(name: &str, value: &str) -> Option<Sample> {
    match value.parse::<f64>() {
        Ok(value) => {
            let metric_name = name_to_metric(name);
            Some(Sample::new(metric_name, value))
        },
        Err(err) => {
            error!("Failed to parse '{}' to decimal, because {}", &name, err);
            None
        }
    }
}

fn value_uuid_to_decimal(name: &str, value: &str) -> Option<Sample> {
    match utils::uuid_to_decimal(value) {
        Ok(decimal) => {
            let metric_name = name_to_metric(&name);
            Some(Sample::new(metric_name, decimal as f64))
        },
        Err(err) => {
            error!("Failed to parse '{}' to decimal, because {}", name, err);
            None
        }
    }
}

fn name_to_metric(name: &str) -> String {
    format!("galera.{}", &name.replace("_", "."))
}

impl From<WsrepStatus> for Option<Sample> {
    fn from(status: WsrepStatus) -> Self {
        match status.name.as_ref() {
            name @ "wsrep_local_state_uuid" => {
                value_uuid_to_decimal(&name, &status.value)
            },
            name @ "wsrep_protocol_version" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_last_committed" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_replicated" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_replicated_bytes" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_repl_keys" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_repl_keys_bytes" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_repl_data_bytes" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_repl_other_bytes" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_received" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_received_bytes" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_commits" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_cert_failures" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_replays" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_send_queue" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_send_queue_max" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_send_queue_min" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_send_queue_avg" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_recv_queue" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_recv_queue_max" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_recv_queue_min" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_recv_queue_avg" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_cached_downto" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_flow_control_paused_ns" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_flow_control_paused" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_flow_control_sent" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_flow_control_recv" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_cert_deps_distance" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_apply_oooe" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_apply_oool" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_apply_window" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_commit_oooe" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_commit_oool" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_commit_window" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_state" => {
                value_number_to_f64(&name, &status.value)
            },
            // Ignore: Just the human readable version of 'wsrep_local_state'
            "wsrep_local_state_comment" => { None },
            name @ "wsrep_cert_index_size" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_cert_bucket_count" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_gcache_pool_size" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_causal_reads" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_cert_interval" => {
                value_number_to_f64(&name, &status.value)
            },
            // Ignore: IP addresses of cluster memberes
            "wsrep_incoming_addresses" => { None },
            name @ "wsrep_desync_count" => {
                value_number_to_f64(&name, &status.value)
            },
            // Ignore: Was "" in example ?!
            "wsrep_evs_delayed" => { None },
            // Ignore: Was "" in example ?!
            "wsrep_evs_evict_list" => { None },
            // Ignore: unclear
            "wsrep_evs_repl_latency" => { None },
            name @ "wsrep_evs_state" => {
                // TODO: Handle other states but only primary
                let value = match status.value.clone().to_lowercase().as_ref() {
                    "operational" => 0,
                    _ => 1,
                };
                let metric_name = name_to_metric(&name);
                Some(Sample::new(metric_name, value))
            }
            name @ "wsrep_gcomm_uuid" => {
                value_uuid_to_decimal(&name, &status.value)
            },
            name @ "wsrep_cluster_conf_id" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_cluster_size" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_cluster_state_uuid" => {
                value_uuid_to_decimal(&name, &status.value)
            },
            name @ "wsrep_cluster_status" => {
                // TODO: Handle other states but only primary
                let value = match status.value.clone().to_lowercase().as_ref() {
                    "primary" => 0,
                    _ => 1,
                };
                let metric_name = name_to_metric(&name);
                Some(Sample::new(metric_name, value))
            },
            name @ "wsrep_connected" => {
                let metric_name = name_to_metric(&name);
                match status.value.clone().to_lowercase().as_ref() {
                    "on" => {
                        Some(Sample::new(metric_name, 0))
                    },
                    "off" => {
                        Some(Sample::new(metric_name, 1))
                    },
                    value => {
                        error! ("Failed to parse 'wsrep_connected', because '{}' is an unexpected value.", value);
                        None
                    }
                }
            },
            name @ "wsrep_local_bf_aborts" => {
                value_number_to_f64(&name, &status.value)
            },
            name @ "wsrep_local_index" => {
                value_number_to_f64(&name, &status.value)
            },
            // Ignore
            "wsrep_provider_name" => { None },
            // Ignore
            "wsrep_provider_vendor" => { None },
            // Ignore
            "wsrep_provider_version" => { None },
            name @ "wsrep_ready" => {
                let metric_name = name_to_metric(&name);
                match status.value.clone().to_lowercase().as_ref() {
                    "on" => {
                        Some(Sample::new(metric_name, 0))
                    },
                    "off" => {
                        Some(Sample::new(metric_name, 1))
                    },
                    value => {
                        error! ("Failed to parse 'wsrep_ready', because '{}' is an unexpected value.", value);
                        None
                    }
                }
            },
            x => {
                warn! ("Galera collector found new wsrep status '{}'.", x);
                None
            }
        }
    }
}

fn query_wsrep_status(pool: &my::Pool) -> Result<Vec<WsrepStatus>, Error> {
    let res = pool.prep_exec("SHOW GLOBAL STATUS LIKE 'wsrep_%'", ());
    match res {
        Ok(result) => {
            let wsrep_states = result.map(|x| x.unwrap())
                .map(|row| {
                    let (name, value): (String, String) = my::from_row(row);
                    WsrepStatus::new(name, value)
                })
                .collect();
            Ok(wsrep_states)
        },
        Err(error) => {
            warn!("Failed to query Galera status, because {}", &error);
            Err(Error::CollectionError(format!("{}", error)))
        }
    }
}

trait ConvertToMetric {
    fn convert_to_metric(self) -> Vec<Sample>;
}

impl ConvertToMetric for Vec<WsrepStatus> {
    fn convert_to_metric(self) -> Vec<Sample> {
        self.into_iter()
            .flat_map(|x| Option::<Sample>::from(x))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{WsrepStatus, ConvertToMetric};
    use bosun::Sample;

    impl PartialEq for Sample {
        fn eq(&self, other: &Sample) -> bool {
            if self.metric == other.metric &&
                self.value == other.value {
                return true
            }
            false
        }
    }

    #[test]
    fn convert_correct_test_data() -> () {
        let test_data = generate_test_data();

        let metric_data = test_data.convert_to_metric();

        assert_eq!(metric_data, vec ! [
            Sample::new( "galera.wsrep.local.state.uuid", 223231124026558f64 ),
            Sample::new( "galera.wsrep.protocol.version", 7 ),
            Sample::new( "galera.wsrep.last.committed", 15156 ),
            Sample::new( "galera.wsrep.replicated", 27 ),
            Sample::new( "galera.wsrep.replicated.bytes", 12308 ),
            Sample::new( "galera.wsrep.repl.keys", 172 ),
            Sample::new( "galera.wsrep.repl.keys.bytes", 1997 ),
            Sample::new( "galera.wsrep.repl.data.bytes", 8583 ),
            Sample::new( "galera.wsrep.repl.other.bytes", 0 ),
            Sample::new( "galera.wsrep.received", 14 ),
            Sample::new( "galera.wsrep.received.bytes", 4893 ),
            Sample::new( "galera.wsrep.local.commits", 25 ),
            Sample::new( "galera.wsrep.local.cert.failures", 0 ),
            Sample::new( "galera.wsrep.local.replays", 0 ),
            Sample::new( "galera.wsrep.local.send.queue", 0 ),
            Sample::new( "galera.wsrep.local.send.queue.max", 1 ),
            Sample::new( "galera.wsrep.local.send.queue.min", 0 ),
            Sample::new( "galera.wsrep.local.send.queue.avg", 0 ),
            Sample::new( "galera.wsrep.local.recv.queue", 0 ),
            Sample::new( "galera.wsrep.local.recv.queue.max", 1 ),
            Sample::new( "galera.wsrep.local.recv.queue.min", 0 ),
            Sample::new( "galera.wsrep.local.recv.queue.avg", 0 ),
            Sample::new( "galera.wsrep.local.cached.downto", 15122 ),
            Sample::new( "galera.wsrep.flow.control.paused.ns", 0 ),
            Sample::new( "galera.wsrep.flow.control.paused", 0 ),
            Sample::new( "galera.wsrep.flow.control.sent", 0 ),
            Sample::new( "galera.wsrep.flow.control.recv", 0 ),
            Sample::new( "galera.wsrep.cert.deps.distance", 3.114286 ),
            Sample::new( "galera.wsrep.apply.oooe", 0 ),
            Sample::new( "galera.wsrep.apply.oool", 0 ),
            Sample::new( "galera.wsrep.apply.window", 1 ),
            Sample::new( "galera.wsrep.commit.oooe", 0 ),
            Sample::new( "galera.wsrep.commit.oool", 0 ),
            Sample::new( "galera.wsrep.commit.window", 1 ),
            Sample::new( "galera.wsrep.local.state", 4 ),
            Sample::new( "galera.wsrep.cert.index.size", 47 ),
            Sample::new( "galera.wsrep.cert.bucket.count", 58 ),
            Sample::new( "galera.wsrep.gcache.pool.size", 18605 ),
            Sample::new( "galera.wsrep.causal.reads", 0 ),
            Sample::new( "galera.wsrep.cert.interval", 0 ),
            Sample::new( "galera.wsrep.desync.count", 0 ),
            Sample::new( "galera.wsrep.evs.state", 0 ),
            Sample::new( "galera.wsrep.gcomm.uuid", 61013330952311f64 ),
            Sample::new( "galera.wsrep.cluster.conf.id", 21 ),
            Sample::new( "galera.wsrep.cluster.size", 3 ),
            Sample::new( "galera.wsrep.cluster.state.uuid", 223231124026558f64 ),
            Sample::new( "galera.wsrep.cluster.status", 0 ),
            Sample::new( "galera.wsrep.connected", 0 ),
            Sample::new( "galera.wsrep.local.bf.aborts", 0 ),
            Sample::new( "galera.wsrep.local.index", 0 ),
            Sample::new( "galera.wsrep.ready", 0 )
        ]);
    }

    fn generate_test_data() -> Vec<WsrepStatus> {
        vec![
            WsrepStatus::new( "wsrep_local_state_uuid", "5a62afb9-7f4a-11e6-a433-cb070bd9b4be" ),
            WsrepStatus::new( "wsrep_protocol_version", "7" ),
            WsrepStatus::new( "wsrep_last_committed", "15156" ),
            WsrepStatus::new( "wsrep_replicated", "27" ),
            WsrepStatus::new( "wsrep_replicated_bytes", "12308" ),
            WsrepStatus::new( "wsrep_repl_keys", "172" ),
            WsrepStatus::new( "wsrep_repl_keys_bytes", "1997" ),
            WsrepStatus::new( "wsrep_repl_data_bytes", "8583" ),
            WsrepStatus::new( "wsrep_repl_other_bytes", "0" ),
            WsrepStatus::new( "wsrep_received", "14" ),
            WsrepStatus::new( "wsrep_received_bytes", "4893" ),
            WsrepStatus::new( "wsrep_local_commits", "25" ),
            WsrepStatus::new( "wsrep_local_cert_failures", "0" ),
            WsrepStatus::new( "wsrep_local_replays", "0" ),
            WsrepStatus::new( "wsrep_local_send_queue", "0" ),
            WsrepStatus::new( "wsrep_local_send_queue_max", "1" ),
            WsrepStatus::new( "wsrep_local_send_queue_min", "0" ),
            WsrepStatus::new( "wsrep_local_send_queue_avg", "0.000000" ),
            WsrepStatus::new( "wsrep_local_recv_queue", "0" ),
            WsrepStatus::new( "wsrep_local_recv_queue_max", "1" ),
            WsrepStatus::new( "wsrep_local_recv_queue_min", "0" ),
            WsrepStatus::new( "wsrep_local_recv_queue_avg", "0.000000" ),
            WsrepStatus::new( "wsrep_local_cached_downto", "15122" ),
            WsrepStatus::new( "wsrep_flow_control_paused_ns", "0" ),
            WsrepStatus::new( "wsrep_flow_control_paused", "0.000000" ),
            WsrepStatus::new( "wsrep_flow_control_sent", "0" ),
            WsrepStatus::new( "wsrep_flow_control_recv", "0" ),
            WsrepStatus::new( "wsrep_cert_deps_distance", "3.114286" ),
            WsrepStatus::new( "wsrep_apply_oooe", "0.000000" ),
            WsrepStatus::new( "wsrep_apply_oool", "0.000000" ),
            WsrepStatus::new( "wsrep_apply_window", "1.000000" ),
            WsrepStatus::new( "wsrep_commit_oooe", "0.000000" ),
            WsrepStatus::new( "wsrep_commit_oool", "0.000000" ),
            WsrepStatus::new( "wsrep_commit_window", "1.000000" ),
            WsrepStatus::new( "wsrep_local_state", "4" ),
            WsrepStatus::new( "wsrep_local_state_comment", "Synced" ),
            WsrepStatus::new( "wsrep_cert_index_size", "47" ),
            WsrepStatus::new( "wsrep_cert_bucket_count", "58" ),
            WsrepStatus::new( "wsrep_gcache_pool_size", "18605" ),
            WsrepStatus::new( "wsrep_causal_reads", "0" ),
            WsrepStatus::new( "wsrep_cert_interval", "0.000000" ),
            WsrepStatus::new( "wsrep_incoming_addresses", "192.168.205.46:3306,192.168.205.47:3306,192.168.205.48:3306" ),
            WsrepStatus::new( "wsrep_desync_count", "0" ),
            WsrepStatus::new( "wsrep_evs_delayed", "" ),
            WsrepStatus::new( "wsrep_evs_evict_list", "" ),
            WsrepStatus::new( "wsrep_evs_repl_latency", "0/0/0/0/0" ),
            WsrepStatus::new( "wsrep_evs_state", "OPERATIONAL" ),
            WsrepStatus::new( "wsrep_gcomm_uuid", "49329803-8175-11e6-ac89-377dc5eb0077" ),
            WsrepStatus::new( "wsrep_cluster_conf_id", "21" ),
            WsrepStatus::new( "wsrep_cluster_size", "3" ),
            WsrepStatus::new( "wsrep_cluster_state_uuid", "5a62afb9-7f4a-11e6-a433-cb070bd9b4be" ),
            WsrepStatus::new( "wsrep_cluster_status", "Primary" ),
            WsrepStatus::new( "wsrep_connected", "ON" ),
            WsrepStatus::new( "wsrep_local_bf_aborts", "0" ),
            WsrepStatus::new( "wsrep_local_index", "0" ),
            WsrepStatus::new( "wsrep_provider_name", "Galera" ),
            WsrepStatus::new( "wsrep_provider_vendor", "Codership Oy <info@codership.com>" ),
            WsrepStatus::new( "wsrep_provider_version", "3.16(r5c765eb )" ),
            WsrepStatus::new( "wsrep_ready", "ON" )
        ]
    }
}

