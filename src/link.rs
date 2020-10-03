use std::{time,fs,f64};
use std::str::FromStr;

pub struct Link {
    last_check: Option<time::Instant>,
    last_state: Option<LinkStats>,
    ifaces: Vec<String>,
}

impl Link {
    pub fn new() -> Result<Self, ()> {
        let ifaces = get_ifaces()?;
        Ok(Link {
            ifaces,
            last_check: None,
            last_state: None,
        })
    }

    pub fn is_up(&self) -> Result<bool, ()> {
        Ok(
            (&self.ifaces)
                .into_iter()
                .any(|iface| {
                    let carrier_path = format!("{}/carrier", iface);
                    match fs::read_to_string(carrier_path) {
                        Ok(carrier) => carrier.trim() == "1",
                        Err(_) => false,
                    }
                })
        )
    }

    pub fn stats(&mut self) -> Result<LinkStats, ()> {
        let stats = (&self.ifaces)
            .into_iter()
            .fold(LinkStats { upload: 0, download: 0 }, |mut stats, iface| {
                let tx_path = get_tx_path(&iface);
                let rx_path = get_rx_path(&iface);
                let maybe_tx = read_transmition(&tx_path);
                let maybe_rx = read_transmition(&rx_path);
                if maybe_tx.is_ok() && maybe_rx.is_ok() {
                    stats.upload += maybe_tx.unwrap();
                    stats.download += maybe_rx.unwrap();
                }
                stats
            });


        let speed_stats;
        if self.last_check.is_some() {
            // XXX alternatives to using `clone` here?
            let last_state = self.last_state.clone().unwrap();
            let passed = time::Instant::now()
                .duration_since(self.last_check.unwrap())
                .as_millis() as f64;

            let upload = get_rate(stats.upload, last_state.upload, passed);
            let download = get_rate(stats.download, last_state.download, passed);

            speed_stats = LinkStats {
                upload: upload as i64,
                download: download as i64,
            };
        } else {
            speed_stats = LinkStats { upload: 0, download: 0 };
        }

        self.last_check = Some(time::Instant::now());
        self.last_state = Some(stats);
        Ok(speed_stats)
    }
}

fn get_rate(current: i64, before: i64, passed_ms: f64) -> i64 {
    let diff = current as f64 - before as f64;
    if diff > (0 as f64) {
        let rate = diff / passed_ms * (1000 as f64);
        rate as i64
    } else { 0 as i64 }
}

#[derive(Clone,Debug)]
pub struct LinkStats {
    pub upload: i64,
    pub download: i64,
}

impl LinkStats {
    pub fn pretty_upload(&self) -> String {
        pretty_bytes(self.upload)
    }

    pub fn pretty_download(&self) -> String {
        pretty_bytes(self.download)
    }
}

fn pretty_bytes(bytes: i64) -> String {
    if bytes > 999 {
        let k_bytes = bytes as f64 / 1000 as f64;
        if k_bytes > 999 as f64 {
            let m_bytes = k_bytes / 1000 as f64;
            format!("{:.*} MB", 1, m_bytes)
        } else {
            format!("{:.*} KB", 1, k_bytes)
        }
    } else {
        format!("{} B", bytes)
    }
}

fn get_tx_path(iface: &str) -> String {
    format!("{}/statistics/tx_bytes", iface)
}

fn get_rx_path(iface: &str) -> String {
    format!("{}/statistics/rx_bytes", iface)
}

fn read_transmition(path: &str) -> Result<i64, ()> {
    // TODO: optimize this, by getting the inode only once
    match fs::read_to_string(path) {
        Ok(bytes) => {
            match i64::from_str(&bytes.trim()) {
                Ok(transmition) => Ok(transmition),
                Err(_) => Err(())
            }
        },
        Err(_) => Err(())
    }
}

fn get_ifaces () -> Result<Vec<String>, ()> {
    match fs::read_dir("/sys/class/net") {
        Ok(dir) => Ok(
            dir
                .into_iter()
                .filter(|dir_entry| dir_entry.is_ok())
                .map(|dir_entry|
                     dir_entry.unwrap()
                         .path()
                         .to_str()
                         .unwrap()
                         .to_owned()
                 )
                .filter(|iface| *iface != "/sys/class/net/lo")
                .collect()
        ),
        Err(_) => Err(()),
    }
}
