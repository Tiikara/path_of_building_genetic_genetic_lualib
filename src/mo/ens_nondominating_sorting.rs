use std::cmp::Ordering;
use crate::mo::Solution;

fn dominates(vals1: &Vec<f64>, other: &Vec<f64>) -> bool {
    let mut self_better = false;
    let mut equal = false;
    for (i, obj) in vals1.iter().enumerate() {
        match obj.partial_cmp(&other[i]).unwrap() {
            Ordering::Less => self_better = true,
            Ordering::Equal => equal = true,
            Ordering::Greater => return false,
        }
    }
    self_better && !equal
}

pub fn ens_nondominated_sorting(pop: &Vec<Vec<f64>>) -> Vec<Vec<usize>> {
    let mut pop_i: Vec<(usize, &Vec<f64>)> = pop.iter().enumerate()
        .map(|(i, p)| (i, p))
        .collect();

    pop_i.sort_unstable_by(|a, b| a.1.first().unwrap().partial_cmp(&b.1.first().unwrap()).unwrap());

    let pop: Vec<&Vec<f64>> = pop_i.iter().map(|p| p.1).collect();

    let mut fronts = vec![];

    for (n, p) in pop.iter().enumerate() {
        let k = sequential_search(&pop, p, &fronts);
        if k == fronts.len() {
            fronts.push(vec![n]);
        } else {
            fronts[k].push(n);
        }
    }

    fronts.iter()
        .map(|front| front.iter().map(|i| pop_i[*i].0 ).collect())
        .collect()
}

fn sequential_search(pop: &Vec<&Vec<f64>>, p: &Vec<f64>, fronts: &[Vec<usize>]) -> usize {
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
