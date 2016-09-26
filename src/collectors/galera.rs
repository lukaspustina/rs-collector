#[macro_use] extern crate mysql;

use mysql as my;


#[derive(Debug)]
struct MetricDatum {
    metric: String,
    value: f64,
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

impl From<WsrepStatus> for Option<MetricDatum> {
    fn from(status: WsrepStatus) -> Self {
        match status.name.as_ref() {
            "wsrep_protocol_version" =>
                Some(MetricDatum { metric: status.name, value: status.value.parse::<f64>().unwrap() }),
            _ => None
        }
    }
}

struct GaleraConfig {
    ip_or_hostname: Option<String>,
    unix_addr: Option<String>,
    prefer_socket: bool,
    username: Option<String>,
    password: Option<String>,
}

impl From<GaleraConfig> for my::Opts {
    fn from(config: GaleraConfig) -> Self {
        let mut optsbuilder: my::OptsBuilder = my::OptsBuilder::new();
        optsbuilder.ip_or_hostname(config.ip_or_hostname)
            .unix_addr(config.unix_addr)
            .prefer_socket(config.prefer_socket)
            .user(config.username)
            .pass(config.password);
        my::Opts::from(optsbuilder)
    }
}

trait Collector<T, S> {
    fn init(config: S) -> T;
    fn run(&self) -> Vec<MetricDatum>;
    fn shutdown(&mut self);
}

struct GaleraCollector {
    pool: my::Pool,
}

fn query_wsrep_status(galera: &GaleraCollector) -> Vec<WsrepStatus> {
    let wsrepstates: Vec<WsrepStatus> = galera.pool
        .prep_exec("SHOW GLOBAL STATUS LIKE 'wsrep_%'", ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (name, value): (String, String) = my::from_row(row);
                    WsrepStatus::new(name, value)
                })
                .collect()
        })
        .unwrap();
    wsrepstates
}

trait ConvertToMetric {
    fn convert_to_metric(self) -> Vec<MetricDatum>;
}

impl ConvertToMetric for Vec<WsrepStatus> {
    fn convert_to_metric(self) -> Vec<MetricDatum> {
        self.into_iter()
            .flat_map(|x| Option::<MetricDatum>::from(x))
            .collect()
    }
}

impl Collector<GaleraCollector, GaleraConfig> for GaleraCollector {
    fn init(collector_config: GaleraConfig) -> GaleraCollector {
        let pool = my::Pool::new(collector_config).unwrap();
        GaleraCollector { pool: pool }
    }

    fn run(&self) -> Vec<MetricDatum> {
        let wsrepstates: Vec<WsrepStatus> = query_wsrep_status(self);
        println!("wsrepstates = {:#?}", wsrepstates);
        let metric_data = wsrepstates.convert_to_metric();
        println!("metric_data = {:#?}", metric_data);

        metric_data
    }

    fn shutdown(&mut self) {}
}


fn main() {
    let config = GaleraConfig {
        ip_or_hostname: None,
        unix_addr: Some("/var/run/mysqld/mysqld.sock".to_string()),
        prefer_socket: true,
        username: Some("".to_string()),
        password: Some("".to_string()),
    };

    println!("Connecting ...");
    let mut galera = GaleraCollector::init(config);
    println!("Connected.");

    println!("Running ...");
    galera.run();
    println!("Run.");

    println!("Shutting down ...");
    galera.shutdown();
    println!("Shut down.");
}


#[cfg(test)]
mod tests {
    use super::WsrepStatus;


    fn generate_test_data() -> Vec<WsrepStatus> {
        vec![WsrepStatus {
                 name: "wsrep_local_state_uuid",
                 value: "5a62afb9-7f4a-11e6-a433-cb070bd9b4be",
             },
             WsrepStatus {
                 name: "wsrep_protocol_version",
                 value: "7",
             },
             WsrepStatus {
                 name: "wsrep_last_committed",
                 value: "15156",
             },
             WsrepStatus {
                 name: "wsrep_replicated",
                 value: "27",
             },
             WsrepStatus {
                 name: "wsrep_replicated_bytes",
                 value: "12308",
             },
             WsrepStatus {
                 name: "wsrep_repl_keys",
                 value: "172",
             },
             WsrepStatus {
                 name: "wsrep_repl_keys_bytes",
                 value: "1997",
             },
             WsrepStatus {
                 name: "wsrep_repl_data_bytes",
                 value: "8583",
             },
             WsrepStatus {
                 name: "wsrep_repl_other_bytes",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_received",
                 value: "14",
             },
             WsrepStatus {
                 name: "wsrep_received_bytes",
                 value: "4893",
             },
             WsrepStatus {
                 name: "wsrep_local_commits",
                 value: "25",
             },
             WsrepStatus {
                 name: "wsrep_local_cert_failures",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_replays",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_send_queue",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_send_queue_max",
                 value: "1",
             },
             WsrepStatus {
                 name: "wsrep_local_send_queue_min",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_send_queue_avg",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_local_recv_queue",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_recv_queue_max",
                 value: "1",
             },
             WsrepStatus {
                 name: "wsrep_local_recv_queue_min",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_recv_queue_avg",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_local_cached_downto",
                 value: "15122",
             },
             WsrepStatus {
                 name: "wsrep_flow_control_paused_ns",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_flow_control_paused",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_flow_control_sent",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_flow_control_recv",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_cert_deps_distance",
                 value: "3.114286",
             },
             WsrepStatus {
                 name: "wsrep_apply_oooe",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_apply_oool",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_apply_window",
                 value: "1.000000",
             },
             WsrepStatus {
                 name: "wsrep_commit_oooe",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_commit_oool",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_commit_window",
                 value: "1.000000",
             },
             WsrepStatus {
                 name: "wsrep_local_state",
                 value: "4",
             },
             WsrepStatus {
                 name: "wsrep_local_state_comment",
                 value: "Synced",
             },
             WsrepStatus {
                 name: "wsrep_cert_index_size",
                 value: "47",
             },
             WsrepStatus {
                 name: "wsrep_cert_bucket_count",
                 value: "58",
             },
             WsrepStatus {
                 name: "wsrep_gcache_pool_size",
                 value: "18605",
             },
             WsrepStatus {
                 name: "wsrep_causal_reads",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_cert_interval",
                 value: "0.000000",
             },
             WsrepStatus {
                 name: "wsrep_incoming_addresses",
                 value: "192.168.205.46:3306,192.168.205.47:3306,192.168.205.48:3306",
             },
             WsrepStatus {
                 name: "wsrep_desync_count",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_evs_delayed",
                 value: "",
             },
             WsrepStatus {
                 name: "wsrep_evs_evict_list",
                 value: "",
             },
             WsrepStatus {
                 name: "wsrep_evs_repl_latency",
                 value: "0/0/0/0/0",
             },
             WsrepStatus {
                 name: "wsrep_evs_state",
                 value: "OPERATIONAL",
             },
             WsrepStatus {
                 name: "wsrep_gcomm_uuid",
                 value: "49329803-8175-11e6-ac89-377dc5eb0077",
             },
             WsrepStatus {
                 name: "wsrep_cluster_conf_id",
                 value: "21",
             },
             WsrepStatus {
                 name: "wsrep_cluster_size",
                 value: "3",
             },
             WsrepStatus {
                 name: "wsrep_cluster_state_uuid",
                 value: "5a62afb9-7f4a-11e6-a433-cb070bd9b4be",
             },
             WsrepStatus {
                 name: "wsrep_cluster_status",
                 value: "Primary",
             },
             WsrepStatus {
                 name: "wsrep_connected",
                 value: "ON",
             },
             WsrepStatus {
                 name: "wsrep_local_bf_aborts",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_local_index",
                 value: "0",
             },
             WsrepStatus {
                 name: "wsrep_provider_name",
                 value: "Galera",
             },
             WsrepStatus {
                 name: "wsrep_provider_vendor",
                 value: "Codership Oy <info@codership.com>",
             },
             WsrepStatus {
                 name: "wsrep_provider_version",
                 value: "3.16(r5c765eb)",
             },
             WsrepStatus {
                 name: "wsrep_ready",
                 value: "ON",
             }]
    }
}
