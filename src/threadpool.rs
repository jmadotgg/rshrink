// Building the ThreadPool Struct Using Compiler Driven Development
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self {
            workers: Default::default(),
            sender: {
                let (sender, _) = mpsc::channel();
                sender
            },
        }
    }
}

// Type alias for a trait object that holds the type of closure that execute receives
type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    // Add documentation with doc comment
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool
    ///
    ///
    // Good documentation pracitce to add a section that calls out the situations in which our
    // function can panic
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    // usize, because negative number of threads doesn't make sense
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));
        // with_capacity is the same as new except that it preallocates space in the vector which
        // is slightly more efficient (new resizes the Vector for each item)
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // create some threads and store them in the vector
            // Channel is multi producer single consumer, therefore we need the Arc Type to let
            // multiple workers own the receiver, and Mutex will ensure that only one worker gets a
            // job from the receiver at a time
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }
    // We still use the () after FnOnce because this FnONce represents a closure that takes not
    // parameters and returns the unit type ()
    // The F type pararmeter slo has the trait bound Send and the lifetime bound 'static, which are
    // useful in our situation: we need Send to tranfer the closure form one thread to another and
    // 'static because we don't know how long the thread will takte to execute
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Create new job instance using the closure
        let job = Box::new(f);

        // Send job down the sending end of the channel
        // Call unwrap in case sending fails
        // This might happen if, for example, we stop all our threads from executing, meaning the
        // receiving end has stopped receiving new messages
        // The reason we use unwrap is that we know the failure case won't happen (The threads
        // continue executing as long as the pool exists), but the compiler
        // doensn't know that
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

// Docs: https://doc.rust-lang.org/stable/book/ch20-03-graceful-shutdown-and-cleanup.html
impl Drop for ThreadPool {
    fn drop(&mut self) {
        // println!("Sending terminate message to all workers.");

        // Two loops to prevent Deadlocks
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        // println!("Shutting down all workers.");

        for worker in &mut self.workers {
            // println!("Shutting down worker {}", worker.id);

            // Take ownership of thread, leave None in place
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}

// Worker is a common term in pooling implementations. Think of people working in the kitchen at a
// restaurant : the workers wait until orders come in form customers, and then they're responsible
// for taking those orders and filling them

struct Worker {
    // id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(_id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    // println!("Worker {} got a job; executing.", id);

                    job();
                }
                Message::Terminate => {
                    // println!("Worker {} was told to terminate.", id);

                    break;
                }
            }

            // Call lock on the receiver to acquire the mutex, and the we call unwrap to panic on
            // any errors. Acquiring a lock might fail if the mutex is poisoned state, which can
            // happen if some other thread panicked while holding the lock reather than releasing
            // the lock. In this situation, calling unwrap to have this thread panic is the correct
            // action to take.
            // Shouldn't use a while let loop (or if let, match) here, because it would not drop temprorary values
            // until the end of the associated block, resulting in other workers not beiing able to
            // receive jobs. With let job = receiver.lock().unwrap().recv().unwrap(); however, any
            // temporary values used in the expression on the right hand side of the equals sing
            // are immediately dropped when the let statement ends.
        });

        Worker {
            // id,
            thread: Some(thread),
        }
    }
}
