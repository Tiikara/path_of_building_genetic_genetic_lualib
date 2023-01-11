use std::cmp::Ordering;
use std::sync::{mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use mlua::prelude::*;
use mlua::Value;
use rand::prelude::ThreadRng;
use rand::Rng;
use crate::dna::Dna;
use crate::globals_channels::{DnaCommand, READER_DNA_QUEUE_CHANNEL, READER_DNA_RESULT_QUEUE_CHANNEL, Reinit, WRITER_DNA_QUEUE_CHANNEL, WRITER_DNA_RESULT_QUEUE_CHANNEL};


pub fn init_genetic_solver(_: &Lua, (): ()) -> LuaResult<()> {
    let (writer_dna_queue_channel, reader_dna_queue_channel): (Sender<*mut DnaCommand>, Receiver<*mut DnaCommand>) =
        mpsc::channel();

    unsafe {
        WRITER_DNA_QUEUE_CHANNEL = Some(writer_dna_queue_channel);
        READER_DNA_QUEUE_CHANNEL = Some(Mutex::new(reader_dna_queue_channel));
    }

    let (writer_dna_result_queue_channel, reader_dna_result_queue_channel): (Sender<i8>, Receiver<i8>) =
        mpsc::channel();

    unsafe {
        WRITER_DNA_RESULT_QUEUE_CHANNEL = Some(Mutex::new(writer_dna_result_queue_channel));
        READER_DNA_RESULT_QUEUE_CHANNEL = Some(reader_dna_result_queue_channel);
    }

    Ok(())
}

pub fn start_genetic_solver(
    lua_context: &Lua,
    (max_generations_count,
        stop_generations_eps,
        population_max_generation_size,
        tree_nodes_count): (usize, usize, usize, usize),
) -> LuaResult<LuaTable>
{
    if population_max_generation_size % 2 != 0
    {
        panic!("population_max_generation_size should be 2");
    }

    let mut alloc_dna_commands: Vec<DnaCommand> = vec![Default::default(); 10000];

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

    let population_len = population.len();

    calc_fitness_with_worker(
        writer_dna_queue_channel,
        reader_dna_result_queue_channel,
        &mut alloc_dna_commands,
        &mut population[0..population_len],
    );

    population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

    let mut best_dna = population[0].clone();

    let mut count_generations_with_best = 0;

    for _ in 1..=max_generations_count {
        let start_mutated_index = population.len();

        for i in 0..population.len() {
            let mut mutated_dna = population[i].clone();

            mutated_dna.mutate(&mut rng);

            population.push(mutated_dna);
        }

        let population_len = population.len();

        calc_fitness_with_worker(
            writer_dna_queue_channel,
            reader_dna_result_queue_channel,
            &mut alloc_dna_commands,
            &mut population[start_mutated_index..population_len],
        );

        population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

        let count_of_fucks =
            if population_max_generation_size / 2 > population.len() {
                population.len()
            } else {
                population_max_generation_size / 2
            };

        make_hard_fuck(
            &population[0..count_of_fucks],
            &population[0..population.len()],
            &mut bastards,
            &mut rng,
        );

        let bastards_len = bastards.len();
        calc_fitness_with_worker(
            writer_dna_queue_channel,
            reader_dna_result_queue_channel,
            &mut alloc_dna_commands,
            &mut bastards[..bastards_len],
        );

        population.truncate(population_max_generation_size / 2);

        while let Some(bastard) = bastards.pop() {
            population.push(bastard);
        }

        population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

        if population[0].fitness_score > best_dna.fitness_score
        {
            best_dna = population[0].clone();
            count_generations_with_best = 0;
        } else {
            count_generations_with_best += 1;
        }

        if count_generations_with_best == stop_generations_eps
        {
            break;
        }
    }

    Ok(create_table_from_dna(lua_context, &best_dna))
}

fn calc_fitness_with_worker(writer_dna_queue_channel: &Sender<*mut DnaCommand>,
                            reader_dna_result_queue_channel: &Receiver<i8>,
                            alloc_dna_commands: &mut Vec<DnaCommand>,
                            dnas: &mut [Box<Dna>])
{
    for (i, dna) in dnas.iter_mut().enumerate() {
        let dna_command = &mut alloc_dna_commands[i];

        dna_command.reinit = None;
        dna_command.stop_thread = false;
        dna_command.dna = Some(&mut **dna);

        writer_dna_queue_channel.send(&mut *dna_command).expect("Cannot send dna to queue");
    }

    for _ in 1..=dnas.len() {
        reader_dna_result_queue_channel.recv().expect("Cannot receive dna result signal");
    }
}

fn make_hard_fuck(dna_masters: &[Box<Dna>], dna_slaves: &[Box<Dna>], out_bastards: &mut Vec<Box<Dna>>, rng: &mut ThreadRng)
{
    for dna_master in dna_masters {
        let index_of_slave = rng.gen_range(0..dna_slaves.len());
        let dna_slave = &dna_slaves[index_of_slave];

        out_bastards.push(Box::new(dna_master.selection(dna_slave, rng)));
    }
}


pub fn create_table_from_dna<'a>(lua_context: &'a Lua, dna: &Dna) -> LuaTable<'a>
{
    let new_table = lua_context.create_table().expect("Nu nihuya");


    let nodes_dna_table = lua_context.create_table().expect("Nu nihuya");
    for (index, nucl) in dna.body.iter().enumerate() {
        if *nucl == 1
        {
            nodes_dna_table.set(index, 1).expect("Nu kak to tak");
        }
    }

    new_table.set("nodesDna", nodes_dna_table).unwrap();

    new_table
}
