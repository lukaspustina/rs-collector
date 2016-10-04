use rs_collector::config::*;

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
  URL = "/var/lib/mysql.sock"
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
    assert_eq!(galera.User, "root");
    assert_eq!(galera.Password, "toor");
    assert_eq!(galera.URL, "/var/lib/mysql.sock");
}

fn create_temp_config_file_from_string(content: &str) -> PathBuf {
    let temp_file_path = Temp::new_file().unwrap().to_path_buf();
    let mut f = File::create(&temp_file_path).unwrap();
    let _ = f.write_all(content.as_bytes()).unwrap();
    let _ = f.sync_data().unwrap();

    temp_file_path
}
