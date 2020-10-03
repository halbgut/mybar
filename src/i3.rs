extern crate serde_json;
extern crate serde;

use std::{io,thread,time};
use std::sync::mpsc;
use std::io::{Write};
use std::collections::{HashMap};
use serde::{Serializer};
use serde::ser::{SerializeSeq};

#[derive(serde::Serialize)]
struct I3BarInit {
    version: String,
}

pub struct I3 {
    writer: io::Stdout,
    handle: Option<thread::JoinHandle<()>>,
    transmitter: mpsc::Sender<Item>,
}

impl I3 {
    pub fn new () -> Result<I3, ()> {
        let (tx, rx) = mpsc::channel();

        let mut i3 = I3 {
            writer: io::stdout(),
            handle: None,
            transmitter: tx,
        };
        i3.init(rx)?;

        Ok(i3)
    }

    pub fn send(&mut self, item: Item) -> Result<(), ()> {
        match self.transmitter.send(item) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    fn init (&mut self, receiver: mpsc::Receiver<Item>) -> Result<(), ()> {
        let init = I3BarInit { version: "1".to_owned() };
        match serde_json::to_string(&init) {
            Ok(init_str) => {
                self.write(&init_str.as_bytes())?;
                flush();

                let handle: thread::JoinHandle<_> = thread::spawn(move || {
                    let mut serializer = serde_json::Serializer::new(io::stdout());
                    let mut sequence = serializer.serialize_seq(None).unwrap();
                    let mut state = HashMap::new();
                    let mut line_keys: Vec<String> = vec![];

                    loop {
                        thread::sleep(time::Duration::new(1, 0));
                        for item in receiver.try_iter() {
                            if !&line_keys.contains(&item.name) {
                                &line_keys.push(item.name.clone());
                            }
                            state.insert(item.name.clone(), item);
                        }
                        let line: Vec<&Item> = (&line_keys)
                            .into_iter()
                            .map(|name| state.get(name).unwrap())
                            .collect();
                        sequence.serialize_element(&line).unwrap();
                        flush();
                    }
                });

                self.handle = Some(handle);
                Ok(())
            },
            Err(_) => Err(()),
        }
    }

    fn write (&mut self, data: &[u8]) -> Result<(), ()> {
        match self.writer.write(data) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

fn flush() {
    std::io::stdout().write("\n".as_bytes()).unwrap();
}

#[derive(serde::Serialize)]
pub struct Item {
    pub name: String,
    pub full_text: String,
    pub color: String,
}
