use mlua::{Lua, UserData};
use mlua::prelude::{LuaResult, LuaTable};
use crate::genetic::create_table_from_dna;
use crate::globals_channels::{DnaCommand, READER_DNA_QUEUE_CHANNEL, WRITER_DNA_RESULT_QUEUE_CHANNEL};

#[derive(Clone)]
pub struct LuaDnaCommand {
     ptr: *mut DnaCommand
}

impl UserData for LuaDnaCommand {}

pub fn worker_receive_next_command(
    lua_context: &Lua,
    (): ()
) -> LuaResult<LuaTable>
{
    let reader_dna_queue_channel = unsafe {
        match &READER_DNA_QUEUE_CHANNEL {
            Some(reader_dna_queue_channel) => reader_dna_queue_channel,
            None => panic!("Queue is not initialized")
        }
    };

    let reader_dna_queue_channel = reader_dna_queue_channel.lock().unwrap();

    let dna_command_ptr = reader_dna_queue_channel.recv().unwrap();

    let dna_command = unsafe { &*dna_command_ptr };


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

    res_table.set("dnaCommandHandler", LuaDnaCommand {
        ptr: dna_command_ptr
    }).unwrap();

    Ok(res_table)
}


pub fn worker_set_result_dna_fitness(_: &Lua, (dna_command_handler, fitness_score): (LuaDnaCommand, f64)) -> LuaResult<()>
{
    let dna_command = unsafe { &*dna_command_handler.ptr };

    let dna =
        match dna_command.dna
        {
            Some(dna) => unsafe { &mut *dna },
            None => panic!("Dna is not exists")
        };

    dna.fitness_score = fitness_score;

    let writer_dna_result_queue_channel = unsafe {
        match &WRITER_DNA_RESULT_QUEUE_CHANNEL {
            Some(writer_dna_result_queue_channel) => writer_dna_result_queue_channel,
            None => panic!("Queue is not initialized")
        }
    };

    let writer_dna_result_queue_channel =
        writer_dna_result_queue_channel.lock().unwrap().clone();

    writer_dna_result_queue_channel.send(1).unwrap();

    Ok(())
}
