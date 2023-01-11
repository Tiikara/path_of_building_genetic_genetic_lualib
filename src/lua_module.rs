use mlua::Lua;
use mlua::prelude::{LuaResult, LuaTable};
use crate::genetic::{init_genetic_solver, start_genetic_solver};
use crate::worker::{worker_get_dna_process_number, worker_receive_next_command, worker_set_result_dna_fitness};

#[mlua::lua_module]
fn path_of_building_genetic_solver(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("InitGeneticSolver", lua.create_function(init_genetic_solver)?)?;
    exports.set("StartGeneticSolver", lua.create_function(start_genetic_solver)?)?;

    exports.set("WorkerReceiveNextCommand", lua.create_function(worker_receive_next_command)?)?;
    exports.set("WorkerSetResultDnaFitness", lua.create_function(worker_set_result_dna_fitness)?)?;

    exports.set("WorkerGetDnaProcessNumber", lua.create_function(worker_get_dna_process_number)?)?;
    Ok(exports)
}
