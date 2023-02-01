
pub trait Problem {
    fn clone_dyn(&self) -> Box<dyn Problem>;
    fn clone_dyn_send(&self) -> Box<dyn Problem + Send>;
    fn name(&self) -> &str;
    fn convergence_metric(&self, x: &[f64]) -> f64;
}

impl Clone for Box<dyn Problem + Send> {
    fn clone(&self) -> Self {
        self.clone_dyn_send()
    }
}

impl Clone for Box<dyn Problem> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}
