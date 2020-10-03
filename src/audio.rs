extern crate libpulse_binding as pulse;

use std::{thread,time};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::mpsc;
use pulse::mainloop::standard::Mainloop;
use pulse::mainloop::standard::IterateResult;
use pulse::context::Context;
use pulse::callbacks::ListResult;
use pulse::volume;
use pulse::context::subscribe::subscription_masks;

pub struct Audio {
    volume: u8,
    receiver: mpsc::Receiver<u8>,
}

// as it turns out, i don't need no pulseaudio. all info is in ALSA, alsa
// provides a more fine-grained event based API
// https://stackoverflow.com/questions/34936783/watch-for-volume-changes-in-alsa-pulseaudio
impl Audio {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let audio = Audio {
            receiver: rx,
            volume: 0,
        };

        thread::spawn(move || {
            loop {
                let mut pulse = Pulse::new()
                    .expect("failed to connect to pulse server");
                // pulse.listen();
                let mut sinks = pulse.introspect()
                    .expect("failed to introspect");
                sinks.sort_by(|a, b| b.open.cmp(&a.open));
                sinks.sort_by(|a, b| b.open.cmp(&a.running));

                let sink = &(sinks[0]);
                let volume = if sink.mute {
                    0
                } else {
                    sink.volume
                };
                tx.send(volume).unwrap();

                thread::sleep(time::Duration::new(1, 0));
            }
        });

        audio
    }

    pub fn get_volume(&mut self) -> u8 {
        for volume in self.receiver.try_iter() {
            self.volume = volume;
        }
        self.volume
    }
}

struct Pulse {
    context: Context,
    mainloop: Mainloop,
}

impl Pulse {
     fn new() -> Result<Self, ()> {
        match Mainloop::new() {
            Some(mainloop) => {
                match Context::new(&mainloop, &"mybar") {
                    Some(context) => {
                        let mut pulse = Pulse { context, mainloop };
                        pulse.connect()?;
                        Ok(pulse)
                    },
                    None => Err(()),
                }
            },
            None => Err(())
        }
     }

     fn iterate(&mut self) -> Result<(), ()> {
        match self.mainloop.iterate(false) {
            IterateResult::Quit(_) |
            IterateResult::Err(_) => {
                return Err(())
            },
            IterateResult::Success(_) => Ok(()),
        }
     }

     fn connect(&mut self) -> Result<(), ()> {
        let connect_res = self.context.connect(None, pulse::context::flags::NOFLAGS, None);
        if connect_res.is_err() { return Err(()); }

        loop {
            self.iterate()?;

            match self.context.get_state() {
                pulse::context::State::Ready => { break; }
                pulse::context::State::Terminated |
                pulse::context::State::Failed => {
                    return Err(())
                },
                _ => {},
            }
        }

        Ok(())
     }

     fn introspect(&mut self) -> Result<Vec<Sink>, ()> {
         // probably need to allocate this in the heap so that it has a static
         // lifetime that way we can then drop it after `ListResult::End` has
         // been received.
         let sink_vec = Rc::new(RefCell::new(vec![] as Vec<Sink>));
         let done = Rc::new(RefCell::new(false));
         let cb_done = Rc::clone(&done);
         let cb_sink_vec = Rc::clone(&sink_vec);
         self.context.introspect().get_sink_info_list(
             move |list_result| {
                 match list_result {
                     ListResult::Item(sink) => {
                         let volume::Volume(avg) = sink.volume.avg();
                         let volume::Volume(norm) = volume::VOLUME_NORM;

                         let name = if sink.name.is_some() {
                             sink.name.clone().unwrap().to_string()
                         } else {
                             "".to_owned()
                         };
                         cb_sink_vec.borrow_mut().push(Sink {
                             // TODO no idea what this does
                             volume: ((
                                 avg * (100 as u32) +
                                 norm / 2
                             ) / norm) as u8,
                             name,
                             running: sink.state.is_running(),
                             open: sink.state.is_opened(),
                             mute: sink.mute,
                         })
                     },
                     ListResult::End |
                     ListResult::Error => {
                         *(cb_done.borrow_mut()) = true;
                     },
                 }
             }
         );
         loop {
             let res = self.iterate();
             if res.is_err() {
                 return Err(())
             }

             if *(done.borrow()) {
                 return Ok(sink_vec.borrow().clone());
             }
         }
     }

     fn listen(&mut self) {
        self.context.subscribe(subscription_masks::ALL, |_| {});
        self.context.set_subscribe_callback(Some(Box::new(|a,b,c| {
            println!("{:?} {:?} {:?}", a, b, c);
        })));
        self.mainloop.run().unwrap();
     }
}

#[derive(Clone,Debug)]
struct Sink {
    volume: u8,
    name: String,
    running: bool,
    open: bool,
    mute: bool,
}
