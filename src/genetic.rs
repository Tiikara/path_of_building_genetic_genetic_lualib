use std::{fs, thread, time};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::sleep;
use std::time::Duration;
use crossbeam::channel::{Receiver, Sender, unbounded};
use mlua::prelude::*;

use rand::prelude::ThreadRng;
use rand::Rng;
use typed_arena::Arena;
use crate::dna::{Dna, DnaData};
use crate::globals_data::{DNA_PROCESS, DNA_PROCESS_STATUS, DnaCommand, READER_DNA_QUEUE_CHANNEL, READER_DNA_RESULT_QUEUE_CHANNEL, WRITER_DNA_QUEUE_CHANNEL, WRITER_DNA_RESULT_QUEUE_CHANNEL};

pub struct DnaProcess {
    pub number: usize,
    pub target_normal_nodes_count: usize,
    pub target_ascendancy_nodes_count: usize
}

pub struct DnaProcessStatus {
    pub best_dna: Option<Box<Dna>>,
    pub best_dna_number: usize,
    pub is_progress: bool
}

struct LuaGeneticSolver
{
    pub static mut WRITER_DNA_QUEUE_CHANNEL: Option<Sender<*mut DnaCommand>> = None,
    pub static mut READER_DNA_QUEUE_CHANNEL: Option<Receiver<*mut DnaCommand>> = None,

    pub static mut WRITER_DNA_RESULT_QUEUE_CHANNEL: Option<Sender<i8>> = None,
    pub static mut READER_DNA_RESULT_QUEUE_CHANNEL: Option<Receiver<i8>> = None,
    workers_data: Arc<RwLock<DnaProcess>>,
    process_status: Arc<RwLock<DnaProcessStatus>>
}


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

    unsafe {
        DNA_PROCESS_STATUS = Some(Mutex::new(DnaProcessStatus{
            best_dna: None,
            best_dna_number: 0,
            is_progress: false
        }))
    }

    Ok(())
}


pub fn start_genetic_solver(
    _: &Lua,
    (max_generations_count,
        stop_generations_eps,
        count_generations_mutate_eps,
        population_max_generation_size,
        tree_nodes_count,
        mysteries_nodes_count): (usize, usize, usize, usize, usize, usize),
) -> LuaResult<()>
{
    if population_max_generation_size % 2 != 0
    {
        panic!("population_max_generation_size should be 2");
    }

    let dna_process_status = unsafe {
        match &DNA_PROCESS_STATUS {
            Some(dna_process_status) => dna_process_status,
            None => panic!("Dolbaeb")
        }
    };

    if dna_process_status.lock().unwrap().is_progress
    {
        panic!("Genetic solve already in progress");
    }

    let mut dna_process_status = dna_process_status.lock().unwrap();
    if dna_process_status.is_progress
    {
        panic!("Genetic solve already in progress");
    }

    dna_process_status.is_progress = true;

    dna_process_status.best_dna = None;
    dna_process_status.best_dna_number = 0;

    thread::spawn(move || {

        unsafe {
            DNA_PROCESS.target_normal_nodes_count = 98;
            DNA_PROCESS.target_ascendancy_nodes_count = 6;
            DNA_PROCESS.number += 1;
        }

        let dna_process_status = unsafe {
            match &DNA_PROCESS_STATUS {
                Some(dna_process_status) => dna_process_status,
                None => panic!("Dolbaeb")
            }
        };

        let mut dna_allocator = Vec::with_capacity(200000);
        for _ in 0..dna_allocator.capacity()
        {
            dna_allocator.push(Box::new(DnaData::new(tree_nodes_count, mysteries_nodes_count)));
        }

        let mut alloc_dna_commands: Vec<DnaCommand> = vec![Default::default(); 200000];
        let mut population = Vec::with_capacity(200000);
        let mut bastards = Vec::with_capacity(200000);
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
                {
                    let mut dna_process_status = dna_process_status.lock().unwrap();

                    dna_process_status.best_dna = Some(Box::new(
                        Dna {
                            reference: best_dna.reference.clone()
                        }
                    ));
                    dna_process_status.best_dna_number += 1;
                }
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
                let population_len = population.len();
                for dna in &mut population[1..population_len]
                {
                    for _ in 0..eps_steps
                    {
                        dna.mutate(&mut rng);
                    }
                }
            }
        }

        {
            dna_process_status.lock().unwrap().is_progress = false;
        }
    });

    sleep(Duration::from_secs(50));

    Ok(())
}

pub fn genetic_solver_get_best_dna(lua_context: &Lua, (): ()) -> LuaResult<LuaTable>
{
    let dna_process_status = unsafe {
        match &DNA_PROCESS_STATUS {
            Some(dna_process_status) => dna_process_status.lock().unwrap(),
            None => panic!("Dolbaeb")
        }
    };

    match &dna_process_status.best_dna {
        None => { panic!("Best dna is not exists") }
        Some(best_dna) => { Ok(create_table_dna_data_from_dna(lua_context, &*best_dna)) }
    }
}

pub fn genetic_solver_is_progress(_: &Lua, (): ()) -> LuaResult<bool>
{
    unsafe {
        match &DNA_PROCESS_STATUS {
            Some(dna_process_status) => Ok(dna_process_status.lock().unwrap().is_progress),
            None => panic!("Dolbaeb")
        }
    }
}

pub fn genetic_solver_get_best_dna_number(_: &Lua, (): ()) -> LuaResult<usize>
{
    unsafe {
        match &DNA_PROCESS_STATUS {
            Some(dna_process_status) => Ok(dna_process_status.lock().unwrap().best_dna_number),
            None => panic!("Dolbaeb")
        }
    }
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

    let mut effects_map = HashMap::new();
    for (index, nucl) in dna.body_mysteries.iter().enumerate() {
        let index_node = index / 6;

        if *nucl == 1
        {
            let effects_table =
                effects_map
                .entry(index_node)
                .or_insert_with(|| Box::new(lua_context.create_table().expect("Nu nihuya")));

            let index_effect = index % 6;

            effects_table.set(index_effect + 1, 1).unwrap();
        }
    }

    let mysteries_dna_table = lua_context.create_table().expect("Nu nihuya");
    for (index_node, effects_table) in effects_map.into_iter() {
        mysteries_dna_table.set(index_node + 1, *effects_table).unwrap();
    }

    new_table.set("treeNodesNumbers", nodes_dna_table).unwrap();
    new_table.set("mysteriesNodesEffectsInfo", mysteries_dna_table).unwrap();

    new_table
}
