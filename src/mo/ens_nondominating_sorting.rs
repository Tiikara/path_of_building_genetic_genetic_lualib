use std::cmp::Ordering;
use crate::mo::Solution;

fn dominates(vals1: &Vec<f64>, vals2: &Vec<f64>) -> bool {
    vals1.iter().zip(vals2).all(|(v1, v2)| *v1 <= *v2) &&
        vals1.iter().zip(vals2).any(|(v1, v2)| *v1 < *v2)
}

pub fn ens_nondominated_sorting(pop: &mut Vec<Vec<f64>>) -> Vec<Vec<usize>> {
    pop.sort_unstable_by(|a, b| a.first().unwrap().partial_cmp(&b.first().unwrap()).unwrap());

    let mut fronts = vec![];

    for (n, p) in pop.iter().enumerate() {
        let k = sequential_search(pop, p, &fronts);
        if k == fronts.len() {
            fronts.push(vec![n]);
        } else {
            fronts[k].push(n);
        }
    }

    fronts
}

fn sequential_search(pop: &Vec<Vec<f64>>,p: &Vec<f64>, fronts: &[Vec<usize>]) -> usize {
    let mut k = 0;
    let x = fronts.len();
    while k < x {
        let mut dominated = false;
        for &index in fronts[k].iter().rev() {
            let sol = &pop[index];
            if dominates(sol, p) {
                dominated = true;
                break;
            }
        }
        if !dominated {
            return k;
        }
        k += 1;
    }
    x
}
