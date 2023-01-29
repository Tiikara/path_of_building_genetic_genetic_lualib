use std::borrow::{BorrowMut};
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::sync::{Arc, LockResult, RwLock};
use crossbeam::channel::{Receiver, Sender};
use mlua::{Function, Lua, LuaOptions, StdLib, UserData};
use mlua::prelude::{LuaMultiValue, LuaResult, LuaString, LuaTable, LuaValue};
use crate::dna::Dna;
use crate::dna_encoder::{create_dna_encoder, DnaEncoder};
use crate::fitness_function_calculator::{FitnessFunctionCalculator, FitnessFunctionCalculatorStats};

use crate::genetic::{DnaCommand, Session};
use crate::user_target::create_tables_from_targets;

#[derive(Clone)]
pub struct LuaDnaCommand
{
    pub reference: Rc<RefCell<Option<Box<DnaCommand>>>>
}

impl UserData for LuaDnaCommand {}

struct SessionProcessRuntime
{
    target_normal_nodes_count: usize,
    target_ascendancy_nodes_count: usize,
    dna_encoder: DnaEncoder,
    fitness_function_calculator: FitnessFunctionCalculator
}

pub fn worker_main(reader_dna_queue_channel: Receiver<Box<DnaCommand>>,
                   writer_dna_result_queue_channel: Sender<Box<DnaCommand>>,
                   session: Arc<RwLock<Session>>,
                   working_dir: &str
)
{
    let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default()).unwrap();

    let globals = lua.globals();

    let std_io_table = globals.get::<&str, LuaTable>("io").unwrap();
    let std_io_open_func = std_io_table.get::<&str, Function>("open").unwrap();

    std_io_table.set("original_open", std_io_open_func).unwrap();

    let working_dir_io_copy = String::from(working_dir);
    let working_dir_io = lua.create_function(move |lua_context, (file_name, mode): (LuaString, LuaString)| -> LuaResult<LuaMultiValue> {
        let file_name = working_dir_io_copy.clone() + file_name.to_str().unwrap().to_string().as_str();

        let globals = lua_context.globals();

        let std_io_table = globals.get::<&str, LuaTable>("io").unwrap();
        let std_io_open_func = std_io_table.get::<&str, Function>("original_open").unwrap();

        Ok(std_io_open_func.call((file_name, mode)).unwrap())
    }).unwrap();

    std_io_table.set("open", working_dir_io).unwrap();

    globals.set("ScriptAbsoluteWorkingDir", working_dir).unwrap();

    lua.load(&fs::read_to_string(String::from(working_dir) + "Classes/GeneticSolverWorker.lua").unwrap())
        .exec()
        .unwrap();

    let mut stored_session_number = 0;

    let lua_build: LuaTable = globals.get("build").unwrap();

    let calculate_stats_func = globals.get::<&str, Function>("GeneticWorkerCalculateStats").unwrap();
    let init_session_func = globals.get::<&str, Function>("GeneticWorkerInitializeSession").unwrap();

    let mut session_process_runtime: Option<Box<SessionProcessRuntime>> = None;

    loop {
        let mut dna_command = reader_dna_queue_channel.recv().unwrap();

        {
            let session = session.read().unwrap();

            if session.number != stored_session_number
            {
                let _: LuaValue = init_session_func.call(()).unwrap();

                stored_session_number = session.number;

                let target_normal_nodes_count = session.target_normal_nodes_count;
                let target_ascendancy_nodes_count = session.target_ascendancy_nodes_count;

                let dna_encoder = create_dna_encoder(&lua_build);

                let fitness_function_calculator =
                    FitnessFunctionCalculator::new(
                        session.targets.clone()
                    );

                session_process_runtime = Some(
                    Box::new(SessionProcessRuntime {
                        target_normal_nodes_count,
                        target_ascendancy_nodes_count,
                        dna_encoder,
                        fitness_function_calculator,
                    })
                );
            }
        };

        let mut iteration_session_runtime = session_process_runtime.take().unwrap();

        let dna_convert_result =
            iteration_session_runtime.dna_encoder.convert_dna_to_build(
                &lua_build,
                dna_command.dna.as_ref().unwrap(),
                iteration_session_runtime.target_normal_nodes_count,
                iteration_session_runtime.target_ascendancy_nodes_count);

        let stats_env: LuaTable = calculate_stats_func.call(()).unwrap();

        let mut stats = FitnessFunctionCalculatorStats::new(&stats_env);

        calculate_targets_for_dna(&iteration_session_runtime.fitness_function_calculator,
                                  &mut stats,
                                  dna_command.dna.as_mut().unwrap());

        writer_dna_result_queue_channel.send(dna_command).unwrap();

        session_process_runtime = Some(iteration_session_runtime);
    }
}

fn calculate_targets_for_dna(fitness_function_calculator: &FitnessFunctionCalculator, stats: &mut FitnessFunctionCalculatorStats, dna: &mut Dna)
{
    for (index_target, target) in fitness_function_calculator.targets.iter().enumerate()
    {
        dna.fitness_score_targets[index_target] = target.calc_fitness_score(&fitness_function_calculator, stats);
    }

    dna.fitness_score = fitness_function_calculator.calculate_and_get_fitness_score(stats);
}
