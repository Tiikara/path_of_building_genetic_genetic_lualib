use std::{fs, panic};
use mlua::Lua;
use mlua::prelude::{LuaResult, LuaTable};
use crate::genetic::{create_genetic_solver};

#[mlua::lua_module]
fn path_of_building_genetic_solver(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("CreateGeneticSolver", lua.create_function(create_genetic_solver)?)?;

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);

        fs::write("rust_panic.txt", panic_info.to_string()).expect("Unable to write file");
    }));

    Ok(exports)
}
