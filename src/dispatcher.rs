use std::{sync::{atomic::{AtomicBool, Ordering}, Arc, Barrier}, thread::JoinHandle};
use parking_lot::{Condvar, Mutex, RwLock};

use crate::{Internal, InternalData, ResourceMask, World};

pub struct Dispatcher {
    pub(crate) handles: Vec<JoinHandle<()>>,
    pub(crate) global_barrier: Arc<Barrier>,
    quit: Arc<AtomicBool>,
}

impl Dispatcher {
    pub(crate) fn build(per_thread: Vec<Vec<Option<Internal>>>, world: Arc<RwLock<World>>) -> Self {
        let total = per_thread.len();
        let group_barrier = Arc::new(Barrier::new(total));
        let global_barrier = Arc::new(Barrier::new(total + 1));
        let var = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::<JoinHandle<()>>::new();

        for (i, mut data) in per_thread.into_iter().enumerate() {
            let group_barrier = group_barrier.clone();
            let global_barrier = global_barrier.clone();
            let var = var.clone();
            let world = world.clone();
            let builder = std::thread::Builder::new().name(format!("thread-{i}"));
            let handle = builder.spawn(move || {
                loop {
                    if var.load(Ordering::Relaxed) {
                        //break;
                    }

                    global_barrier.wait();

                    for group in data.iter_mut() {
                        group_barrier.wait();
                        if let Some(Internal { boxed, reads, writes, .. }) = group {
                            let world = world.read();
                            let data = InternalData {
                                read: *reads,
                                write: *writes,
                            };

                            world.set_internal(data);
                            boxed(&world);
                        }
                        group_barrier.wait();
                    }

                    global_barrier.wait();
                }
            }).unwrap();
            handles.push(handle);
        }

        Self {
            handles: Vec::default(),
            global_barrier,
            quit: var
        }
    }
    
    pub fn dispatch(&mut self) {
        self.global_barrier.wait();
        self.global_barrier.wait();
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        
        self.quit.store(true, Ordering::Relaxed);
        for thread in self.handles.drain(..) {
            println!("Bruh");
            thread.join().unwrap();
        }
    }
}