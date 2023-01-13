use std::sync::Mutex;
use crossbeam::channel::{Receiver, Sender};
use crate::dna::{Dna, DnaData};

#[derive(Default, Clone)]
pub struct DnaCommand {
    pub dna: Option<*mut Dna>
}



pub static mut WRITER_DNA_QUEUE_CHANNEL: Option<Sender<*mut DnaCommand>> = None;
pub static mut READER_DNA_QUEUE_CHANNEL: Option<Receiver<*mut DnaCommand>> = None;

pub static mut WRITER_DNA_RESULT_QUEUE_CHANNEL: Option<Sender<i8>> = None;
pub static mut READER_DNA_RESULT_QUEUE_CHANNEL: Option<Receiver<i8>> = None;


pub static mut DNA_PROCESS: DnaProcess = DnaProcess {
    number: 0,
    target_normal_nodes_count: 0,
    target_ascendancy_nodes_count: 0,
};

pub static mut DNA_PROCESS_STATUS: Option<Mutex<DnaProcessStatus>> = None;
