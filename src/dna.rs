use std::ops::{Deref, DerefMut, RangeInclusive};
use std::rc::Rc;
use mlua::UserData;
use rand::prelude::SliceRandom;
use rand::{Rng, thread_rng};

const MAX_MUTATE_CLUSTER_SIZE: usize = 4;

#[derive(Clone)]
pub struct LuaDna
{
    pub reference: Rc<Dna>
}

impl UserData for LuaDna {}

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
    pub body_masteries: Vec<u8>,
    pub max_count_nodes: usize,
    pub fitness_score: f64,
    pub fitness_score_targets: Vec<f64>
}

impl DnaData {
    pub(crate) fn new(tree_nodes_count: usize, mastery_count: usize, targets_count: usize, max_count_nodes: usize) -> DnaData {
        DnaData {
            body_nodes: vec![0; tree_nodes_count],
            body_masteries: vec![0; mastery_count * 6],
            max_count_nodes,
            fitness_score: -1.0,
            fitness_score_targets: vec![-1.0; targets_count]
        }
    }
}

impl Clone for Dna {
    fn clone(&self) -> Dna {
        Dna {
            reference: self.reference.clone()
        }
    }
}

impl Dna {
    pub fn new(dna_data: DnaData) -> Dna {
        Dna {
            reference: Box::new(dna_data)
        }
    }

    pub fn mutate(&mut self) {
        let mut rng = thread_rng();

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

        // Mutate masteries
        let mutate_cluster_size = 1;
        let start_num = rng.gen_range(0..self.body_masteries.len() - mutate_cluster_size);

        let body_slice = &mut self.body_masteries[start_num..start_num+mutate_cluster_size];

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

    pub fn combine(&self, dna2: &Dna) -> Dna {
        let mut rng = thread_rng();

        let crossover_body_start: usize = rng.gen_range(0..self.body_nodes.len());
        let crossover_body_end: usize = rng.gen_range(0..self.body_nodes.len());

        let crossover_masteries_start: usize = rng.gen_range(0..self.body_masteries.len());
        let crossover_masteries_end: usize = rng.gen_range(crossover_masteries_start..self.body_masteries.len());

        let range_masteries_nodes = crossover_masteries_start..=crossover_masteries_end;

        if crossover_body_start < crossover_body_end
        {
            Dna::crossover_dna(dna2,
                               self,
                               crossover_body_start..=crossover_body_end,
                               range_masteries_nodes)
        }
        else
        {
            Dna::crossover_dna(self,
                               dna2,
                               crossover_body_end..=crossover_body_start,
                               range_masteries_nodes)
        }
    }

    fn crossover_dna(dna1: &Dna,
                     dna2: &Dna,
                     range_body_nodes: RangeInclusive<usize>,
                     range_masteries_nodes: RangeInclusive<usize>) -> Dna
    {
        let mut new_dna = dna1.clone();

        new_dna.body_nodes[range_body_nodes.clone()].clone_from_slice(&dna2.body_nodes[range_body_nodes]);
        new_dna.body_masteries[range_masteries_nodes.clone()].clone_from_slice(&dna2.body_masteries[range_masteries_nodes]);

        let mut selected_nodes = Vec::new();
        for (index, nucl) in new_dna.body_nodes.iter().enumerate()
        {
            if *nucl == 1
            {
                selected_nodes.push(index);
            }
        }

        if selected_nodes.len() > new_dna.max_count_nodes
        {
            selected_nodes.shuffle(&mut thread_rng());

            while selected_nodes.len() > new_dna.max_count_nodes
            {
                let index = selected_nodes.pop().unwrap();

                new_dna.body_nodes[index] = 0;
            }
        }

        new_dna
    }
}
