use std::{env, thread};
use std::collections::{HashSet};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{JoinHandle};

use crossbeam::channel::{Receiver, Sender, unbounded};
use mlua::prelude::*;
use mlua::{UserData, UserDataMethods};

use rand::prelude::{ThreadRng};
use rand::{thread_rng};
use crate::auto_targets::{AutoTargetFromStatToStat, AutoTargetManaCost, AutoTargetManaRegen};

use crate::dna::{Dna, DnaData, LuaDna};
use crate::mo::{Constraint, Meta, Objective, Ratio, Solution, SolutionsRuntimeProcessor};
use crate::mo::evaluator::DefaultEvaluator;
use crate::mo::optimizer::Optimizer;
use crate::mo::optimizers::nsga2::NSGA2Optimizer;
use crate::target::Target;
use crate::user_target::{create_targets_from_tables};
use crate::worker::worker_main;

pub struct DnaCommand {
    pub dna: Option<Dna>
}

pub struct Session {
    pub number: usize,
    pub target_normal_nodes_count: usize,
    pub target_ascendancy_nodes_count: usize,
    pub targets: Vec<Box<dyn Target>>
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

    pub current_generation_number: Arc<AtomicU64>,

    pub workers_was_created: bool,

    pub main_thread: Option<JoinHandle<()>>,

    pub is_received_stop_request: Arc<AtomicBool>
}

impl<'a> Debug for Dna {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Solution for Dna {
    fn crossover(&mut self, other: &mut Self)
    {
        let mut tmp_dna1 = self.combine(other);
        let mut tmp_dna2 = self.combine(other);

        std::mem::swap(self, &mut tmp_dna1);
        std::mem::swap(other, &mut tmp_dna2);
    }

    fn mutate(&mut self)
    {
        self.mutate()
    }
}

struct SolutionsRuntimeDnaProcessor
{
    writer_dna_queue_channel: Sender<Box<DnaCommand>>,
    reader_dna_result_queue_channel: Receiver<Box<DnaCommand>>,
    process_status: Arc<RwLock<ProcessStatus>>,
    current_generation_number: Arc<AtomicU64>,
    is_received_stop_request: Arc<AtomicBool>,
    best_solution_fitness: f64
}

impl SolutionsRuntimeProcessor<Dna> for SolutionsRuntimeDnaProcessor
{
    fn new_candidates(&mut self, mut dnas: Vec<&mut Dna>) {
        for dna in dnas.iter_mut()
        {
            let mut new_dna = Dna::new(DnaData::new(1, 1, 1, 1));

            std::mem::swap(&mut new_dna, *dna);

            let mut dna_command = DnaCommand {
                dna: Some(new_dna)
            };

            self.writer_dna_queue_channel.send(Box::new(dna_command)).unwrap();
        }

        for dna in dnas
        {
            let mut dna_command = self.reader_dna_result_queue_channel.recv().expect("Cannot receive dna result signal");

            let mut dna_from_command = dna_command.dna.take().unwrap();

            std::mem::swap(&mut dna_from_command, dna);
        }
    }

    fn iter_solutions(&mut self, candidates: Vec<&mut Dna>) {
        for dna in candidates
        {
            if dna.fitness_score > self.best_solution_fitness
            {
                {
                    let mut process_status = self.process_status.write().unwrap();

                    process_status.best_dna = Some(
                        Dna {
                            reference: dna.reference.clone()
                        }
                    );
                    process_status.best_dna_number += 1;
                }

                self.best_solution_fitness = dna.fitness_score;
            }
        }
    }

    fn iteration_num(&mut self, num: usize) {
        self.current_generation_number.store(num as u64, Ordering::SeqCst);
    }

    fn needs_early_stop(&mut self) -> bool {
        self.is_received_stop_request.load(Ordering::SeqCst)
    }
}

pub struct FitnessScoreObjective
{

}

impl<'a> Objective<Dna> for FitnessScoreObjective {
    fn value(&self, candidate: &Dna) -> f64 {
        -candidate.fitness_score
    }

    fn good_enough(&self, val: f64) -> bool {
        false
    }
}

pub struct TargetObjective {
    target_index: usize
}

impl<'a> Objective<Dna> for TargetObjective {
    fn value(&self, candidate: &Dna) -> f64 {
        -candidate.fitness_score_targets[self.target_index]
    }

    fn good_enough(&self, val: f64) -> bool {
        false
    }
}

struct Params<'a> {
    population_max_generation_size: usize,
    tree_nodes_count: usize,
    masteries_nodes_count: usize,
    max_nodes_count: usize,
    targets_count: usize,
    rng: &'a mut ThreadRng,
    objectives: Vec<Box<dyn Objective<Dna>>>,
    constraints: Vec<Box<dyn Constraint<Dna>>>,
}

impl<'a> Meta<'a, Dna> for Params<'a> {
    fn population_size(&self) -> usize {
        self.population_max_generation_size
    }

    fn crossover_odds(&self) -> &'a Ratio {
        &Ratio(1, 1)
    }

    fn mutation_odds(&self) -> &'a Ratio {
        &Ratio(1, 1)
    }

    fn random_solution(&mut self) -> Dna {
        Dna::new(
            DnaData::new(self.tree_nodes_count,
                         self.masteries_nodes_count,
                         self.targets_count,
                         self.max_nodes_count)
        )
    }

    fn objectives(&self) -> &Vec<Box<dyn Objective<Dna>>> {
        &self.objectives
    }

    fn constraints(&self) -> &Vec<Box<dyn Constraint<Dna>>> {
        &self.constraints
    }
}

impl UserData for LuaGeneticSolver {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("IsProgress", |_, this, ()| {
            Ok(this.process_status.read().unwrap().is_progress)
        });

        methods.add_method("GetBestDnaNumber", |_, this, ()| {
            Ok(this.process_status.read().unwrap().best_dna_number)
        });

        methods.add_method("GetCurrentGenerationNumber", |_, this, ()| {
            Ok(this.current_generation_number.load(Ordering::SeqCst))
        });

        methods.add_method("GetBestDna", |_lua_context, this, ()| {
            Ok(
                LuaDna {
                    reference: Rc::new(
                        Dna {
                            reference: this.process_status.read().unwrap().best_dna.as_ref().unwrap().reference.clone()
                        }
                    )
                }
            )
        });

        methods.add_method_mut("StopSolve", |_lua_context, this, (): ()| {

            let process_status = this.process_status.read().unwrap();

            if process_status.is_progress == false
            {
                panic!("Solve is not in progress");
            }

            this.is_received_stop_request.store(true, Ordering::SeqCst);

            Ok(())
        });

        methods.add_method_mut("CreateWorkers", |_lua_context, this, workers_count: Option<usize>| {
            if this.workers_was_created
            {
                panic!("Workers already created")
            }

            let workers_count =
                match workers_count {
                    None => {
                        let cpus = num_cpus::get();

                        // Allocate workers based on cpu
                        // 1 thread for main genetic thread, 2 for PoB UI
                        if cpus <= 3
                        {
                            1
                        }
                        else
                        {
                            cpus - 2
                        }
                    }
                    Some(workers_count) => workers_count
                };

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
            stop_generations_eps,
            population_max_generation_size,
            tree_nodes_count,
            masteries_nodes_count,
            target_normal_nodes_count,
            target_ascendancy_nodes_count,
            targets_table,
            maximizes_table
        ): (usize, usize, usize, usize, usize, usize, LuaTable, LuaTable)| {

            if population_max_generation_size % 2 != 0
            {
                panic!("population_max_generation_size should be 2");
            }

            let targets_count =
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

                    let user_targets = create_targets_from_tables(targets_table, maximizes_table);

                    session_parameters.targets.clear();

                    for user_target in user_targets
                    {
                        session_parameters.targets.push(Box::new(user_target));
                    }

                    session_parameters.targets.push(Box::new(AutoTargetManaCost{}));
                    session_parameters.targets.push(Box::new(AutoTargetManaRegen{}));
                    session_parameters.targets.push(Box::new(AutoTargetFromStatToStat{
                        target_stat_name: String::from("ReqStr"),
                        current_stat_name: String::from("Str"),
                    }));
                    session_parameters.targets.push(Box::new(AutoTargetFromStatToStat{
                        target_stat_name: String::from("ReqInt"),
                        current_stat_name: String::from("Int"),
                    }));
                    session_parameters.targets.push(Box::new(AutoTargetFromStatToStat{
                        target_stat_name: String::from("ReqDex"),
                        current_stat_name: String::from("Dex"),
                    }));

                    this.is_received_stop_request.store(false, Ordering::SeqCst);

                    this.current_generation_number.store(0, Ordering::SeqCst);

                    session_parameters.targets.len()
                };

            // Drain all current messages from previous iterations
            while this.reader_dna_queue_channel.try_recv().is_ok() {}
            while this.reader_dna_result_queue_channel.try_recv().is_ok() {}

            let writer_dna_queue_channel = this.writer_dna_queue_channel.clone();
            let reader_dna_result_queue_channel = this.reader_dna_result_queue_channel.clone();
            let process_status = this.process_status.clone();
            let is_received_stop_request = this.is_received_stop_request.clone();
            let current_generation_number = this.current_generation_number.clone();
            let thread = thread::spawn(move || {
                genetic_solve(writer_dna_queue_channel,
                              reader_dna_result_queue_channel,
                              process_status,
                              is_received_stop_request,
                              current_generation_number,
                              stop_generations_eps,
                              population_max_generation_size,
                              tree_nodes_count,
                              masteries_nodes_count,
                              targets_count,
                              target_normal_nodes_count + target_ascendancy_nodes_count)
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
            target_normal_nodes_count: 0,
            targets: vec![]
        })),
        process_status: Arc::new(RwLock::new(ProcessStatus {
            best_dna: None,
            best_dna_number: 0,
            is_progress: false
        })),
        main_thread: None,
        workers_was_created: false,
        is_received_stop_request: Arc::new(AtomicBool::new(false)),
        current_generation_number: Arc::new(Default::default()),
    })
}

pub fn genetic_solve(writer_dna_queue_channel: Sender<Box<DnaCommand>>,
                     reader_dna_result_queue_channel: Receiver<Box<DnaCommand>>,
                     process_status: Arc<RwLock<ProcessStatus>>,
                     is_received_stop_request: Arc<AtomicBool>,
                     current_generation_number: Arc<AtomicU64>,
                     stop_generations_eps: usize,
                     population_max_generation_size: usize,
                     tree_nodes_count: usize,
                     masteries_nodes_count: usize,
                     targets_count: usize,
                     target_nodes_count: usize)
{
    let mut objectives: Vec<Box<dyn Objective<Dna>>> = Vec::new();

    for target_index in 0..targets_count
    {
        objectives.push(Box::new(TargetObjective {
            target_index
        }));
    }

    objectives.push(Box::new(FitnessScoreObjective{}));

    let meta = Params {
        population_max_generation_size,
        tree_nodes_count,
        masteries_nodes_count,
        max_nodes_count: target_nodes_count,
        targets_count,
        rng: &mut thread_rng(),
        objectives,
        constraints: vec![]
    };

    let mut optimizer = NSGA2Optimizer::new(meta);
    optimizer
        .optimize(
            Box::new(DefaultEvaluator::new(stop_generations_eps)),
            Box::new(SolutionsRuntimeDnaProcessor {
                writer_dna_queue_channel,
                reader_dna_result_queue_channel,
                process_status: process_status.clone(),
                current_generation_number,
                is_received_stop_request: is_received_stop_request.clone(),
                best_solution_fitness: -1.0
            })
        );

    {
        process_status.write().unwrap().is_progress = false;
        is_received_stop_request.store(false, Ordering::SeqCst);
    }
}

