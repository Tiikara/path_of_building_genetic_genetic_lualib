use std::cmp::Ordering;
use std::sync::{mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use mlua::prelude::*;
use rand::prelude::ThreadRng;
use rand::Rng;
use crate::dna::Dna;

static mut WRITER_DNA_QUEUE_CHANNEL: Option<Sender<*mut Dna>> = None;
static mut READER_DNA_QUEUE_CHANNEL: Option<Mutex<Receiver<*mut Dna>>> = None;

static mut WRITER_DNA_RESULT_QUEUE_CHANNEL: Option<Sender<i8>> = None;
static mut READER_DNA_RESULT_QUEUE_CHANNEL: Option<Receiver<i8>> = None;

unsafe fn init_genetic_solver(_: &Lua) {
    let (writer_dna_queue_channel, reader_dna_queue_channel): (Sender<*mut Dna>, Receiver<*mut Dna>) =
        mpsc::channel();

    unsafe {
        WRITER_DNA_QUEUE_CHANNEL = Some(writer_dna_queue_channel);
        READER_DNA_QUEUE_CHANNEL = Some(Mutex::new(reader_dna_queue_channel));
    }

    let (writer_dna_result_queue_channel, reader_dna_result_queue_channel): (Sender<i8>, Receiver<i8>) =
        mpsc::channel();

    unsafe {
        WRITER_DNA_RESULT_QUEUE_CHANNEL = Some(writer_dna_result_queue_channel);
        READER_DNA_RESULT_QUEUE_CHANNEL = Some(reader_dna_result_queue_channel);
    }
}

fn start_genetic_solver(
    _: &Lua,
    generations_count: usize,
    tree_nodes_count: usize,
    population_max_generation_size: usize
) {
    if population_max_generation_size % 2 != 0
    {
        panic!("population_max_generation_size should be 2");
    }

    let mut population = Vec::with_capacity(10000);

    let mut bastards = Vec::with_capacity(10000);

    let mut rng = rand::thread_rng();

    let writer_dna_queue_channel = unsafe {
        match &WRITER_DNA_QUEUE_CHANNEL {
            Some(writer_dna_queue_channel) => writer_dna_queue_channel,
            None => panic!("Dolbaeb")
        }
    };

    let reader_dna_result_queue_channel = unsafe {
        match &READER_DNA_RESULT_QUEUE_CHANNEL {
            Some(reader_dna_result_queue_channel) => reader_dna_result_queue_channel,
            None => panic!("Dolbaeb")
        }
    };

    for _ in 1..1600 {
        population.push(Box::new(Dna::new(tree_nodes_count)))
    }

    calc_fitness_with_worker(
        writer_dna_queue_channel,
        reader_dna_result_queue_channel,
        &mut population[0..population.len()]
    );

    population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

    for _ in 1..=generations_count {
        let start_mutated_index = population.len();

        for i in 0..population.len() {
            let mutated_dna = Box::new(population[i].clone_dna());

            mutated_dna.mutate(&mut rng);

            population.push(mutated_dna);
        }

        calc_fitness_with_worker(
            writer_dna_queue_channel,
            reader_dna_result_queue_channel,
            &mut population[start_mutated_index..population.len()]
        );

        population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

        let count_of_fucks =
            if population_max_generation_size / 2 > population.len() {
                population.len()
            }
            else {
                population_max_generation_size / 2
            };

    }

}

fn calc_fitness_with_worker(writer_dna_queue_channel: &Sender<*mut Dna>,
                            reader_dna_result_queue_channel: &Receiver<i8>,
                            dnas: &mut [Box<Dna>])
{
    for dna in dnas.iter_mut() {
        writer_dna_queue_channel.send(&mut **dna).expect("Cannot send dna to queue");
    }

    for _ in 1..=dnas.len() {
        reader_dna_result_queue_channel.recv().expect("Cannot receive dna result signal");
    }
}

fn make_hard_fuck(dna_masters: &[Box<Dna>], dna_slaves: &[Box<Dna>], out_bastards: Vec<Box<Dna>>, rng: &mut ThreadRng)
{
    for dna_master in dna_masters {
        let index_of_slave = rng.gen_range(0..dna_slaves.len());
        let dna_slave = &dna_slaves[index_of_slave];

        //out_bastards.push();
    }
}
