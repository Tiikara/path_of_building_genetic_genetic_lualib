use std::borrow::Borrow;
use mlua::{Lua, Table, UserData};
use mlua::prelude::{LuaTable, LuaValue};

#[derive(Clone)]
pub struct Target
{
    pub stat: String,
    pub actor: String,
    pub weight: f64,
    pub target: f64,
    pub is_maximize: bool
}


pub fn create_targets_from_tables(targets_table: LuaTable, maximize_table: LuaTable) -> Vec<Target>
{
    let mut targets = Vec::new();

    for entry_target in targets_table.pairs()
    {
        let (_, lua_target): (LuaValue, LuaTable) = entry_target.unwrap();

        targets.push(Target {
            stat: lua_target.get("stat").unwrap(),
            actor: lua_target.get("actor").unwrap(),
            weight: lua_target.get("weight").unwrap(),
            target: lua_target.get("target").unwrap(),
            is_maximize: false
        });
    }

    for entry_target in maximize_table.pairs()
    {
        let (_, lua_target): (LuaValue, LuaTable) = entry_target.unwrap();

        targets.push(Target {
            stat: lua_target.get("stat").unwrap(),
            actor: lua_target.get("actor").unwrap(),
            weight: lua_target.get("weight").unwrap(),
            target: 0.0,
            is_maximize: true
        });
    }

    targets
}

pub fn create_tables_from_targets<'lua>(lua: &'lua Lua, targets: &Vec<Target>) -> (Table<'lua>, Table<'lua>)
{
    let targets_table = lua.create_table().unwrap();
    let maximizes_table = lua.create_table().unwrap();

    let mut count_targets = 0;
    let mut count_maximizes = 0;

    for target in targets
    {
        if target.is_maximize
        {
            let maximize_table = lua.create_table().unwrap();

            maximize_table.set("stat", target.stat.clone()).unwrap();
            maximize_table.set("weight", target.weight).unwrap();
            maximize_table.set("actor", target.actor.clone()).unwrap();

            count_maximizes += 1;
            maximizes_table.set(count_maximizes, maximize_table).unwrap();
        }
        else
        {
            let target_table = lua.create_table().unwrap();

            target_table.set("stat", target.stat.clone()).unwrap();
            target_table.set("weight", target.weight).unwrap();
            target_table.set("actor", target.actor.clone()).unwrap();
            target_table.set("target", target.target).unwrap();

            count_targets += 1;
            targets_table.set(count_targets, target_table).unwrap();
        }
    }

    (targets_table, maximizes_table)
}
