use std::cmp::min;
use std::collections::HashMap;
use std::fs;
use mlua::prelude::LuaTable;
use mlua::Table;
use crate::targets::Target;

const USED_NODE_COUNT_WEIGHT: f64 = 5.0;
const USED_NODE_COUNT_FACTOR: f64 = 0.0005;
const CSV_WEIGHT_MULTIPLIER: f64 = 10.0;

pub struct FitnessFunctionCalculator
{
    targets: Vec<Target>,
    target_normal_nodes_count: f64,
    target_ascendancy_nodes_count: f64
}

impl FitnessFunctionCalculator
{
    pub fn new(target_normal_nodes_count: usize, target_ascendancy_nodes_count: usize, targets: Vec<Target>) -> Self
    {
        FitnessFunctionCalculator{
            targets,
            target_normal_nodes_count: target_normal_nodes_count as f64,
            target_ascendancy_nodes_count: target_ascendancy_nodes_count as f64,
        }
    }

    pub fn calculate_and_get_fitness_score(&self, stats_env: &LuaTable, used_normal_node_count: usize, used_ascendancy_node_count: usize) -> f64
    {
        let mut actor_outputs = HashMap::with_capacity(2);

        let mut csvs = 1.0;

        let used_normal_node_count = used_normal_node_count as f64;
        let used_ascendancy_node_count = used_ascendancy_node_count as f64;

        if used_normal_node_count > self.target_normal_nodes_count
        {
            csvs *= self.calc_scv(2.0 * self.target_normal_nodes_count - used_normal_node_count, USED_NODE_COUNT_WEIGHT, self.target_normal_nodes_count);
        }
        else if used_normal_node_count < self.target_normal_nodes_count {
            csvs *= 1.0 + USED_NODE_COUNT_FACTOR * (self.target_normal_nodes_count + 1.0 - used_normal_node_count).ln()
        }

        if used_ascendancy_node_count > self.target_ascendancy_nodes_count
        {
            csvs *= self.calc_scv(2.0 * self.target_ascendancy_nodes_count - used_ascendancy_node_count, USED_NODE_COUNT_WEIGHT, self.target_ascendancy_nodes_count);
        }
        else if used_ascendancy_node_count < self.target_ascendancy_nodes_count {
            csvs *= 1.0 + USED_NODE_COUNT_FACTOR * (self.target_ascendancy_nodes_count + 1.0 - used_ascendancy_node_count).ln()
        }

        for target in &self.targets
        {
            let actor_output_table = actor_outputs
                .entry(target.actor.as_str())
                .or_insert_with(|| {
                    let actor_table = stats_env.get::<&str, LuaTable>(target.actor.as_str()).unwrap();

                    actor_table.get::<&str, LuaTable>("output").unwrap()
                });

            let stat_value: Option<f64> = actor_output_table.get(target.stat.as_str()).unwrap();

            if target.is_maximize
            {
                match stat_value {
                    None => {
                        csvs *= 0.01;
                    }
                    Some(stat_value) => {
                        csvs *= stat_value * target.weight;
                    }
                }
            }
            else
            {
                match stat_value {
                    None => {
                        csvs *= 0.01;
                    }
                    Some(stat_value) => {
                        csvs *= self.calc_scv(stat_value, target.weight, target.target);
                    }
                }
            }
        }

        let player_output_table = actor_outputs
            .entry("player")
            .or_insert_with(|| {
                let actor_table = stats_env.get::<&str, LuaTable>("player").unwrap();

                actor_table.get::<&str, LuaTable>("output").unwrap()
            });

        let mut mana_recovery_sum = 0.0;

        match player_output_table.get::<&str, Option<f64>>("ManaRegenRecovery").unwrap() {
            None => {},
            Some(mana_regen_recovery) => {
                mana_recovery_sum += mana_regen_recovery;
            }
        }

        match player_output_table.get::<&str, Option<f64>>("ManaLeechGainRate").unwrap() {
            None => {},
            Some(mana_leech_gain_rate) => {
                mana_recovery_sum += mana_leech_gain_rate;
            }
        }

        csvs *=
            match player_output_table.get::<&str, Option<f64>>("ManaPerSecondCost").unwrap() {
                None => {
                    0.01
                },
                Some(mana_per_second_cost) => {
                    if mana_per_second_cost == 0.0
                    {
                        mana_per_second_cost
                    }
                    else
                    {
                        self.calc_scv(mana_recovery_sum / mana_per_second_cost, 1.0, 1.0)
                    }
                }
            };

        match player_output_table.get::<&str, Option<f64>>("ReqStr").unwrap() {
            None => {},
            Some(req) => {
                if req != 0.0
                {
                    match player_output_table.get::<&str, Option<f64>>("Str").unwrap() {
                        None => {},
                        Some(stat) => {
                            csvs *= self.calc_scv(stat / req, 1.0, 1.0)
                        }
                    }
                }
            }
        }

        match player_output_table.get::<&str, Option<f64>>("ReqInt").unwrap() {
            None => {},
            Some(req) => {
                if req != 0.0
                {
                    match player_output_table.get::<&str, Option<f64>>("Int").unwrap() {
                        None => {},
                        Some(stat) => {
                            csvs *= self.calc_scv(stat / req, 1.0, 1.0)
                        }
                    }
                }
            }
        }

        match player_output_table.get::<&str, Option<f64>>("ReqDex").unwrap() {
            None => {},
            Some(req) => {
                if req != 0.0
                {
                    match player_output_table.get::<&str, Option<f64>>("Dex").unwrap() {
                        None => {},
                        Some(stat) => {
                            csvs *= self.calc_scv(stat / req, 1.0, 1.0)
                        }
                    }
                }
            }
        }

        csvs
    }

    fn calc_scv(&self, x: f64, weight: f64, target: f64) -> f64
    {
        let x =
            if x < target
            {
                x
            }
            else
            {
                target
            };

        (CSV_WEIGHT_MULTIPLIER * x * (weight / target)).exp() / (weight * CSV_WEIGHT_MULTIPLIER).exp()
    }


}
