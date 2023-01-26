use std::cmp::min;
use std::collections::HashMap;
use std::f64::MIN;
use std::fs;
use mlua::prelude::LuaTable;
use mlua::Table;
use crate::targets::Target;

const MIN_TARGET_MULTIPLIER: f64 = 0.01;

pub struct FitnessFunctionCalculator
{
    pub targets: Vec<Target>,
    target_normal_nodes_count: f64,
    target_ascendancy_nodes_count: f64
}

pub struct FitnessFunctionCalculatorStats<'a>
{
    stats_env: &'a LuaTable<'a>,
    actor_outputs: HashMap<String, LuaTable<'a>>,
    stat_values: HashMap<String, Option<f64>>
}

impl<'a> FitnessFunctionCalculatorStats<'a>
{
    pub fn new(stats_env: &'a LuaTable<'_>) ->  Self
    {
        FitnessFunctionCalculatorStats {
            stats_env,
            actor_outputs: HashMap::with_capacity(2),
            stat_values: Default::default(),
        }
    }

    pub fn try_get_stat(&mut self, actor: String, stat: String) -> Option<f64> {
        let actor_output_table = self.actor_outputs
            .entry(actor.clone())
            .or_insert_with(|| {
                let actor_table = self.stats_env.get::<&str, LuaTable>(actor.as_str()).unwrap();

                actor_table.get::<&str, LuaTable>("output").unwrap()
            });

        self.stat_values
            .entry(stat.clone())
            .or_insert_with(|| {
                actor_output_table.get::<&str, Option<f64>>(stat.as_str()).unwrap()
            }).clone()
    }
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

    pub fn calculate_fitness_score_for_target<'a>(&self, stats: &mut FitnessFunctionCalculatorStats<'a>, target: &Target) -> f64
    {
        let stat = stats.try_get_stat(target.actor.clone(), target.stat.clone());

        if target.is_maximize
        {
            match stat {
                None => {
                    MIN_TARGET_MULTIPLIER
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
                    MIN_TARGET_MULTIPLIER
                }
                Some(stat_value) => {
                    self.calc_target_mul(stat_value, target.weight, target.target, target.lower_is_better)
                }
            }
        }
    }

    pub fn calculate_and_get_fitness_score<'a>(&self, stats: &mut FitnessFunctionCalculatorStats<'a>, used_normal_node_count: usize, used_ascendancy_node_count: usize) -> f64
    {
        let mut score = 1.0;

        for target in &self.targets
        {
            score *= self.calculate_fitness_score_for_target(stats, target);
        }

        /*let player_output_table = actor_outputs
            .entry("player")
            .or_insert_with(|| {
                let actor_table = stats_env.get::<&str, LuaTable>("player").unwrap();

                actor_table.get::<&str, LuaTable>("output").unwrap()
            });*/

        /*let mut mana_recovery_sum = 0.0;

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

        score *=
            match player_output_table.get::<&str, Option<f64>>("ManaPerSecondCost").unwrap() {
                None => {
                    MIN_TARGET_MULTIPLIER
                },
                Some(mana_per_second_cost) => {
                    self.calc_target_mul(mana_recovery_sum, 1.0, mana_per_second_cost, false)
                }
            };

        score *=
            match player_output_table.get::<&str, Option<f64>>("ManaUnreserved").unwrap() {
                None => {
                    MIN_TARGET_MULTIPLIER
                },
                Some(unreserved_mana) => {
                    match player_output_table.get::<&str, Option<f64>>("ManaCost").unwrap() {
                        None => {
                            MIN_TARGET_MULTIPLIER
                        },
                        Some(mana_cost) => {
                            self.calc_target_mul(unreserved_mana, 1.0, mana_cost, false)
                        }
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
                            score *= self.calc_target_mul(stat, 1.0, req, false);
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
                            score *= self.calc_target_mul(stat, 1.0, req, false);
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
                            score *= self.calc_target_mul(stat, 1.0, req, false);
                        }
                    }
                }
            }
        }*/

        score
    }

    fn calc_target_mul(&self, mut x: f64, weight: f64, mut target: f64, lower_is_better: bool) -> f64
    {
        if x < 0.0
        {
            target -= x;
            x = 0.0;
        }

        if target < 0.0
        {
            x -= target;
            target = 0.0;
        }

        let mut ratio =
            if lower_is_better
            {
                if x == 0.0
                {
                    1.0
                }
                else
                {
                    target / x
                }
            }
            else
            {
                if target == 0.0
                {
                    1.0
                }
                else
                {
                    x / target
                }
            };

        if ratio > 1.0
        {
            ratio = 1.0;
        }

        MIN_TARGET_MULTIPLIER + (1.0 - MIN_TARGET_MULTIPLIER) * ratio
    }
}
