use mlua::{Lua, Table};
use mlua::prelude::{LuaTable, LuaValue};
use crate::fitness_function_calculator::{FitnessFunctionCalculator, FitnessFunctionCalculatorStats};
use crate::target::Target;


#[derive(Clone)]
pub struct UserTarget
{
    pub stat: String,
    pub actor: String,
    pub weight: f64,
    pub target: f64,
    pub is_maximize: bool,
    pub lower_is_better: bool
}

impl Target for UserTarget
{
    fn clone_dyn(&self) -> Box<dyn Target> {
        Box::new(self.clone())
    }

    fn calc_fitness_score(&self, fitness_function_calculator: &FitnessFunctionCalculator, stats: &mut FitnessFunctionCalculatorStats) -> f64 {
        let stat = stats.try_get_stat(self.actor.clone(), self.stat.clone());

        if self.is_maximize
        {
            match stat {
                None => {
                    0.01
                }
                Some(stat_value) => {
                    stat_value
                }
            }
        }
        else
        {
            match stat {
                None => {
                    0.01
                }
                Some(stat_value) => {
                    fitness_function_calculator.calc_target_mul(stat_value, self.weight, self.target, self.lower_is_better)
                }
            }
        }
    }

    fn get_maximize_value(&self, stats: &mut FitnessFunctionCalculatorStats) -> f64 {
        let stat = stats.try_get_stat(self.actor.clone(), self.stat.clone());

        if self.lower_is_better
        {
            match stat {
                None => {
                    0.0
                }
                Some(stat_value) => {
                    -stat_value
                }
            }
        }
        else
        {
            match stat {
                None => {
                    0.0
                }
                Some(stat_value) => {
                    stat_value
                }
            }
        }
    }
}

pub fn create_targets_from_tables(targets_table: LuaTable, maximize_table: LuaTable) -> Vec<UserTarget>
{
    let mut targets = Vec::new();

    for entry_target in targets_table.pairs()
    {
        let (_, lua_target): (LuaValue, LuaTable) = entry_target.unwrap();

        let lower_is_better =
            match lua_target.get::<&str, Option<bool>>("lowerIsBetter").unwrap() {
                None => false,
                Some(lower_is_better) => lower_is_better
            };

        targets.push(UserTarget {
            stat: lua_target.get("stat").unwrap(),
            actor: lua_target.get("actor").unwrap(),
            weight: lua_target.get("weight").unwrap(),
            target: lua_target.get("target").unwrap(),
            is_maximize: false,
            lower_is_better
        });
    }

    for entry_target in maximize_table.pairs()
    {
        let (_, lua_target): (LuaValue, LuaTable) = entry_target.unwrap();

        let lower_is_better =
            match lua_target.get::<&str, Option<bool>>("lowerIsBetter").unwrap() {
                None => false,
                Some(lower_is_better) => lower_is_better
            };

        targets.push(UserTarget {
            stat: lua_target.get("stat").unwrap(),
            actor: lua_target.get("actor").unwrap(),
            weight: lua_target.get("weight").unwrap(),
            target: 0.0,
            is_maximize: true,
            lower_is_better
        });
    }

    targets
}

pub fn create_tables_from_targets<'lua>(lua: &'lua Lua, targets: &Vec<UserTarget>) -> (Table<'lua>, Table<'lua>)
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
