use dyn_clone::DynClone;

pub trait Problem: DynClone {
    fn name(&self) -> &str;
    fn convergence_metric(&self, x: &[f64]) -> f64;
}

dyn_clone::clone_trait_object!(Problem);
