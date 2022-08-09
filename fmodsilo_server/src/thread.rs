use std::{thread::{JoinHandle, self}, sync::{mpsc, Arc, Mutex}};

type Task = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Task(Task),
    Terminate
}

pub struct ThreadPool {
    threads: Vec<Option<JoinHandle<()>>>,
    transmitter: mpsc::Sender<Message>
}

impl ThreadPool {
    pub fn new(count: usize) -> ThreadPool {
        assert!(count > 0);

        let mut threads = Vec::with_capacity(count);

        let (transmitter, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for _ in 0..count {
            let rec = Arc::clone(&receiver);

            threads.push(Some(thread::spawn(move || {
                loop {
                    let message = rec.lock().unwrap().recv().unwrap();

                    match message {
                        Message::Task(job) => job(),
                        Message::Terminate => break
                    };
                }
            })));
        }

        ThreadPool{ threads, transmitter }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, func: F) {
        self.transmitter.send(Message::Task(Box::new(func))).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.threads {
            self.transmitter.send(Message::Terminate).unwrap();
        }

        for handle in &mut self.threads {
            if let Some(thr) = handle.take() {
                thr.join().unwrap();
            }
        }
    }
}