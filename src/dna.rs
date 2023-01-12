use std::mem;
use std::ops::{Deref, DerefMut, RangeInclusive};
use rand::Rng;
use rand::rngs::ThreadRng;

const MAX_MUTATE_CLUSTER_SIZE: usize = 4;

pub struct Dna {
    pub reference: Box<DnaData>
}

impl<'a> Deref for Dna {
    type Target = DnaData;
    fn deref(&self) -> &DnaData { &self.reference }
}

impl<'a> DerefMut for Dna {
    fn deref_mut(&mut self) -> &mut DnaData { &mut self.reference }
}

pub struct DnaData {
    pub body: Vec<i8>,
    pub fitness_score: f64
}

impl DnaData {
    pub(crate) fn new(tree_nodes_count: usize) -> DnaData {
        DnaData {
            body: vec![0; tree_nodes_count],
            fitness_score: -1.0
        }
    }

    fn init(&mut self) {
        for item in &mut self.body { *item = 0; }
        self.fitness_score = -1.0;
    }
}

impl Dna {
    pub fn new(dna_data_allocator: &mut Vec<Box<DnaData>>) -> Dna {
        let mut dna_data = dna_data_allocator.pop().unwrap();

        dna_data.init();

        Dna {
            reference: dna_data
        }
    }

    pub fn clone(&self, dna_data_allocator: &mut Vec<Box<DnaData>>) -> Dna {
        let mut dna_data = dna_data_allocator.pop().unwrap();

        dna_data.body[..self.body.len()].clone_from_slice(&self.body[..self.body.len()]);
        dna_data.fitness_score = self.fitness_score;

        Dna {
            reference: dna_data
        }
    }

    pub fn mutate(&mut self, rng: &mut ThreadRng) {
        let mutate_cluster_size = rng.gen_range(1..=MAX_MUTATE_CLUSTER_SIZE);
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

    pub fn selection(&self, dna_data_allocator: &mut Vec<Box<DnaData>>, dna2: &Dna, rng: &mut ThreadRng) -> Dna {
        let crossover_start: usize = rng.gen_range(0..self.body.len());
        let crossover_end: usize = rng.gen_range(0..self.body.len());

        if crossover_start < crossover_end
        {
            Dna::crossover_dna(dna_data_allocator, dna2, self, crossover_start..=crossover_end)
        }
        else
        {
            Dna::crossover_dna(dna_data_allocator, self, dna2, crossover_end..=crossover_start)
        }
    }

    fn crossover_dna(dna_data_allocator: &mut Vec<Box<DnaData>>, dna1: &Dna, dna2: &Dna, range: RangeInclusive<usize>) -> Dna {
        let mut new_dna = dna1.clone(dna_data_allocator);

        new_dna.body[range.clone()].clone_from_slice(&dna2.body[range]);

        new_dna
    }
}
