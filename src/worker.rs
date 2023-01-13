use std::borrow::{BorrowMut};
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use crossbeam::channel::{Receiver, Sender};
use mlua::{Function, Lua, LuaOptions, StdLib, UserData};
use mlua::prelude::{LuaFunction, LuaMultiValue, LuaResult, LuaString, LuaTable};

use crate::genetic::{create_table_dna_data_from_dna, DnaCommand, Session};

#[derive(Clone)]
struct LuaDnaCommand
{
    reference: Rc<RefCell<Option<Box<DnaCommand>>>>
}

impl UserData for LuaDnaCommand {}

pub fn worker_main(reader_dna_queue_channel: Receiver<Box<DnaCommand>>,
                   writer_dna_result_queue_channel: Sender<Box<DnaCommand>>,
                   session: Arc<RwLock<Session>>,
                   working_dir: &str
)
{
    let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default()).unwrap();

    let worker_reader_dna_queue_channel = reader_dna_queue_channel.clone();
    let worker_receive_next_command_func = lua.create_function(move |lua_context, (): ()| {
        let dna_command = worker_reader_dna_queue_channel.recv().unwrap();

        let res_table = lua_context.create_table().unwrap();

        match &dna_command.dna {
            Some(dna) => res_table.set("dnaData", create_table_dna_data_from_dna(lua_context, dna)).unwrap(),
            None => panic!("Dna is not exists")
        }

        res_table.set("handler", LuaDnaCommand { reference: Rc::new(RefCell::new(Some(dna_command))) }).unwrap();

        Ok(res_table)
    }).unwrap();

    let worker_set_result_func = lua.create_function(move |_lua_context, (mut dna_command, fitness_score): (LuaDnaCommand, f64)| {
        let dna_command = dna_command.reference.borrow_mut();

        let mut dna_command = dna_command.take().unwrap();

        match &mut dna_command.dna
        {
            Some(dna) => dna.fitness_score = fitness_score,
            None => panic!("Dna is not present in dna command")
        }

        writer_dna_result_queue_channel.send(dna_command).unwrap();

        Ok(())
    }).unwrap();

    let get_session_session = session.clone();
    let worker_get_session_number_func = lua.create_function(move |_lua_context, (): ()| {
        Ok(get_session_session.read().unwrap().number)
    }).unwrap();

    let worker_get_session_parameters_func = lua.create_function(move |lua_context, (): ()| {

        let (target_normal_nodes_count, target_ascendancy_nodes_count) =
            {
                let session = session.read().unwrap();

                (session.target_normal_nodes_count, session.target_ascendancy_nodes_count)
            };

        let res_table = lua_context.create_table().unwrap();

        res_table.set("targetNormalNodesCount", target_normal_nodes_count).unwrap();
        res_table.set("targetAscendancyNodesCount", target_ascendancy_nodes_count).unwrap();

        Ok(res_table)
    }).unwrap();

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

    globals.set("GeneticWorkerReceiveNextCommand", worker_receive_next_command_func).unwrap();
    globals.set("GeneticWorkerSetResultToHandler", worker_set_result_func).unwrap();

    globals.set("GeneticWorkerGetSessionNumber", worker_get_session_number_func).unwrap();
    globals.set("GeneticWorkerGetSessionParameters", worker_get_session_parameters_func).unwrap();

    globals.set("ScriptAbsoluteWorkingDir", working_dir).unwrap();

    lua.load(&fs::read_to_string(String::from(working_dir) + "Classes/GeneticSolverWorker.lua").unwrap())
        .exec()
        .unwrap();

    lua.load(r#"
        GeneticSolverWorker()
    "#)
        .exec()
        .unwrap();
}
