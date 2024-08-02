use std::{sync::{Arc, Barrier}, thread::JoinHandle};

use crate::World;

#[derive(Default)]
pub struct Dispatcher {
    pub(crate) per_thread: Vec<Vec<Option<Box<dyn FnMut(&World) + Sync + Send>>>>,
    pub(crate) handles: Vec<JoinHandle<()>>,
}

impl Dispatcher {
    pub fn dispatch(mut self, world: Arc<World>) {
        let group = Arc::new(Barrier::new(self.per_thread.len()));

        for (_, mut data) in self.per_thread.into_iter().enumerate() {
            let barrier = group.clone();
            let world = world.clone();
            let handle = std::thread::spawn(move || {
                loop {
                    for group in data.iter_mut() {
                        barrier.wait();
                        if let Some(func) = group {
                            func(&world);
                        }
                    }
                }
            });
            self.handles.push(handle);
        }

        for i in self.handles {
            i.join().unwrap();
        }
    }
}