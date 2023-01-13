use std::{env, thread, time};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread::{JoinHandle};

use crossbeam::channel::{Receiver, Sender, unbounded};
use mlua::prelude::*;
use mlua::{UserData, UserDataMethods};

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::dna::{Dna, DnaData};
use crate::worker::worker_main;

pub struct DnaCommand {
    pub dna: Option<Dna>
}

pub struct Session {
    pub number: usize,
    pub target_normal_nodes_count: usize,
    pub target_ascendancy_nodes_count: usize
}

pub struct ProcessStatus {
    pub best_dna: Option<Dna>,
    pub best_dna_number: usize,
    pub is_progress: bool
}

pub struct LuaGeneticSolver
{
    pub writer_dna_queue_channel: Sender<Box<DnaCommand>>,
    pub reader_dna_queue_channel: Receiver<Box<DnaCommand>>,

    pub writer_dna_result_queue_channel: Sender<Box<DnaCommand>>,
    pub reader_dna_result_queue_channel: Receiver<Box<DnaCommand>>,

    pub session: Arc<RwLock<Session>>,
    pub process_status: Arc<RwLock<ProcessStatus>>,

    pub workers_was_created: bool,

    pub main_thread: Option<JoinHandle<()>>
}

impl UserData for LuaGeneticSolver {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("IsProgress", |_, this, ()| {
            Ok(this.process_status.read().unwrap().is_progress)
        });

        methods.add_method("GetBestDnaNumber", |_, this, ()| {
            Ok(this.process_status.read().unwrap().best_dna_number)
        });

        methods.add_method("GetBestDnaData", |lua_context, this, ()| {
            Ok(
                create_table_dna_data_from_dna(
                    lua_context,
                    &this.process_status.read().unwrap().best_dna.as_ref().unwrap()
                )
            )
        });

        methods.add_method_mut("CreateWorkers", |_lua_context, this, workers_count: usize| {
            if this.workers_was_created
            {
                panic!("Workers already created")
            }

            for _ in 0..workers_count
            {
                let reader_dna_queue_channel = this.reader_dna_queue_channel.clone();
                let writer_dna_result_queue_channel = this.writer_dna_result_queue_channel.clone();
                let workers_data = this.session.clone();

                let working_dir = String::from(env::current_dir().unwrap().to_str().unwrap()) + "/";

                thread::spawn(move || {
                    worker_main(reader_dna_queue_channel,
                                writer_dna_result_queue_channel,
                                workers_data,
                                &working_dir);
                });
            }

            this.workers_was_created = true;

            Ok(())
        });

        methods.add_method_mut("WaitSolve", |_lua_context, this, (): ()| {
            this.main_thread.take().expect("Solve process is not started").join().unwrap();

            Ok(())
        });

        methods.add_method_mut("StartSolve", |_lua_context, this, (
            max_generations_count,
            stop_generations_eps,
            count_generations_mutate_eps,
            population_max_generation_size,
            tree_nodes_count,
            mysteries_nodes_count,
            target_normal_nodes_count,
            target_ascendancy_nodes_count, is_wait): (usize, usize, usize, usize, usize, usize, usize, usize, bool)| {

            if population_max_generation_size % 2 != 0
            {
                panic!("population_max_generation_size should be 2");
            }

            {
                let mut process_status = this.process_status.write().unwrap();

                if process_status.is_progress
                {
                    panic!("Genetic solve already in progress");
                }

                process_status.is_progress = true;

                process_status.best_dna = None;
                process_status.best_dna_number = 0;

                let mut session_parameters = this.session.write().unwrap();

                session_parameters.target_normal_nodes_count = target_normal_nodes_count;
                session_parameters.target_ascendancy_nodes_count = target_ascendancy_nodes_count;
                session_parameters.number += 1;
            }

            let writer_dna_queue_channel = this.writer_dna_queue_channel.clone();
            let reader_dna_result_queue_channel = this.reader_dna_result_queue_channel.clone();
            let process_status = this.process_status.clone();
            let thread = thread::spawn(move || {
                genetic_solve(writer_dna_queue_channel,
                              reader_dna_result_queue_channel,
                              process_status,
                              max_generations_count,
                              stop_generations_eps,
                              count_generations_mutate_eps,
                              population_max_generation_size,
                              tree_nodes_count,
                              mysteries_nodes_count)
            });

            this.main_thread = Some(thread);

            Ok(())
        });
    }
}

pub fn create_genetic_solver(_: &Lua, (): ()) -> LuaResult<LuaGeneticSolver> {
    let (writer_dna_queue_channel, reader_dna_queue_channel) =
        unbounded();

    let (writer_dna_result_queue_channel, reader_dna_result_queue_channel) =
        unbounded();

    Ok(LuaGeneticSolver {
        writer_dna_queue_channel,
        reader_dna_queue_channel,
        writer_dna_result_queue_channel,
        reader_dna_result_queue_channel,
        session: Arc::new(RwLock::new(Session {
            number: 0,
            target_ascendancy_nodes_count: 0,
            target_normal_nodes_count: 0
        })),
        process_status: Arc::new(RwLock::new(ProcessStatus {
            best_dna: None,
            best_dna_number: 0,
            is_progress: false
        })),
        main_thread: None,
        workers_was_created: false
    })
}

pub fn genetic_solve(writer_dna_queue_channel: Sender<Box<DnaCommand>>,
                     reader_dna_result_queue_channel: Receiver<Box<DnaCommand>>,
                     process_status: Arc<RwLock<ProcessStatus>>,
                     max_generations_count: usize,
                     stop_generations_eps: usize,
                     count_generations_mutate_eps: usize,
                     population_max_generation_size: usize,
                     tree_nodes_count: usize,
                     mysteries_nodes_count: usize)
{
    let mut dna_allocator = Vec::with_capacity(200000);
    for _ in 0..dna_allocator.capacity()
    {
        dna_allocator.push(Box::new(DnaData::new(tree_nodes_count, mysteries_nodes_count)));
    }

    let mut dna_command_allocator = Vec::with_capacity(200000);
    for _ in 0..dna_command_allocator.capacity()
    {
        dna_command_allocator.push(Box::new(DnaCommand {
            dna: None
        }));
    }

    let mut population = Vec::with_capacity(200000);
    let mut bastards = Vec::with_capacity(200000);
    let mut rng = rand::thread_rng();

    for index_node in 0..tree_nodes_count {
        let mut dna = Dna::new(&mut dna_allocator);

        dna.body_nodes[index_node] = 1;

        population.push(dna);
    }

    let population_len = population.len();
    calc_fitness_with_worker(
        &writer_dna_queue_channel,
        &reader_dna_result_queue_channel,
        &mut dna_command_allocator,
        &mut population,
        population_len,
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
            &writer_dna_queue_channel,
            &reader_dna_result_queue_channel,
            &mut dna_command_allocator,
            &mut population,
            population_len - start_mutated_index,
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
            &writer_dna_queue_channel,
            &reader_dna_result_queue_channel,
            &mut dna_command_allocator,
            &mut bastards,
            bastards_len,
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
                let mut process_status = process_status.write().unwrap();

                process_status.best_dna = Some(
                    Dna {
                        reference: best_dna.reference.clone()
                    }
                );
                process_status.best_dna_number += 1;
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
        process_status.write().unwrap().is_progress = false;
    }
}

fn calc_fitness_with_worker(writer_dna_queue_channel: &Sender<Box<DnaCommand>>,
                            reader_dna_result_queue_channel: &Receiver<Box<DnaCommand>>,
                            dna_commands_allocator: &mut Vec<Box<DnaCommand>>,
                            dnas: &mut Vec<Dna>,
                            calc_count_from_end: usize
)
{
    for _ in 0..calc_count_from_end
    {
        let dna = dnas.pop().unwrap();
        let mut dna_command = dna_commands_allocator.pop().unwrap();

        dna_command.dna = Some(dna);

        writer_dna_queue_channel.send(dna_command).unwrap();
    }

    for _ in 0..calc_count_from_end {
        let mut dna_command = reader_dna_result_queue_channel.recv().expect("Cannot receive dna result signal");

        let dna = dna_command.dna.take().unwrap();

        dnas.push(dna);
        dna_commands_allocator.push(dna_command);
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


pub fn create_table_dna_data_from_dna<'a, 'b>(lua_context: &'a Lua, dna: &'b Dna) -> LuaTable<'a>
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
