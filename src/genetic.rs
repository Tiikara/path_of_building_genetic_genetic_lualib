use std::{fs, thread, time};
use crossbeam::channel::{Receiver, Sender, unbounded};
use mlua::prelude::*;

use rand::prelude::ThreadRng;
use rand::Rng;
use typed_arena::Arena;
use crate::dna::{Dna, DnaData};
use crate::globals_channels::{DNA_PROCESS, DnaCommand, READER_DNA_QUEUE_CHANNEL, READER_DNA_RESULT_QUEUE_CHANNEL, WRITER_DNA_QUEUE_CHANNEL, WRITER_DNA_RESULT_QUEUE_CHANNEL};


pub fn init_genetic_solver(_: &Lua, (): ()) -> LuaResult<()> {
    let (writer_dna_queue_channel, reader_dna_queue_channel) =
        unbounded();

    unsafe {
        WRITER_DNA_QUEUE_CHANNEL = Some(writer_dna_queue_channel);
        READER_DNA_QUEUE_CHANNEL = Some(reader_dna_queue_channel);
    }

    let (writer_dna_result_queue_channel, reader_dna_result_queue_channel) =
        unbounded();

    unsafe {
        WRITER_DNA_RESULT_QUEUE_CHANNEL = Some(writer_dna_result_queue_channel);
        READER_DNA_RESULT_QUEUE_CHANNEL = Some(reader_dna_result_queue_channel);
    }

    Ok(())
}


pub fn start_genetic_solver(
    lua_context: &Lua,
    (max_generations_count,
        stop_generations_eps,
        count_generations_mutate_eps,
        population_max_generation_size,
        tree_nodes_count,
        mysteries_nodes_count): (usize, usize, usize, usize, usize, usize),
) -> LuaResult<LuaTable>
{
    if population_max_generation_size % 2 != 0
    {
        panic!("population_max_generation_size should be 2");
    }

    unsafe {
        DNA_PROCESS.target_normal_nodes_count = 98;
        DNA_PROCESS.target_ascendancy_nodes_count = 6;
        DNA_PROCESS.number += 1;
    }

    let mut dna_allocator = Vec::with_capacity(20000);
    for _ in 0..20000
    {
        dna_allocator.push(Box::new(DnaData::new(tree_nodes_count, mysteries_nodes_count)));
    }

    let mut alloc_dna_commands: Vec<DnaCommand> = vec![Default::default(); 20000];
    let mut population = Vec::with_capacity(20000);
    let mut bastards = Vec::with_capacity(20000);
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

    for index_node in 0..tree_nodes_count {
        let mut dna = Dna::new(&mut dna_allocator);

        dna.body_nodes[index_node] = 1;

        population.push(dna);
    }

    let population_len = population.len();

    calc_fitness_with_worker(
        writer_dna_queue_channel,
        reader_dna_result_queue_channel,
        &mut alloc_dna_commands,
        &mut population[0..population_len],
    );

    population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

    let mut best_dna = population[0].clone(&mut dna_allocator);

    let mut count_generations_with_best = 1;

    for _ in 1..=max_generations_count {
        let start_mutated_index = population.len();

        for i in 0..population.len() {
            let mut mutated_dna = population[i].clone(&mut dna_allocator);

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
            &mut dna_allocator,
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

        for _ in population_max_generation_size / 2..population.len()
        {
            let dna_to_remove = population.pop().unwrap();
            dna_allocator.push(dna_to_remove.reference);
        }

        while let Some(bastard) = bastards.pop() {
            population.push(bastard);
        }

        population.sort_unstable_by(|a, b| b.fitness_score.total_cmp(&a.fitness_score));

        if population[0].fitness_score > best_dna.fitness_score
        {
            best_dna = population[0].clone(&mut dna_allocator);
            count_generations_with_best = 1;
        } else {
            count_generations_with_best += 1;
        }

        if count_generations_with_best == stop_generations_eps
        {
            break;
        }

        if count_generations_with_best % count_generations_mutate_eps == 0
        {
            let eps_steps = count_generations_with_best / count_generations_mutate_eps;
            let mutation_count = 1 + (eps_steps - 1) * 5;
            for dna in &mut population
            {
                for _ in 0..mutation_count
                {
                    dna.mutate(&mut rng);
                }
            }
        }
    }

    Ok(create_table_dna_data_from_dna(lua_context, &best_dna))
}

fn calc_fitness_with_worker(writer_dna_queue_channel: &Sender<*mut DnaCommand>,
                            reader_dna_result_queue_channel: &Receiver<i8>,
                            alloc_dna_commands: &mut Vec<DnaCommand>,
                            dnas: &mut [Dna])
{
    for (i, dna) in dnas.iter_mut().enumerate() {
        let dna_command = &mut alloc_dna_commands[i];

        dna_command.dna = Some(&mut *dna);

        writer_dna_queue_channel.send(&mut alloc_dna_commands[i]).expect("Cannot send dna to queue");
    }

    for _ in 0..dnas.len() {
        reader_dna_result_queue_channel.recv().expect("Cannot receive dna result signal");
    }
}

fn make_hard_fuck(dna_data_allocator: &mut Vec<Box<DnaData>>, dna_masters: &[Dna], dna_slaves: &[Dna], out_bastards: &mut Vec<Dna>, rng: &mut ThreadRng)
{
    for dna_master in dna_masters {
        let index_of_slave = rng.gen_range(0..dna_slaves.len());
        let dna_slave = &dna_slaves[index_of_slave];

        out_bastards.push(dna_master.combine(dna_data_allocator, dna_slave, rng));
    }
}


pub fn create_table_dna_data_from_dna<'a>(lua_context: &'a Lua, dna: &Dna) -> LuaTable<'a>
{
    let new_table = lua_context.create_table().expect("Nu nihuya");

    let nodes_dna_table = lua_context.create_table().expect("Nu nihuya");
    for (index, nucl) in dna.body_nodes.iter().enumerate() {
        if *nucl == 1
        {
            nodes_dna_table.set(index + 1, 1).expect("Nu kak to tak");
        }
    }

    let mysteries_dna_table = lua_context.create_table().expect("Nu nihuya");

    for (index, nucl) in dna.body_mysteries.iter().enumerate() {
        mysteries_dna_table.set(index + 1, (*nucl as f64) / 255.0).expect("Nu kak to tak");
    }

    new_table.set("treeNodesNumbers", nodes_dna_table).unwrap();
    new_table.set("mysteriesIndexes", mysteries_dna_table).unwrap();

    new_table
}
