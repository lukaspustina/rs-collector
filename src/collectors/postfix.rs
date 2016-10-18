use bosun::{Metadata, Rate, Sample, Tags};
use collectors::*;
use config::Config;

use itertools::Itertools;
use std::process::{Command, Output};
use std::io::Result as IoResult;

static METRIC_NAME_QUEUES: &'static str = "postfix.queues";
static POSTFIX_QUEUS: &'static [&'static str] = &["maildrop", "incoming", "hold", "active", "deferred"];

#[derive(Debug)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct PostfixConfig {}

#[derive(Clone)]
pub struct Postfix {
    id: Id,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    match config.Postfix {
        Some(_) => {
            let id = "postfix".to_string();
            info!("Created instance of Postfix collector: {}", id);

            let collector = Postfix{ id: id };
            vec![Box::new(collector)]
        },
        None => {
            Vec::new()
        }
    }
}

impl Collector for Postfix {
    fn init(&mut self) -> Result<(), Box<Error>> {
        // TODO: Check if qshape is installed and fail if not
        Ok(())
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let metric_data = try!(sample_queues());

        Ok(metric_data)
    }

    fn shutdown(&self) {}

    fn metadata(&self) -> Vec<Metadata> {
        vec![
            Metadata::new(format!("{}.maildrop", METRIC_NAME_QUEUES),
                          Rate::Gauge,
                          "messages",
                          "local submission directory; bucket tag represents age distribution."),
            Metadata::new(format!("{}.incoming", METRIC_NAME_QUEUES),
                          Rate::Gauge,
                          "messages",
                          "new message queue; bucket tag represents age distribution."),
            Metadata::new(format!("{}.hold", METRIC_NAME_QUEUES),
                          Rate::Gauge,
                          "messages",
                          "messages waiting for tech support; bucket tag represents age distribution."),
            Metadata::new(format!("{}.active", METRIC_NAME_QUEUES),
                          Rate::Gauge,
                          "messages",
                          "messages scheduled for delivery; bucket tag represents age distribution."),
            Metadata::new(format!("{}.deferred", METRIC_NAME_QUEUES),
                          Rate::Gauge,
                          "messages",
                          "messages postponed for later delivery; bucket tag represents age distribution."),
        ]
    }
}

struct QueueLength {
    name: String,
    bucket: String,
    len: i32,
}

fn sample_queues() -> Result<Vec<Sample>, Error> {
    let q_lens: Vec<QueueLength> = POSTFIX_QUEUS.iter().map( |q| get_queue_len(q).unwrap() ).flatten().collect();
    let metric_data: Vec<Sample> = q_lens.convert_to_metric();
    debug!("metric_data = {:#?}", metric_data);

    Ok(metric_data)
}

fn get_queue_len(q_name: &str) -> Result<Vec<QueueLength>, Error> {
    // TODO: 1. Check result code and error! stderr and 2. use timeouts for execution
    let output = execute_qshape_for_queue(q_name).unwrap();
    if !output.status.success() {
        // TODO define me some errors
        error!("Failed to run qshape for queue '{}'", q_name);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().take(2).collect();
    if lines.len() < 2 {
        // TODO define me some errors
        error!("Failed to parse qshape output for queue '{}'", q_name);
    }
    let header: Vec<&str> = lines[0].split_whitespace().collect();
    let totals: Vec<&str> = lines[1].split_whitespace().collect();

    let mut q_lens = Vec::new();
    // TODO: Bug: last column contains '+' which is not a valid opentsdb tag
    for i in 2..totals.len() {
        // TODO: Make sure tag values are valid -- check opentsdb
        let bucket = header[i-1].to_string().replace("+", "p");
        // TODO: try!
        let len = totals[i].parse::<i32>().unwrap();
        q_lens.push(QueueLength{ name: q_name.to_string(), bucket: bucket, len: len });

    }

    Ok(q_lens)
}

fn execute_qshape_for_queue(q_name: &str) -> IoResult<Output> {
    Command::new("/usr/sbin/qshape")
        .arg(q_name)
        .output()
}

impl From<QueueLength> for Option<Sample> {
    fn from(q_len: QueueLength) -> Self {
        let mut tags = Tags::new();
        tags.insert("bucket".to_string(), q_len.bucket);
        let metric_name = format!("{}.{}", METRIC_NAME_QUEUES, q_len.name);
        let sample = Sample::new_with_tags(metric_name, q_len.len, tags);

        Some(sample)
    }
}

trait ConvertToMetric {
    fn convert_to_metric(self) -> Vec<Sample>;
}

impl ConvertToMetric for Vec<QueueLength> {
    fn convert_to_metric(self) -> Vec<Sample> {
        self.into_iter()
            .flat_map(|x| Option::<Sample>::from(x))
            .collect()
    }
}
