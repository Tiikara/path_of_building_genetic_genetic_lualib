
pub trait Problem {
    fn clone_dyn(&self) -> Box<dyn Problem>;
    fn name(&self) -> &str;
    fn convergence_metric(&self, x: &[f64]) -> f64;
}

impl Clone for Box<dyn Problem> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}
