use bosun::{Metadata, Rate, Sample, Tags};
use collectors::*;
use config::Config;

use regex::Regex;
use std::collections::HashMap;
use std::process::{Command, Output};
use std::io::Result as IoResult;
use itertools::Itertools;

static METRIC_NAME_HWDISK: &'static str = "hw.disk";


pub struct Megaraid {
    id: Id,
    metadata: HashMap<String, Metadata>,
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct MegaraidConfig {}

pub fn create_instances(config: &Config) -> Vec<Box<Collector + Send>> {
    if let Some(ref cfg) = config.Megaraid {
        info!("Created instance of Megaraid collector");
        let id = format!("megaraid#{}", "0");
        let metadata = metadata();
        let collector = Megaraid { id: id, metadata: metadata };
        vec![Box::new(collector)]
    } else {
        Vec::new()
    }
}

impl Collector for Megaraid {
    fn init(&mut self) -> Result<(), Box<Error>> {
        // MegaCli64 prüfen
        let result = Command::new("/bin/cat").arg("/etc/passwd").output();
        if let Err(err) = handle_command_output("MegaCli64", result) {
            return Err(Box::new(err));
        };

        Ok(())
    }

    fn id(&self) -> &Id { &self.id }

    fn metadata(&self) -> Vec<Metadata> {
        let mut metadata = metadata();
        let result = metadata.drain().map(|(_, v)| v).collect();
        result
    }

    #[allow(unstable_name_collision)]
    fn collect(&self) -> Result<Vec<Sample>, Error> {
        let pdinfos = try!(get_ldpdinfo());

        let results: Vec<Vec<Sample>> =
            pdinfos.into_iter()
                .map(|pdinfo| pdinfo_to_samples(&self.metadata, pdinfo))
                .collect();


//
//        let (oks, fails): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
//        if !fails.is_empty() {
//            for f in fails {
//                if let Err(err) = f {
//                    warn!("Failed to parse MegaCli64 stats: {}", err);
//                }
//            }
//        }

        let result = results.into_iter().flatten().collect();
        trace!("Collected these Megaraid samples: '{:#?}'", result);

        Ok(result)
    }

    fn shutdown(&mut self) {}

}

fn metadata() -> HashMap<String, Metadata> {
    let mut metadata: HashMap<String, Metadata> = HashMap::new();
    metadata.insert("mediaerrors".to_string(),
                    Metadata::new(format!("{}.mediaerrors", METRIC_NAME_HWDISK),
                                  Rate::Gauge,
                                  "None",
                                  "Reported media error count"));

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
        }
        Err(err) => {
            let msg = format!("Failed to run {}, because '{}'.", command, err.description());
            debug!("{}", msg);
            Err(Error::InitError(msg))
        }
    }
}

#[derive(Debug)]
struct InquiryData {
    manuf: String,
    model: String,
    serial: String,
}

#[derive(Debug)]
struct PdInfo {
    enclosure_id: Option<u8>,
    slot_number: Option<u8>,
    media_errors: Option<u32>,
    other_errors: Option<u32>,
    predictive_failure_errors: Option<u32>,
    last_predictive_failure_event_seqno: Option<u32>,
    smart_flag: Option<bool>,
    manufacturer: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    firmware_state: Option<u8>,
}

#[derive(Debug)]
struct PdInfoBuilder {
    enclosure_id: Option<u8>,
    slot_number: Option<u8>,
    media_errors: Option<u32>,
    other_errors: Option<u32>,
    predictive_failure_errors: Option<u32>,
    last_predictive_failure_event_seqno: Option<u32>,
    smart_flag: Option<bool>,
    manufacturer: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    firmware_state: Option<u8>,
}

impl PdInfoBuilder {
    fn enclosure_id(mut self, enclosure_id: u8) -> Self {
        self.enclosure_id = Some(enclosure_id);
        self
    }
    fn slot_number(mut self, slot_number: u8) -> Self {
        self.slot_number = Some(slot_number);
        self
    }
    fn media_errors(mut self, media_errors: u32) -> Self {
        self.media_errors = Some(media_errors);
        self
    }
    fn other_errors(mut self, other_errors: u32) -> Self {
        self.other_errors = Some(other_errors);
        self
    }
    fn predictive_failure_errors(mut self, predictive_failure_errors: u32) -> Self {
        self.predictive_failure_errors = Some(predictive_failure_errors);
        self
    }
    fn last_predictive_failure_event_seqno(mut self, last_predictive_failure_event_seqno: u32) -> Self {
        self.last_predictive_failure_event_seqno = Some(last_predictive_failure_event_seqno);
        self
    }
    fn smart_flag(mut self, smart_flag: bool) -> Self {
        self.smart_flag = Some(smart_flag);
        self
    }
    fn manufacturer(mut self, manufacturer: String) -> Self {
        self.manufacturer = Some(manufacturer);
        self
    }
    fn model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }
    fn serial_number(mut self, serial_number: String) -> Self {
        self.serial_number = Some(serial_number);
        self
    }
    fn firmware_state(mut self, firmware_state: u8) -> Self {
        self.firmware_state = Some(firmware_state);
        self
    }
    fn build(self) -> PdInfo {
        PdInfo {
            enclosure_id: self.enclosure_id,
            slot_number: self.slot_number,
            media_errors: self.media_errors,
            other_errors: self.other_errors,
            predictive_failure_errors: self.predictive_failure_errors,
            last_predictive_failure_event_seqno: self.last_predictive_failure_event_seqno,
            smart_flag: self.smart_flag,
            manufacturer: self.manufacturer,
            model: self.model,
            serial_number: self.serial_number,
            firmware_state: self.firmware_state,
        }
    }
    fn new() -> Self {
        PdInfoBuilder {
            enclosure_id: None,
            slot_number: None,
            media_errors: None,
            other_errors: None,
            predictive_failure_errors: None,
            last_predictive_failure_event_seqno: None,
            smart_flag: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_state: None,
        }
    }
}

trait StringUtils {
    fn substring(&self, start: usize, len: usize) -> Self;
}

impl StringUtils for String {
    fn substring(&self, start: usize, len: usize) -> Self {
        self.chars().skip(start).take(len).collect()
    }
}


fn parse_inquiry_data(raw_inquiry_data: &str) -> Option<InquiryData> {
    let parts: Vec<&str> = raw_inquiry_data.split_whitespace().collect();
    // special case to discern Intel SSDs. They munch some stuff together
    // without whitespace.
    let intel = "INTEL";
    match parts.len() {
        0 => {
            trace!("Inquiry Data seems to be missing: {}", raw_inquiry_data);
            None
        }
        3 => if parts[0].ends_with(intel) {
            let part0 = String::from(parts[0]);
            let intel_serial = part0.substring(0, part0.len() - intel.len());

            Some(InquiryData {
                manuf: String::from("INTEL"),
                model: String::from(parts[1]),
                serial: intel_serial,
            })
        } else {
            Some(InquiryData {
                manuf: String::from(parts[0]),
                model: String::from(parts[1]),
                serial: String::from(parts[2]),
            })
        },
        _ => {
            trace!("Inquiry Data cannot be parsed: {}", raw_inquiry_data);
            None
        }
    }
}

fn parse_firmware_state(raw_firmware_state: &str) -> Option<u8> {
    match raw_firmware_state {
        "Failed" => Some(8),
        "not Online" => Some(7),
        "Unconfigured(bad)" => Some(6),
        "Unconfigured(good), Spun down" => Some(5),
        "Hotspare, Spun down" => Some(4),
        "Hotspare, Spun up" => Some(3),
        "Rebuild" => Some(2),
        "Online, Spun Down" => Some(1),
        "Online, Spun Up" => Some(0),
        _ => None
    }
}

fn get_ldpdinfo() -> Result<Vec<PdInfo>, Error> {
    let result = execute_megacli_pdldinfo();
    let output = try!(handle_command_output("MegaCli64", result));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        let msg = format!("Failed to parse MegaCli64 output");
        trace!("Failed to parse MegaCli64 output for lines: '{}'", stdout);
        return Err(Error::CollectionError(msg));
    }

    let re_enclosure_device_id = Regex::new(r"^Enclosure Device ID: (\d+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_slot_number = Regex::new(r"^Slot Number: (\d+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_media_error_count = Regex::new(r"^Media Error Count: (\d+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_other_error_count = Regex::new(r"^Other Error Count: (\d+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_predictive_failure_count = Regex::new(r"^Predictive Failure Count: (\d+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_predictive_failure_count_event_seqno = Regex::new(r"^Last Predictive Failure Event Seq Number: (\d+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_drive_flagged_smart_alert = Regex::new(r"^Drive has flagged a S.M.A.R.T alert : (\w+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_inquiry_data = Regex::new(r"^Inquiry Data: (.+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;

    let re_firmware_state = Regex::new(r"^Firmware state: (.+)")
        .map_err(|e| Error::CollectionError(e.to_string()))?;


    let mut pdinfos = Vec::new();
    let mut current_disk: Option<PdInfoBuilder> = None;


    for line in lines {
        // Next Disk Section Begins

        if let Some(caps) = re_enclosure_device_id.captures(line) {
            if let Some(disk) = current_disk {
                pdinfos.push(disk.build());
            }
            let mut disk = PdInfoBuilder::new();

            let c = caps.get(1).unwrap().as_str().parse()?;
            current_disk = Some(disk.enclosure_id(c));
        } else if let Some(caps) = re_slot_number.captures(line) {
            if let Some(c) = caps.get(1) {
                let x = c.as_str().parse()?;
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.slot_number(x));
                }
            }
        } else if let Some(caps) = re_media_error_count.captures(line) {
            if let Some(c) = caps.get(1) {
                let x = c.as_str().parse()?;
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.media_errors(x));
                }
            }
        } else if let Some(caps) = re_other_error_count.captures(line) {
            if let Some(c) = caps.get(1) {
                let x = c.as_str().parse()?;
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.other_errors(x));
                }
            }
        } else if let Some(caps) = re_predictive_failure_count.captures(line) {
            if let Some(c) = caps.get(1) {
                let x = c.as_str().parse()?;
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.predictive_failure_errors(x));
                }
            }
        } else if let Some(caps) = re_predictive_failure_count_event_seqno.captures(line) {
            if let Some(c) = caps.get(1) {
                let x = c.as_str().parse()?;
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.last_predictive_failure_event_seqno(x));
                }
            }
        } else if let Some(caps) = re_drive_flagged_smart_alert.captures(line) {
            if let Some(c) = caps.get(1) {
                if let Some(disk) = current_disk {
                    let smartflag = c.as_str().to_lowercase() == "yes";
                    current_disk = Some(disk.smart_flag(smartflag));
                }
            }
        } else if let Some(caps) = re_firmware_state.captures(line) {
            if let Some(firmware_state) = match caps.get(1) {
                Some(c) => parse_firmware_state(c.as_str()),
                _ => None
            } {
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.firmware_state(firmware_state));
                }
            }
        } else if let Some(caps) = re_inquiry_data.captures(line) {
            if let Some(inquiry_data) = match caps.get(1) {
                Some(c) => parse_inquiry_data(c.as_str()),
                _ => None
            } {
                if let Some(disk) = current_disk {
                    current_disk = Some(disk.manufacturer(inquiry_data.manuf)
                        .model(inquiry_data.model)
                        .serial_number(inquiry_data.serial));
                }
            }
        } else {
            trace!("Line '{:?}' did not match any regexes", line);
        }
    }
    if let Some(disk) = current_disk {
        pdinfos.push(disk.build());
    }
    Ok(pdinfos)
}

fn execute_megacli_pdldinfo() -> IoResult<Output> {
    // TODO: use timeout for execution
    Command::new("/bin/cat").arg("/Users/ds/ldpdinfo.txt").output()
}

fn pdinfo_to_samples(_: &HashMap<String, Metadata>, pdinfo: PdInfo) -> Vec<Sample> {
    let mut tags = Tags::new();

    if let Some(x) = pdinfo.slot_number {
        tags.insert("slot_number".to_string(), x.to_string());
    }
    if let Some(x) = pdinfo.enclosure_id {
        tags.insert("enclosure_id".to_string(), x.to_string());
    }
    if let Some(x) = pdinfo.serial_number {
        tags.insert("serial_number".to_string(), x.to_string());
    }
    if let Some(x) = pdinfo.model {
        tags.insert("model".to_string(), x.to_string());
    }
    if let Some(x) = pdinfo.manufacturer {
        tags.insert("manufacturer".to_string(), x.to_string());
    }


    let mut samples = Vec::new();

    pdinfo.media_errors
        .map(|x| samples.push(Sample::new_with_tags("mediaerrors", x, tags.clone())));
    pdinfo.other_errors
        .map(|x| samples.push(Sample::new_with_tags("othererrors", x, tags.clone())));
    pdinfo.predictive_failure_errors
        .map(|x| samples.push(Sample::new_with_tags("predfailerrors", x, tags.clone())));
    pdinfo.smart_flag // todo bool nach 0 / 1
        .map(|x| samples.push(Sample::new_with_tags("smartflag", if x { 1 } else { 0 }, tags.clone())));
    pdinfo.firmware_state
        .map(|x| samples.push(Sample::new_with_tags("firmwarestate", x, tags.clone())));
    pdinfo.last_predictive_failure_event_seqno
        .map(|x| samples.push(Sample::new_with_tags("predfaileventno", x, tags.clone())));

    samples
}
