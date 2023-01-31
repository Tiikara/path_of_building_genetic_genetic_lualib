pub mod dtlz1;

use std::fmt::{Formatter};
use rand::{Rng, thread_rng};
use crate::mo::{Meta, Objective, Ratio, Solution, SolutionsRuntimeProcessor};

fn g1(x_m: &[f64]) -> f64
{
    let mut sum = 0.0;

    for x_m_i in x_m
    {
        sum += (x_m_i - 0.5).powi(2) - (20.0 * std::f64::consts::PI * (x_m_i - 0.5)).cos();
    }

    100.0 * (x_m.len() as f64 + sum)
}

fn g2(x_m: &Vec<f64>) -> f64
{
    let mut sum = 0.0;

    for x_m_i in x_m.iter()
    {
        sum += (x_m_i - 0.5).sqrt();
    }

    sum
}
