use std::sync::mpsc;
use std::fs::OpenOptions;
use std::thread;
use std::io::{Seek, SeekFrom, Write, Read};
use std::path::PathBuf;
use CONTROL;

pub struct Disk {
    queue: mpsc::Receiver<Request>,
}

pub struct Handle {
    pub tx: mpsc::Sender<Request>,
}

impl Handle {
    pub fn get(&self) -> mpsc::Sender<Request> {
        self.tx.clone()
    }
}

unsafe impl Sync for Handle {}

pub enum Request {
    Write { data: Box<[u8; 16384]>, locations: Vec<Location> },
    Read { data: Box<[u8; 16384]>, locations: Vec<Location>, context: Ctx }
}

pub struct Ctx {
    pub id: usize,
    pub idx: u32,
    pub begin: u32,
    pub length: u32,
}

impl Ctx {
    pub fn new(id: usize, idx: u32, begin: u32, length: u32) -> Ctx {
        Ctx { id, idx, begin, length }
    }
}

impl Request {
    pub fn write(data: Box<[u8; 16384]>, locations: Vec<Location>) -> Request {
        Request::Write { data, locations }
    }

    pub fn read(context: Ctx, data: Box<[u8; 16384]>, locations: Vec<Location>) -> Request {
        Request::Read { context, data, locations }
    }
}

pub struct Location {
    pub file: PathBuf,
    pub offset: u64,
    pub start: usize,
    pub end: usize,
}

impl Location {
    pub fn new(file: PathBuf, offset: u64, start: usize, end: usize) -> Location {
        Location { file, offset, start, end }
    }
}

pub struct Response {
    pub context: Ctx,
    pub data: Box<[u8; 16384]>,
}

impl Disk {
    pub fn new(queue: mpsc::Receiver<Request>) -> Disk {
        Disk {
            queue
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.queue.recv() {
                Ok(Request::Write { data, locations } ) => {
                    for loc in locations {
                        OpenOptions::new().write(true).open(&loc.file).and_then(|mut f| {
                            f.seek(SeekFrom::Start(loc.offset)).unwrap();
                            f.write(&data[loc.start..loc.end])
                        }).unwrap();
                    }
                }
                Ok(Request::Read { context, mut data, locations } ) =>  {
                    for loc in locations {
                        OpenOptions::new().read(true).open(&loc.file).and_then(|mut f| {
                            f.seek(SeekFrom::Start(loc.offset)).unwrap();
                            f.read(&mut data[loc.start..loc.end])
                        }).unwrap();
                    }
                    CONTROL.disk_tx.send(Response { context, data }).unwrap();
                }
                _ => break,
            }
        }
    }
}

pub fn start() -> Handle {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        Disk::new(rx).run();
    });
    Handle { tx }
}
