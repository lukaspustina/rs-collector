use bosun::{Metadata, Rate, Sample, Tags};
use collectors::*;
use config::Config;

use get_if_addrs::{get_if_addrs, IfAddr};
use itertools::Itertools;
use std::collections::HashSet;
use std::error::{Error as StdError};
use std::iter::FromIterator;
use std::io;
use std::net::Ipv4Addr;
use std::str::FromStr;


static HAS_IP_METRIC_NAME: &'static str = "os.net.has_ipv4s";
static IPV4_TAG: &'static str = "ipv4";

#[derive(Debug)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct HasIpAddrConfig {
    pub Ipv4: Vec<String>,
}

#[derive(Clone)]
pub struct HasIpAddr {
    id: Id,
    ipv4: Vec<Ipv4Addr>,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    match config.HasIpAddr {
        Some(ref config) => {
            let id = format!("hasipaddr#{}", config.Ipv4.iter().join(","));
            info!("Created instance of HasIpAddr collector: {}", id);

            // TODO: Take care of failure case -- ip addr cannot be converted.
            let ipv4 = config.Ipv4.iter().map(|ip| Ipv4Addr::from_str(&ip).unwrap()).collect();
            let collector = HasIpAddr { id: id, ipv4: ipv4 };
            vec![Box::new(collector)]
        }
        None => {
            Vec::new()
        }
    }
}

impl Collector for HasIpAddr {
    fn init(&mut self) -> Result<(), Box<Error>> {
        Ok(())
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let metric_data = try!(check_for_ip_addrs(&self.ipv4));

        Ok(metric_data)
    }

    fn shutdown(&mut self) {}

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new(HAS_IP_METRIC_NAME,
                          Rate::Gauge,
                          "",
                          "Shows whether a host bound a specified IPv4 address. [0 = No, 1 = Yes]")
        ]
    }
}

fn check_for_ip_addrs(ipv4s: &Vec<Ipv4Addr>) -> Result<Vec<Sample>, Error> {
    let local_addrs = try!(get_if_addrs()).into_iter()
        .flat_map(|i|
            match i.addr {
                IfAddr::V4(iface) => Some(iface.ip),
                _ => None
            });
    let has_ip_addr: HashSet<Ipv4Addr> = HashSet::from_iter(local_addrs);

    let result = ipv4s.iter()
        .map(|ipv4| {
            let mut tags = Tags::new();
            tags.insert(IPV4_TAG.to_string(), ipv4.to_string());
            let value = has_ip_addr.contains(ipv4) as u8;
            Sample::new_with_tags(HAS_IP_METRIC_NAME, value, tags)
        }).collect();

    Ok(result)
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::CollectionError(err.description().to_string())
    }
}
