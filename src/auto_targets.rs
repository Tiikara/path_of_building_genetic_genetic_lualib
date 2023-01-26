use crate::fitness_function_calculator::{FitnessFunctionCalculator, FitnessFunctionCalculatorStats};
use crate::target::Target;

#[derive(Clone)]
pub struct AutoTargetManaRegen
{}

impl Target for AutoTargetManaRegen
{
    fn clone_dyn(&self) -> Box<dyn Target> {
        Box::new(self.clone())
    }

    fn calc_fitness_score(&self, fitness_function_calculator: &FitnessFunctionCalculator, stats: &mut FitnessFunctionCalculatorStats) -> f64 {
        let mut mana_recovery_sum = 0.0;

        match stats.try_get_stat(String::from("player"), String::from("ManaRegenRecovery")) {
            None => {},
            Some(mana_regen_recovery) => {
                mana_recovery_sum += mana_regen_recovery;
            }
        }

        match stats.try_get_stat(String::from("player"), String::from("ManaLeechGainRate")) {
            None => {},
            Some(mana_leech_gain_rate) => {
                mana_recovery_sum += mana_leech_gain_rate;
            }
        }

        match stats.try_get_stat(String::from("player"), String::from("ManaPerSecondCost")) {
            None => {
                0.01
            },
            Some(mana_per_second_cost) => {
                fitness_function_calculator.calc_target_mul(mana_recovery_sum, 1.0, mana_per_second_cost, false)
            }
        }
    }
}

#[derive(Clone)]
pub struct AutoTargetManaCost
{}

impl Target for AutoTargetManaCost
{
    fn clone_dyn(&self) -> Box<dyn Target> {
        Box::new(self.clone())
    }

    fn calc_fitness_score(&self, fitness_function_calculator: &FitnessFunctionCalculator, stats: &mut FitnessFunctionCalculatorStats) -> f64 {
        match stats.try_get_stat(String::from("player"), String::from("ManaUnreserved")) {
            None => {
                0.01
            },
            Some(unreserved_mana) => {
                match stats.try_get_stat(String::from("player"), String::from("ManaCost")) {
                    None => {
                        0.01
                    },
                    Some(mana_cost) => {
                        fitness_function_calculator.calc_target_mul(unreserved_mana, 1.0, mana_cost, false)
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct AutoTargetFromStatToStat
{
    pub(crate) target_stat_name: String,
    pub(crate) current_stat_name: String
}

impl Target for AutoTargetFromStatToStat
{
    fn clone_dyn(&self) -> Box<dyn Target> {
        Box::new(self.clone())
    }

    fn calc_fitness_score(&self, fitness_function_calculator: &FitnessFunctionCalculator, stats: &mut FitnessFunctionCalculatorStats) -> f64 {
        match stats.try_get_stat(String::from("player"),  self.target_stat_name.clone()) {
            None => {
                1.0
            },
            Some(req) => {
                if req != 0.0
                {
                    match stats.try_get_stat(String::from("player"),  self.current_stat_name.clone()) {
                        None => {
                            0.01
                        },
                        Some(stat) => {
                            fitness_function_calculator.calc_target_mul(stat, 1.0, req, false)
                        }
                    }
                }
                else
                {
                    1.0
                }
            }
        }
    }
}
