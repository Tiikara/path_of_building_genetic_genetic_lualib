use std::ops::{Range, RangeInclusive};
use rand::distributions::uniform::SampleRange;
use rand::Rng;
use rand::rngs::ThreadRng;

const MAX_MUTATE_CLUSTER_SIZE: i32 = 4;

#[derive(Clone)]
pub struct Dna {
    pub body: Vec<i8>,
    pub fitness_score: f64
}

impl Dna {
    pub fn new(tree_nodes_count: usize) -> Dna {
        Dna{
            body: vec![0; tree_nodes_count],
            fitness_score: -1.0
        }
    }

    pub fn mutate(&mut self, rng: &mut ThreadRng) {
        let mutate_cluster_size = rng.gen_range(1..self.body.len() + 1);
        let start_num = rng.gen_range(0..self.body.len() - mutate_cluster_size);

        let body_slice = &mut self.body[start_num..start_num+mutate_cluster_size];

        for nucl in body_slice.iter_mut() {
            if *nucl == 1
            {
                *nucl = 0;
            }
            else
            {
                *nucl = 1;
            }
        }
    }

    pub fn selection(&self, dna2: &Dna, rng: &mut ThreadRng) -> Dna {
        let crossover_start: usize = rng.gen_range(0..self.body.len());
        let crossover_end: usize = rng.gen_range(0..self.body.len());

        if crossover_start > crossover_end
        {
            Dna::crossover_dna(dna2, &self, crossover_start..=crossover_end)
        }
        else
        {
            Dna::crossover_dna(&self, dna2, crossover_end..=crossover_start)
        }
    }

    fn crossover_dna(dna1: &Dna, dna2: &Dna, range: RangeInclusive<usize>) -> Dna {
        let mut new_dna = dna1.clone();

        new_dna.body[range.clone()].clone_from_slice(&dna2.body[range]);

        new_dna
    }
}
