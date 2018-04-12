use std::fs::File;
use std::io::Read;

// TODO: Implement API to allow callbacks backed via uevent / inotify

fn read_attribute(attr: &str) -> Result<String, String> {
    let mut data = String::new();
    match File::open(format!("/sys/class/power_supply/bq27441/{0}", attr)) {
        Err(e) => Err(format!("Unable to open file: {0}", e)),
        Ok(ref mut f) => match f.read_to_string(&mut data).unwrap_or(0) {
            0 => Err("Unable to read file".to_owned()),
            _ => Ok(data.trim().to_owned()),
        },
    }
}

/// $ cat /sys/class/power_supply/bq27441/capacity
/// 97
pub fn percentage() -> Result<i32, String> {
    let curr = read_attribute("capacity")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => {
            Err("Unable to parse the contents of 'capacity' during a battery query".to_owned())
        }
    }
}

/// $ cat /sys/class/power_supply/bq27441/capacity_level
/// Normal
pub fn human_readable_capacity_level() -> Result<String, String> {
    Ok(read_attribute("capacity_level")?)
}

/// $ cat /sys/class/power_supply/bq27441/charge_full
/// 1635000
pub fn charge_full() -> Result<i32, String> {
    let curr = read_attribute("charge_full")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => {
            Err("Unable to parse the contents of 'charge_full' during a battery query".to_owned())
        }
    }
}

/// $ cat /sys/class/power_supply/bq27441/charge_full_design
/// 1340000
pub fn charge_full_design() -> Result<i32, String> {
    let curr = read_attribute("charge_full_design")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => Err(
            "Unable to parse the contents of 'charge_full_design' during a battery query"
                .to_owned(),
        ),
    }
}

/// $ cat /sys/class/power_supply/bq27441/charge_now
/// 1528000
pub fn charge() -> Result<i32, String> {
    let curr = read_attribute("charge_now")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => {
            Err("Unable to parse the contents of 'charge_now' during a battery query".to_owned())
        }
    }
}

/// $ cat /sys/class/power_supply/bq27441/status
/// Discharging
pub fn human_readable_charging_status() -> Result<String, String> {
    Ok(read_attribute("status")?)
}

/// $ cat /sys/class/power_supply/bq27441/temp
/// 201
pub fn temperature() -> Result<i32, String> {
    let curr = read_attribute("temp")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => {
            Err("Unable to parse the contents of 'current_now' during a battery query".to_owned())
        }
    }
}

/// $ cat /sys/class/power_supply/bq27441/voltage_now
/// 4164000
pub fn voltage() -> Result<i32, String> {
    let curr = read_attribute("voltage_now")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => {
            Err("Unable to parse the contents of 'voltage_now' during a battery query".to_owned())
        }
    }
}

/// $ cat /sys/class/power_supply/bq27441/current_now
/// -132000
pub fn current() -> Result<i32, String> {
    let curr = read_attribute("current_now")?;
    match curr.parse::<i32>() {
        Ok(r) => Ok(r),
        Err(_) => {
            Err("Unable to parse the contents of 'current_now' during a battery query".to_owned())
        }
    }
}
