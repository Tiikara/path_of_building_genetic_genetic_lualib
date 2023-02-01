use std::fmt::format;
use rand::{Rng, thread_rng};
use crate::mo::array_solution::ArraySolutionEvaluator;
use crate::mo::problem::Problem;
use crate::mo::tests::dtlz::{g1};

#[derive(Clone)]
pub struct Dtlz1
{
    name: String,
    n_var: usize,
    n_obj: usize
}

impl Dtlz1 {
    pub fn new(n_var: usize, n_obj: usize) -> Self
    {
        Dtlz1 {
            name: format!("DTLZ1 ({} {})", n_var, n_obj),
            n_var,
            n_obj
        }
    }
}

impl Problem for Dtlz1
{
    fn clone_dyn(&self) -> Box<dyn Problem> {
        Box::new(self.clone())
    }

    fn clone_dyn_send(&self) -> Box<dyn Problem + Send> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn convergence_metric(&self, in_x: &[f64]) -> f64 {
        let x_m = &in_x[self.n_obj - 1..];

        g1(x_m)
    }
}

impl ArraySolutionEvaluator for Dtlz1
{
    fn clone_dyn(&self) -> Box<dyn ArraySolutionEvaluator> {
        Box::new(self.clone())
    }

    fn clone_dyn_send(&self) -> Box<dyn ArraySolutionEvaluator + Send> {
        Box::new(self.clone())
    }

    fn calculate_objectives(&self, in_x: &Vec<f64>, f: &mut Vec<f64>) {
        let x = &in_x[..self.n_obj - 1];
        let x_m = &in_x[self.n_obj - 1..];

        let g = g1(x_m);

        if f.len() != self.n_obj
        {
            f.resize(self.n_obj, 0.0);
        }

        for i in 0..self.n_obj
        {
            let mut f_val = 0.5 * (1.0 + g);

            for x_i in &x[..x.len() - i]
            {
                f_val *= x_i;
            }

            if i > 0
            {
                f_val *= 1.0 - x[x.len() - i];
            }

            f[i] = f_val;
        }
    }

    fn x_len(&self) -> usize {
        self.n_var
    }

    fn objectives_len(&self) -> usize {
        self.n_obj
    }

    fn min_x_value(&self) -> f64 {
        0.0
    }

    fn max_x_value(&self) -> f64 {
        1.0
    }
}
