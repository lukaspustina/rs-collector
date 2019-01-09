use rs_collector::config::*;
use rs_collector::collectors::mongo::ReadPreference;

use mktemp::Temp;
use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;

#[test]
fn load_rs_collector_config() {
    let rs_collector_toml = r#"
Host = "bosun:8070"
FullHost = false
Hostname = "webserver"

[Tags]
  hostgroup = "webservers"
  domain = "webserver.de"
  hosttype = "baremetal"
"#;
    let temp_file_path = create_temp_config_file_from_string(rs_collector_toml);
    let config = Config::load_from_rs_collector_config(&temp_file_path).unwrap();

    assert_eq!(config.Host, "bosun:8070");
    assert_eq!(config.Hostname, "webserver");
    assert_eq!(config.Tags["hostgroup"], "webservers");
    assert_eq!(config.Tags["domain"], "webserver.de");
    assert_eq!(config.Tags["hosttype"], "baremetal");
    assert_eq!(config.Galera.is_some(), false);
}

#[test]
fn load_rs_collector_config_with_galera_config() {
    let rs_collector_toml = r#"
Host = "bosun:8070"
FullHost = false
Hostname = "webserver"

[Tags]
  hostgroup = "webservers"
  domain = "webserver.de"
  hosttype = "baremetal"

[Galera]
  User = "root"
  Password = "toor"
  Socket = "/var/lib/mysql.sock"
"#;
    let temp_file_path = create_temp_config_file_from_string(rs_collector_toml);
    let config = Config::load_from_rs_collector_config(&temp_file_path).unwrap();

    assert_eq!(config.Host, "bosun:8070");
    assert_eq!(config.Hostname, "webserver");
    assert_eq!(config.Tags["hostgroup"], "webservers");
    assert_eq!(config.Tags["domain"], "webserver.de");
    assert_eq!(config.Tags["hosttype"], "baremetal");
    assert_eq!(config.Galera.is_some(), true);

    let galera = config.Galera.unwrap();
    assert_eq!(galera.User.unwrap(), "root");
    assert_eq!(galera.Password.unwrap(), "toor");
    assert_eq!(galera.Host.is_none(), true);
    assert_eq!(galera.Socket.unwrap(), "/var/lib/mysql.sock");
}

#[test]
fn load_rs_collector_config_with_mongo_config() {
    let rs_collector_toml = r#"
Host = "bosun:8070"
FullHost = false
Hostname = "webserver"

[Tags]
  hostgroup = "webservers"
  domain = "webserver.de"
  hosttype = "baremetal"

[[Mongo]]
  Name = "replicaset01"
  Host = "localhost"
  Port = 27015
  # 'admin' database user
  User = "root"
  Password = "secret"
  # Activates SSL transport encryption and requires a least a CA certificate; currently only supported on Linux
  UseSsl = true
  CaCert = "Path to CA certificate"
  # If a client cert is set, a client cert key file is required
  ClientCert = "certs/my_mongo_client_cert.pem"
  ClientCertKey = "certs/my_mongo_client_cert.key"

[[Mongo]]
  Name = "replicaset02"
  Host = "localhost"
  Port = 27016
  ReadPreference = "Secondary"
"#;
    let temp_file_path = create_temp_config_file_from_string(rs_collector_toml);
    let config = Config::load_from_rs_collector_config(&temp_file_path).unwrap();
    eprintln!("Config: {:#?}", config);

    assert_eq!(config.Host, "bosun:8070");
    assert_eq!(config.Hostname, "webserver");
    assert_eq!(config.Tags["hostgroup"], "webservers");
    assert_eq!(config.Tags["domain"], "webserver.de");
    assert_eq!(config.Tags["hosttype"], "baremetal");
    assert!(config.Mongo.is_some());
    let mongo = &config.Mongo.unwrap(); // Safe
    assert_eq!(mongo.len(), 2);

    let m1 = &mongo[0];
    assert!(m1.ReadPreference.is_none());

    let m2 = &mongo[1];
    assert!(m2.ReadPreference.is_some());
    match &m2.ReadPreference {
        Some(ReadPreference::Secondary) => { },
        x => {
                let msg = format!("Wrong read preference {:?}", x);
                panic!(msg);
        },
    }
}

fn create_temp_config_file_from_string(content: &str) -> PathBuf {
    let temp_file_path = Temp::new_file().unwrap().to_path_buf();
    let mut f = File::create(&temp_file_path).unwrap();
    let _ = f.write_all(content.as_bytes()).unwrap();
    let _ = f.sync_data().unwrap();

    temp_file_path
}
