use mlua::{Lua, UserData};
use mlua::prelude::LuaTable;
use crate::genetic::create_table_from_dna;
use crate::globals_channels::READER_DNA_QUEUE_CHANNEL;

struct LuaDnaCommand {
     *mut DnaCommand
}

impl UserData for LuaDnaCommand {

}

fn receive_next_command(
    lua_context: &Lua
) -> LuaTable
{
    let reader_dna_queue_channel = unsafe {
        match &READER_DNA_QUEUE_CHANNEL {
            Some(reader_dna_queue_channel) => reader_dna_queue_channel,
            None => panic!("Queue is not initialized")
        }
    };

    let reader_dna_queue_channel = reader_dna_queue_channel.lock().unwrap();

    let dna_command = unsafe { &*reader_dna_queue_channel.recv().unwrap() };


    let res_table = lua_context.create_table().unwrap();


    match dna_command.dna {
        Some(dna) => res_table.set("dnaData", create_table_from_dna(lua_context, unsafe { &*dna })).unwrap(),
        None => {}
    };

    if let Some(reinit) = &dna_command.reinit
    {
        res_table.set("isReinit", true).unwrap();
        res_table.set("targetNormalNodesCount", reinit.target_normal_nodes_count).unwrap();
        res_table.set("targetAscendancyNodesCount", reinit.target_ascendancy_nodes_count).unwrap();
    }

    if dna_command.stop_thread {
        res_table.set("stopThread", dna_command.stop_thread).unwrap();
    }

    res_table
}


fn set_result_dna_fitness(lua_context: &Lua, fitness_score: f64)
{

}
