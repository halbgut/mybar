use std::io;
use std::str;
use std::io::{Read};
use std::fs::{File};
use std::str::FromStr;

pub struct Mem {}

#[derive(Debug)]
pub struct MemInfo {
    pub free: u64,
    pub used: u64,
    pub total: u64,
}

impl Mem {
    pub fn new() -> Self {
        Mem { }
    }

    pub fn read(&self) -> Result<MemInfo, ()> {
        match self.p_read() {
            Ok(meminfo) => Ok(meminfo),
            Err(_) => Err(())
        }
    }

    fn p_read(&self) -> io::Result<MemInfo> {
        let mut meminfo_file = File::open("/proc/meminfo")?;
        let mut free_opt = None;
        let mut total_opt = None;

        let mut chunk_buf = Vec::with_capacity(256);
        while free_opt.is_none() && total_opt.is_none() {
            let mut buf = [0; 128];
            let red_bytes = meminfo_file.read(&mut buf)?;
            if red_bytes == 0 {
                return Err(
                    io::Error::new(io::ErrorKind::Other, "Data not found")
                );
            }

            chunk_buf.extend_from_slice(&buf);
            let meminfo_chunk = str::from_utf8(&chunk_buf).unwrap();
            let mut sub_meminfo_chunk = &meminfo_chunk[..];
            let mut last_idx = 0;
            loop {
                let idx_res = sub_meminfo_chunk.find('\n');
                if idx_res.is_none() { break }
                let idx = idx_res.unwrap();
                let raw_line = sub_meminfo_chunk.get(0..idx);
                let line = parse_meminfo_line(raw_line)?;
                if line.key == "MemAvailable" {
                    free_opt = Some(line.value);
                } else if line.key == "MemTotal" {
                    total_opt = Some(line.value);
                }
                let new_idx = last_idx + idx + 1;
                sub_meminfo_chunk = &meminfo_chunk[new_idx..];
                last_idx = new_idx;
            }
            let new_buf = sub_meminfo_chunk.as_bytes().to_vec();
            chunk_buf = new_buf;
        }

        let free = free_opt.unwrap();
        let total = total_opt.unwrap();
        let used = total - free;
        Ok(MemInfo {
            total,
            free,
            used
        })
    }
}

struct MemInfoLine {
    key: String,
    value: u64,
}

fn parse_meminfo_line(raw: Option<&str>) -> io::Result<MemInfoLine> {
    if raw.is_none() {
        return Err(io::Error::new(io::ErrorKind::Other, "No line found"));
    }
    let line = raw.unwrap();
    let mut key_value_pair = line.trim().split(':');
    let key = key_value_pair.nth(0).unwrap();
    let raw_value = key_value_pair.nth(0).unwrap();
    let str_value = raw_value.trim()
        // split away unit
        .split(' ').nth(0).unwrap();
    let value = u64::from_str(str_value).unwrap();
    Ok(MemInfoLine {
        key: key.to_string(),
        value,
    })
}
