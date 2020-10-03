extern crate serde;
extern crate serde_json;
extern crate chrono;

use std::{time,thread,fs};
use chrono::{Datelike,Timelike};
use std::str::FromStr;
use std::io;

mod i3;
mod link;
mod audio;
mod mem;

struct Item {
    name: String,
    text: String,
    good: Option<bool>,
}

impl Item {
    fn to_i3_item (self: &Item) -> i3::Item {
        let color;
        if self.good.is_some() {
            if self.good.unwrap() {
                color = "#00FF00".to_owned();
            } else {
                color = "#FF0000".to_owned();
            }
        } else {
            color = "#FFFFFF".to_owned();
        }

        i3::Item {
            name: self.name.clone(),
            full_text: self.text.clone(),
            color,
        }
    }
}

// todo use `?` short syntax
fn main() {
    start_bar().unwrap();
}

fn start_bar() -> Result<(), ()> {
    let mut bar = i3::I3::new()?;
    let mut net = link::Link::new()?;

    let mut pulse = audio::Audio::new();
    let memory = mem::Mem::new();

    let mut i = 0;
    loop {
        let inet = get_inet(&net)?.to_i3_item();
        bar.send(inet)?;

        if i % 5 == 0 {
            let traffic = get_traffic(&mut net)?.to_i3_item();
            bar.send(traffic)?;
        }

        let memory = memory.read()?;
        bar.send(format_mem(memory))?;

        bar.send(get_cpu())?;

        let volume = get_volume(&mut pulse)?.to_i3_item();
        bar.send(volume)?;

        let volume = get_volume(&mut pulse)?.to_i3_item();
        bar.send(volume)?;

        let battery = read_battery()?.to_i3_item();
        bar.send(battery)?;

        let date_time = get_date_time()?.to_i3_item();
        bar.send(date_time)?;

        i += 1;
        thread::sleep(time::Duration::new(1, 0));
    }
}

fn read_battery() -> Result<Item, ()> {
    match read_battery_p() {
        Ok(item) => Ok(item),
        Err(_) => Err(()),
    }
}

fn read_battery_p() -> io::Result<Item> {
    let capacity_path = "/sys/class/power_supply/BAT0/capacity";
    let charging_path = "/sys/class/power_supply/ADP1/online";

    let capacity_str = fs::read_to_string(capacity_path)?;
    let capacity = u64::from_str(capacity_str.trim()).unwrap();

    let charging_str = fs::read_to_string(charging_path)?;
    let is_charging = charging_str.trim() == "1";

    let good = if is_charging {
        Some(true)
    } else if capacity < 30 {
        Some(false)
    } else if capacity > 70 {
        Some(true)
    } else {
        None
    };

    let affix =
        if is_charging { "\u{1f5f2} ".to_owned() }
        else { "%".to_owned() };

    Ok(Item {
        name: "Battery".to_owned(),
        text: format!("bat {}{}", capacity, affix),
        good,
    })
}

fn get_inet(net: &link::Link) -> Result<Item, ()> {
    let text;
    let good = net.is_up()?;

    if good {
        text = "\u{263C}".to_owned();
    } else {
        text = "\u{2694}".to_owned();
    }

    Ok(Item {
        name: "INet".to_owned(),
        text,
        good: Some(good),
    })
}

fn get_traffic(net: &mut link::Link) -> Result<Item, ()> {
    let stats = net.stats()?;
    Ok(Item {
        name: "Traffic".to_owned(),
        text: format!(
            "net \u{2191}{} / \u{2193}{}",
            stats.pretty_upload(),
            stats.pretty_download()
        ).to_owned(),
        good: None,
    })
}

// TODO: better error handling
fn get_date_time() -> Result<Item, ()> {
    let now = chrono::Local::now();

    let item = Item {
        name: "DateTime".to_owned(),
        text: format!(
            "{}.{:0>2}.{:0>2} {:0>2}.{:0>2}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
        ).to_owned(),
        good: None,
    };

    Ok(item)
}

fn get_volume(pulse: &mut audio::Audio) -> Result<Item, ()> {
    let volume = pulse.get_volume();
    Ok(Item {
        name: "AudioVolume".to_owned(),
        text: format!("aud {} %", volume).to_owned(),
        good: None,
    })
}

fn format_mem_amount(amount: u64) -> f64 {
    amount as f64 / 1_000_000 as f64
}

fn format_mem(info: mem::MemInfo) -> i3::Item {
    let used = format_mem_amount(info.used);
    let total = format_mem_amount(info.total);
    let good = if used / total > 0.8 {
        Some(false)
    } else {
        None
    };
    let item = Item {
        name: "Memory".to_owned(),
        text: format!("mem {:.*}/{:.*}", 1, used, 1, total).to_owned(),
        good,
    };
    item.to_i3_item()
}

// TODO: who cares abour errors?!
fn get_cpu() -> i3::Item {
    let stats = fs::read_to_string("/proc/loadavg").unwrap();
    let mut split = stats.split(" ");
    let min5 = split.nth(1).unwrap();
    let item = Item {
        name: "CPU Load Average".to_owned(),
        text: format!("cpu {}", min5).to_owned(),
        // TODO
        good: None,
    };
    item.to_i3_item()
}
