use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use crate::dna::Dna;

#[derive(Default, Clone)]
pub struct Reinit {
    pub target_normal_nodes_count: usize,
    pub target_ascendancy_nodes_count: usize
}

#[derive(Default, Clone)]
pub struct DnaCommand {
    pub dna: Option<*mut Dna>,
    pub stop_thread: bool,
    pub reinit: Option<Reinit>
}


pub static mut WRITER_DNA_QUEUE_CHANNEL: Option<Sender<*mut DnaCommand>> = None;
pub static mut READER_DNA_QUEUE_CHANNEL: Option<Mutex<Receiver<*mut DnaCommand>>> = None;

pub static mut WRITER_DNA_RESULT_QUEUE_CHANNEL: Option<Mutex<Sender<i8>>> = None;
pub static mut READER_DNA_RESULT_QUEUE_CHANNEL: Option<Receiver<i8>> = None;
