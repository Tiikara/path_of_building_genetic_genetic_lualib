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

#[derive(Clone)]
pub struct DnaData {
    pub body_nodes: Vec<u8>,
    pub body_mysteries: Vec<u8>,
    pub fitness_score: f64
}

impl DnaData {
    pub(crate) fn new(tree_nodes_count: usize, mastery_count: usize) -> DnaData {
        DnaData {
            body_nodes: vec![0; tree_nodes_count],
            body_mysteries: vec![0; mastery_count * 6],
            fitness_score: -1.0
        }
    }

    fn init(&mut self) {
        for item in &mut self.body_nodes { *item = 0; }
        for item in &mut self.body_mysteries { *item = 0; }
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

        dna_data.body_nodes[..self.body_nodes.len()].clone_from_slice(&self.body_nodes[..self.body_nodes.len()]);
        dna_data.body_mysteries[..self.body_mysteries.len()].clone_from_slice(&self.body_mysteries[..self.body_mysteries.len()]);
        dna_data.fitness_score = self.fitness_score;

        Dna {
            reference: dna_data
        }
    }

    pub fn mutate(&mut self, rng: &mut ThreadRng) {
        // Mutate nodes
        let mutate_cluster_size = rng.gen_range(1..=MAX_MUTATE_CLUSTER_SIZE);
        let start_num = rng.gen_range(0..self.body_nodes.len() - mutate_cluster_size);

        let body_slice = &mut self.body_nodes[start_num..start_num+mutate_cluster_size];

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

        // Mutate mysteries
        let mutate_cluster_size = 1;
        let start_num = rng.gen_range(0..self.body_mysteries.len() - mutate_cluster_size);

        let body_slice = &mut self.body_mysteries[start_num..start_num+mutate_cluster_size];

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

    pub fn combine(&self, dna_data_allocator: &mut Vec<Box<DnaData>>, dna2: &Dna, rng: &mut ThreadRng) -> Dna {
        let crossover_body_start: usize = rng.gen_range(0..self.body_nodes.len());
        let crossover_body_end: usize = rng.gen_range(0..self.body_nodes.len());

        let crossover_mysteries_start: usize = rng.gen_range(0..self.body_mysteries.len());
        let crossover_mysteries_end: usize = rng.gen_range(crossover_mysteries_start..self.body_mysteries.len());

        let range_mysteries_nodes = crossover_mysteries_start..=crossover_mysteries_end;

        if crossover_body_start < crossover_body_end
        {
            Dna::crossover_dna(dna_data_allocator,
                               dna2,
                               self,
                               crossover_body_start..=crossover_body_end,
                               range_mysteries_nodes)
        }
        else
        {
            Dna::crossover_dna(dna_data_allocator,
                               self,
                               dna2,
                               crossover_body_end..=crossover_body_start,
                               range_mysteries_nodes)
        }
    }

    fn crossover_dna(dna_data_allocator: &mut Vec<Box<DnaData>>,
                     dna1: &Dna,
                     dna2: &Dna,
                     range_body_nodes: RangeInclusive<usize>,
                     range_mysteries_nodes: RangeInclusive<usize>) -> Dna
    {
        let mut new_dna = dna1.clone(dna_data_allocator);

        new_dna.body_nodes[range_body_nodes.clone()].clone_from_slice(&dna2.body_nodes[range_body_nodes]);
        new_dna.body_mysteries[range_mysteries_nodes.clone()].clone_from_slice(&dna2.body_mysteries[range_mysteries_nodes]);

        new_dna
    }
}
