use crate::mo::evaluator::Evaluator;
use crate::mo::{Solution, SolutionsRuntimeProcessor};

pub trait Optimizer<S: Solution>
{
    fn optimize(&mut self, eval: Box<dyn Evaluator>,
                runtime_solutions_processor: Box<dyn SolutionsRuntimeProcessor<S>>);
}
