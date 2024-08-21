use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    thread::JoinHandle,
};

use crate::{Internal, InternalData, World};

pub struct Dispatcher {
    pub(crate) handles: Vec<JoinHandle<()>>,
    pub(crate) global_barrier: Arc<Barrier>,
    pub(crate) var: Arc<AtomicBool>,
}

impl Dispatcher {
    pub(crate) fn build(per_thread: Vec<Vec<Option<Internal>>>, world: Arc<World>) -> Self {
        let total = per_thread.len();
        log::debug!("Total: {total}");
        let var = Arc::new(AtomicBool::new(false));
        let group_barrier = Arc::new(Barrier::new(total));
        let global_barrier = Arc::new(Barrier::new(total + 1));
        let mut handles = Vec::<JoinHandle<()>>::new();

        for (i, mut data) in per_thread.into_iter().enumerate() {
            let group_barrier = group_barrier.clone();
            let global_barrier = global_barrier.clone();
            let world = world.clone();
            let name = format!("thread-{i}");
            log::debug!("Spawning dispatcher thread '{}'", &name);
            let builder = std::thread::Builder::new().name(name);
            let var = var.clone();
            let handle = builder
                .spawn(move || loop {
                    global_barrier.wait();

                    if var.load(Ordering::Relaxed) {
                        break;
                    }

                    for group in data.iter_mut() {
                        group_barrier.wait();
                        if let Some(Internal {
                            boxed,
                            reads,
                            writes,
                            ..
                        }) = group
                        {
                            let data = InternalData {
                                read: *reads,
                                write: *writes,
                            };

                            world.set_internal(Some(data));
                            boxed(&world);
                        }
                        group_barrier.wait();
                    }

                    global_barrier.wait();
                })
                .unwrap();
            handles.push(handle);
        }

        Self {
            handles,
            global_barrier,
            var,
        }
    }

    pub fn dispatch(&mut self) {
        self.global_barrier.wait();
        self.global_barrier.wait();
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        self.var.store(true, Ordering::Relaxed);
        self.global_barrier.wait();
        for thread in self.handles.drain(..) {
            thread.join().unwrap();
        }
    }
}
