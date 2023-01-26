use crate::fitness_function_calculator::{FitnessFunctionCalculator, FitnessFunctionCalculatorStats};

pub trait Target: Send + Sync
{
    fn clone_dyn(&self) -> Box<dyn Target>;
    fn calc_fitness_score(&self, fitness_function_calculator: &FitnessFunctionCalculator, stats: &mut FitnessFunctionCalculatorStats) -> f64;
}

impl Clone for Box<dyn Target> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}
