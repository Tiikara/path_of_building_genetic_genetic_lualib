use std::collections::HashMap;
use mlua::prelude::LuaTable;

use crate::target::Target;

const MIN_TARGET_MULTIPLIER: f64 = 0.01;

pub struct FitnessFunctionCalculator
{
    pub(crate) targets: Vec<Box<dyn Target>>
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
    pub fn new(targets: Vec<Box<dyn Target>>) -> Self
    {
        FitnessFunctionCalculator{
            targets
        }
    }

    pub fn calculate_and_get_fitness_score<'a>(&self, stats: &mut FitnessFunctionCalculatorStats<'a>) -> f64
    {
        let mut score = 1.0;

        for target in &self.targets
        {
            score *= target.calc_fitness_score(self, stats);
        }

        score
    }

    pub(crate) fn calc_target_mul(&self, mut x: f64, weight: f64, mut target: f64, lower_is_better: bool) -> f64
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
