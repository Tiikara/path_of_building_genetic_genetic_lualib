use crate::mo::evaluator::Evaluator;
use crate::mo::{Solution, SolutionsRuntimeProcessor};

pub mod nsga2;

pub trait Optimizer<S: Solution>
{
    fn name(&self) -> &str;
    fn optimize(&mut self, eval: &mut Box<dyn Evaluator>,
                runtime_solutions_processor: &mut Box<dyn SolutionsRuntimeProcessor<S>>);
    fn best_solutions(&self) -> Vec<(Vec<f64>, S)>;
}
