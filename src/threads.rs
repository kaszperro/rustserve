use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

struct Worker {
    id: usize,
    handle: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let handle = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv();
                match job {
                    Ok(job) => {
                        println!("Worker {id} got a job, executing");
                        job();
                    }
                    Err(e) => {
                        println!("Worker {id} disconnected: {e}");
                        break;
                    }
                }
            }
        });

        Worker { id, handle }
    }
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> ThreadPool {
        let mut workers = Vec::with_capacity(num_threads);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for id in 0..num_threads {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender.as_ref().unwrap().send(Box::new(f)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in self.workers.drain(..) {
            println!("Shutting down worker {}", worker.id);
            worker.handle.join().unwrap();
        }
    }
}
