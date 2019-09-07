use crate::bosun::{Metadata, Rate, Sample, Tags};
use crate::collectors::*;
use crate::config::Config;

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

pub fn create_instances(config: &Config) -> Vec<Box<dyn Collector + Send>> {
    match config.Postfix {
        Some(_) => {
            let id = "postfix".to_string();
            info!("Created instance of Postfix collector: {}", id);

            let collector = Postfix { id: id };
            vec![Box::new(collector)]
        },
        None => {
            Vec::new()
        }
    }
}

impl Collector for Postfix {
    fn init(&mut self) -> Result<(), Box<Error>> {
        let result = Command::new("/usr/sbin/qshape").output();
        match handle_command_output(result) {
            Ok(_) => Ok(()),
            Err(err) => Err(Box::new(err)),
        }
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let metric_data = r#try!(sample_queues());

        Ok(metric_data)
    }

    fn shutdown(&mut self) {}

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

fn handle_command_output(result: IoResult<Output>) -> Result<Output, Error> {
    match result {
        Ok(output) => {
            if output.status.success() {
                debug!("Successfully found and run qshape.");
                Ok(output)
            } else {
                let exit_code = output.status.code().map_or("<received signal>".to_string(), |i| format!("{}", i));
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let msg = format!("Running qshape returned exit code {}: out '{}', err '{}'.", exit_code, stdout, stderr);
                debug!("{}", msg);
                Err(Error::InitError(msg))
            }
        },
        Err(err) => {
            let msg = format!("Failed to run qshape, because '{}'.", err.description());
            debug!("{}", msg);
            Err(Error::InitError(msg))
        },
    }
}

struct QueueLength {
    name: String,
    bucket: String,
    len: i32,
}

fn sample_queues() -> Result<Vec<Sample>, Error> {
    let mut q_lens: Vec<QueueLength> = Vec::new();
    for q in POSTFIX_QUEUS {
        let mut single_q_lens = r#try!(get_queue_len(q));
        q_lens.append(&mut single_q_lens);
    }
    let metric_data: Vec<Sample> = q_lens.convert_to_metric();
    debug!("metric_data = {:#?}", metric_data);

    Ok(metric_data)
}

fn get_queue_len(q_name: &str) -> Result<Vec<QueueLength>, Error> {
    let result = execute_qshape_for_queue(q_name);
    let output = r#try!(handle_command_output(result));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().take(2).collect();
    if lines.len() < 2 {
        let msg = format!("Failed to parse qshape output for queue '{}'", q_name);
        trace!("Failed to parse qshape out for lines: '{}'", stdout);
        return Err(Error::CollectionError(msg));
    }
    let header: Vec<&str> = lines[0].split_whitespace().collect();
    let totals: Vec<&str> = lines[1].split_whitespace().collect();

    let mut q_lens = Vec::new();
    for i in 2..totals.len() {
        // Last column name ends in '+' which is an invalid char for OpenTSDB tag values:
        let bucket = header[i - 1].to_string().replace("+", "p");
        let len = r#try!(totals[i].parse::<i32>());
        q_lens.push(QueueLength { name: q_name.to_string(), bucket: bucket, len: len });
    }

    Ok(q_lens)
}

fn execute_qshape_for_queue(q_name: &str) -> IoResult<Output> {
    // TODO: use timeout for execution
    Command::new("/usr/sbin/qshape").arg(q_name).output()
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
