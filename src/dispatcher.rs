use std::sync::{Arc, Barrier};

use crate::World;

#[derive(Default)]
pub struct Dispatcher {

    pub(crate) per_thread: Vec<Vec<Box<dyn FnMut(&World) + Sync + Send>>>,
}

impl Dispatcher {
    pub fn dispatch(self, threads: usize, world: Arc<World>) {
        let mut per_thread = self.per_thread;
        let group = Arc::new(Barrier::new(threads));

        for i in 0..threads {
            let barrier = group.clone();
            let mut data = per_thread.remove(0);
            let world = world.clone();
            std::thread::spawn(move || {
                loop {
                    /*
                    for group in data.iter_mut() {
                        barrier.wait();
                        if let Some(func) = group {
                            func(&world);
                        }
                    }
                    */
                }
            });
        }
    }
}