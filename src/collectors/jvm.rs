use bosun::{Metadata, Rate, Sample, Tags};
use collectors::*;
use config::Config;

use regex::Regex;
use std::collections::HashMap;
use std::process::{Command, Output};
use std::io::Result as IoResult;

static METRIC_NAME_GC: &'static str = "jvm.gc.stats";

#[derive(Debug)]
#[derive(Clone)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct JvmConfig {
    Command: String,
    Name: String,
}

pub struct Jvm {
    id: Id,
    jvms: Vec<JvmConfig>,
    metadata: HashMap<String, Metadata>,
}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    if !config.Jvm.is_empty() {
        let id = "jvm".to_string();
        info!("Created instance of JVM collector: {}", id);

        let metadata = metadata();
        let collector = Jvm { id: id, jvms: config.Jvm.clone(), metadata: metadata };
        vec![Box::new(collector)]
    } else {
        Vec::new()
    }
}

impl Collector for Jvm {
    fn init(&mut self) -> Result<(), Box<Error>> {
        let result = Command::new("/usr/bin/jps").arg("-help").output();
        if let Err(err) = handle_command_output("jps", result) {
            return Err(Box::new(err))
        };
        let result = Command::new("/usr/bin/jstat").arg("-help").output();
        if let Err(err) = handle_command_output("jps", result) {
            return Err(Box::new(err))
        };

        Ok(())
    }

    fn id(&self) -> &Id {
        &self.id
    }

    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let jvm_processes = try!(get_jps());
        let results: Vec<Result<Vec<GcStat>, Error>> = jvm_processes.iter()
            .map(|jp| identify_jvms(&self.jvms, jp))
            .filter(|jvm| jvm.name.is_some())
            .map(|jvm| sample_gc_stats(&jvm))
            .collect();

        let (oks, fails): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
        if !fails.is_empty() {
            for f in fails {
                if let Err(err) = f {
                    warn!("Failed to sample gc stats: {}", err);
                }
            }
        }

        let result = oks.into_iter()
            .flat_map(|ok| ok) // Res -> Vec
            .flat_map(|gcs| gcs.into_iter()) // Vec<Vec> -> Vec
            .filter_map(|gc| gcstat_to_sample(&self.metadata, gc)) // None, Some(GC), None -> Some(Sample)
            .collect();
        trace!("Collected these GC samples: '{:#?}'", result);

        Ok(result)
    }

    fn shutdown(&mut self) {}

    fn metadata(&self) -> Vec<Metadata> {
        let mut metadata = metadata();
        let result = metadata.drain().map(|(_, v)| v).collect();
        result
    }
}

fn metadata() -> HashMap<String, Metadata> {
    let mut metadata: HashMap<String, Metadata> = HashMap::new();
    metadata.insert("S0C".to_string(),
        Metadata::new(format!("{}.survivor_space_0_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "S0C: Current survivor space 1 capacity"));
    metadata.insert("S1C".to_string(),
        Metadata::new(format!("{}.survivor_space_1_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "S1C: Current survivor space 1 capacity"));
    metadata.insert("S0U".to_string(),
        Metadata::new(format!("{}.survivor_space_0_utilization", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "S0U: Survivor space 1 utilization"));
    metadata.insert("S1U".to_string(),
        Metadata::new(format!("{}.survivor_space_1_utilization", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "S1U: Survivor space 1 utilization"));
    metadata.insert("EC".to_string(),
        Metadata::new(format!("{}.current_eden_space_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "EC: Current eden space capacity"));
    metadata.insert("EU".to_string(),
        Metadata::new(format!("{}.eden_space_utilization", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "EU: Eden space utilization"));
    metadata.insert("OC".to_string(),
        Metadata::new(format!("{}.current_old_space_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "OC: Current old capacity"));
    metadata.insert("OU".to_string(),
        Metadata::new(format!("{}.old_space_utilization", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "OU: Old space utilization"));
    metadata.insert("PC".to_string(),
        Metadata::new(format!("{}.current_permanent_space_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "PC: Current permanent space capacity"));
    metadata.insert("PU".to_string(),
        Metadata::new(format!("{}.permanent_space_utilization", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "PU: permanent space utilization"));
    metadata.insert("MC".to_string(),
        Metadata::new(format!("{}.metaspace_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "MC: metaspace capacity"));
    metadata.insert("MU".to_string(),
        Metadata::new(format!("{}.metaspace_utilization", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "MU: metaspace utilization"));
    metadata.insert("CCSC".to_string(),
        Metadata::new(format!("{}.compressed_class_space_capacity", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "CCSC: Compressed class space capacity"));
    metadata.insert("CCSU".to_string(),
        Metadata::new(format!("{}.compressed_class_space_used", METRIC_NAME_GC),
                      Rate::Gauge,
                      "kB",
                      "CCSU: Compressed class space used"));
    metadata.insert("YGC".to_string(),
        Metadata::new(format!("{}.young_generation_gc_events", METRIC_NAME_GC),
                      Rate::Counter,
                      "Event",
                      "YGC: Number of young generation garbage collection events"));
    metadata.insert("YGCT".to_string(),
        Metadata::new(format!("{}.young_generation_gc_time", METRIC_NAME_GC),
                      Rate::Counter,
                      "s",
                      "YGCT: Young generation garbage collection time"));
    metadata.insert("FGC".to_string(),
        Metadata::new(format!("{}.full_gc_events", METRIC_NAME_GC),
                      Rate::Counter,
                      "Event",
                      "FGC: Number of full GC events."));
    metadata.insert("FGCT".to_string(),
        Metadata::new(format!("{}.full_gc_time", METRIC_NAME_GC),
                      Rate::Counter,
                      "s",
                      "FGCT: Full garbage collection time"));
    metadata.insert("GCT".to_string(),
        Metadata::new(format!("{}.total_gc_time", METRIC_NAME_GC),
                      Rate::Counter,
                      "s",
                      "GCT: Total garbage collection time"));

    metadata
}

fn handle_command_output(command: &str, result: IoResult<Output>) -> Result<Output, Error> {
    match result {
        Ok(output) => {
            if output.status.success() {
                debug!("Successfully found and run {}.", command);
                Ok(output)
            } else {
                let exit_code = output.status.code().map_or("<received signal>".to_string(), |i| format!("{}", i));
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let msg = format!("Running {} returned exit code {}: out '{}', err '{}'.", command, exit_code, stdout, stderr);
                debug!("{}", msg);
                Err(Error::InitError(msg))
            }
        },
        Err(err) => {
            let msg = format!("Failed to run {}, because '{}'.", command, err.description());
            debug!("{}", msg);
            Err(Error::InitError(msg))
        },
    }
}

#[derive(Debug)]
struct JvmProcess {
    pid: u16,
    class: String,
    cmdline: String,
}

fn get_jps() -> Result<Vec<JvmProcess>, Error>{
    let result = execute_jps();
    let output = try!(handle_command_output("jps", result));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.is_empty() {
        let msg = format!("Failed to parse jps output");
        trace!("Failed to parse jps output for lines: '{}'", stdout);
        return Err(Error::CollectionError(msg));
    }

    let mut jps = Vec::new();
    for line in lines {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            let msg = format!("Failed to parse jps output; expected 'PID CLASS CMDLINE'");
            trace!("Failed to parse jps output for lines: '{}'", stdout);
            return Err(Error::CollectionError(msg));
        }
        let pid: u16 = try!(cols[0].parse::<u16>());
        let jp = JvmProcess { pid: pid, class: cols[1].to_string(), cmdline: cols[2..].join(" ").to_string() };
        trace!("Found JVM Process '{:?}'", jp);
        jps.push(jp)
    }
    Ok(jps)
}

fn execute_jps() -> IoResult<Output> {
    // TODO: use timeout for execution
    Command::new("/usr/bin/jps").arg("-vl").output()
}

#[derive(Debug)]
struct IdentifiedJvm {
    pid: u16,
    name: Option<String>,
}

fn identify_jvms(jvm_configs: &Vec<JvmConfig>, jp: &JvmProcess) -> IdentifiedJvm {
    // TODO: Use regex set for a single scan
    // TODO: Use func combinators
    let mut name = None;
    for jvm_config in jvm_configs {
        // TODO: Make this safe
        let re = Regex::new(&jvm_config.Command).unwrap();
        if re.is_match(&jp.class) || re.is_match(&jp.cmdline) {
            name = Some(jvm_config.Name.clone());
            break;
        }
    }
    IdentifiedJvm {pid: jp.pid, name: name }
}

#[derive(Debug)]
struct GcStat {
    jvm_name: String,
    name: String,
    value: f64,
}

fn sample_gc_stats(jvm: &IdentifiedJvm) -> Result<Vec<GcStat>, Error> {
    let result = execute_jstat(jvm);
    let output = try!(handle_command_output("jstat", result));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() != 2  {
        let msg = format!("Failed to parse jstat output for pid {}", jvm.pid);
        trace!("Failed to parse jstat out for lines: '{}'", stdout);
        return Err(Error::CollectionError(msg));
    }
    let names: Vec<&str> = lines[0].split_whitespace().collect();
    let values: Vec<&str> = lines[1].split_whitespace().collect();

    let mut gcstats = Vec::new();
    for i in 0..values.len() {
        let name = names[i];
        let value = try!(values[i].parse::<f64>());
        // TODO: Unwrap is safe, but only due to the filter in the main algorithm
        let gcstat = GcStat{ jvm_name: jvm.name.as_ref().unwrap().clone(), name: name.to_string(), value: value };
        trace!("Successfully run gcstat for JVM Process '{:?}': '{:?}'", jvm, gcstat);
        gcstats.push(gcstat);
    }

    Ok(gcstats)
}

fn execute_jstat(jvm: &IdentifiedJvm) -> IoResult<Output> {
    // TODO: use timeout for execution
    let pid = format!("{}", jvm.pid);
    Command::new("/usr/bin/jstat").arg("-gc").arg(pid).output()
}

fn gcstat_to_sample(metadata: &HashMap<String, Metadata>, gcstat: GcStat) -> Option<Sample> {
    let mut tags = Tags::new();
    tags.insert("jvm_name".to_string(), gcstat.jvm_name );
    let metric_name = metadata.get(&gcstat.name).map(|m| m.metric.to_string());
    if let Some(name) = metric_name {
        let sample = Sample::new_with_tags(name, gcstat.value, tags);
        Some(sample)
    } else {
        None
    }
}

