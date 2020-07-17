use crate::vue::{RENDER, VUE_RENDERER};
use quick_js::Context;
use std::fs::read_to_string;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

static BUNDLE_PATH: &'static str = "./app/bundle.js";

type Job = Box<RendererRequest>;

struct RendererRequest {
    url: String,
    sender: mpsc::Sender<String>,
}

pub struct RendererPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

impl RendererPool {
    pub fn new(size: usize) -> RendererPool {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        lazy_static! {
            static ref BUNDLE: String = read_to_string(BUNDLE_PATH).unwrap();
        }
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver), &BUNDLE));
        }
        RendererPool { workers, sender }
    }
    pub fn render(&self, url: String) -> String {
        let (sender, receiver) = mpsc::channel();
        let ssr_request = RendererRequest { url, sender };
        let job = Box::new(ssr_request);
        self.sender.send(job).unwrap();
        receiver.recv().unwrap()
    }
}

struct Worker {}

impl Worker {
    fn new<'a>(receiver: Arc<Mutex<mpsc::Receiver<Job>>>, bundle: &'static str) -> Worker {
        thread::spawn(move || {
            let ctx = Context::new().unwrap();
            let shared_ctx = Arc::new(Mutex::new(&ctx));
            let _loaded_renderer = ctx.eval(VUE_RENDERER).unwrap();
            let _loaded_bundle = ctx.eval(bundle).unwrap();
            let (tx, rx): (mpsc::Sender<Context>, mpsc::Receiver<Context>) = channel();
            let (shared_ctx, tx) = (Arc::clone(&shared_ctx), tx.clone());
            loop {
                let ctx = shared_ctx.lock().unwrap();
                let job = receiver.lock().unwrap().recv().unwrap();
                println!("Worker got a job; executing.");
                let result = ctx.eval(RENDER).unwrap();
                job.sender.send(result.into_string().unwrap()).unwrap();
            }
        });

        Worker {}
    }
}
