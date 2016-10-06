use bosun::Sample;
use config::Config;


pub enum Error {
    Init,
    Collection,
    Shutdown,
}

pub type Id = String;

pub trait Collector {
    fn init(&mut self) -> Result<(), Box<Error>>;
    fn id(&self) -> &Id;
    fn collect(&self) -> Sample;
    fn shutdown(&self);
}

pub fn create_collectors(config: &Config) -> Vec<Box<Collector + Send>> {
    let mut collectors = Vec::new();
    let mut galeras = galera::create_instances(config);
    collectors.append(&mut galeras);

    collectors
}

pub mod galera {
    use mysql as my;
    use std::time::Duration;
    use std::thread;

    use bosun::Sample;
    use super::*;
    use super::super::config::Config;

    #[derive(Debug)]
    #[derive(RustcDecodable)]
    #[allow(non_snake_case)]
    pub struct GaleraConfig {
        pub User: String,
        pub Password: String,
        pub UseSocket: Option<Bool>,
        pub URL: String,
    }

    pub struct Galera {
        id: Id,
        user: Option<String>,
        password: Option<String>,
        url: String,
        pool: Option<my::Pool>,
    }

    pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
        match config.Galera {
            Some(ref config) => {
                let id = format!("galera#{}@{}", config.User, config.URL );
                info!("Created instance of Galera collector: {}", id);

                let collector = Galera {
                    id: id, user: config.User.clone(), password: config.Password.clone(),
                    url: config.URL.clone(), pool: None
                };
                vec![Box::new(collector)]
            }
            None => {
                Vec::new()
            }
        }
    }

    impl From<Galera> for my::Opts {
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

    impl Collector for Galera {
        fn init(&mut self) -> Result<(), Box<Error>> {
            let pool = my::Pool::new(collector_config).unwrap();
            self.pool = Some(pool);
            Ok(())
        }

        fn id(&self) -> &Id {
            &self.id
        }

        fn collect(&self) -> Sample {
            // TODO: make this safe -> if let / match
            let wsrepstates: Vec<WsrepStatus> = query_wsrep_status(&self.pool.unwrap());
            trace!("wsrepstates = {:#?}", wsrepstates);
            let metric_data = wsrepstates.convert_to_metric();
            debug!("metric_data = {:#?}", metric_data);

            // TODO: Send all
            metric_data[0]
        }
        fn shutdown(&self) {}
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

    impl From<WsrepStatus> for Option<Sample> {
        fn from(status: WsrepStatus) -> Self {
            match status.name.as_ref() {
                "wsrep_protocol_version" =>
                    Some(Sample::new(status.name, status.value.parse::<f64>().unwrap()) ),
                _ => None
            }
        }
    }

    fn query_wsrep_status(pool: &my::Pool) -> Vec<WsrepStatus> {
        let wsrepstates: Vec<WsrepStatus> = pool
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
        fn convert_to_metric(self) -> Vec<Sample>;
    }

    impl ConvertToMetric for Vec<WsrepStatus> {
        fn convert_to_metric(self) -> Vec<Sample> {
            self.into_iter()
                .flat_map(|x| Option::<Sample>::from(x))
                .collect()
        }
    }

}
#[cfg(test)]
mod tests {
    use super::galera::WsrepStatus;


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
