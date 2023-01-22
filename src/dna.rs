use std::ops::{Deref, DerefMut, RangeInclusive};
use std::rc::Rc;
use mlua::UserData;
use rand::Rng;
use rand::rngs::ThreadRng;
use rand_distr::{Normal, Distribution};

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
    pub body_node_edges: Vec<u8>,
    pub body_masteries: Vec<u8>,
    pub fitness_score: f64
}

impl DnaData {
    pub(crate) fn new(tree_nodes_count: usize, mastery_count: usize) -> DnaData {
        DnaData {
            body_node_edges: vec![0; tree_nodes_count * 10],
            body_masteries: vec![0; mastery_count * 6],
            fitness_score: -1.0
        }
    }

    fn init(&mut self) {
        for item in &mut self.body_node_edges { *item = 0; }
        for item in &mut self.body_masteries { *item = 0; }
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

        dna_data.body_node_edges[..self.body_node_edges.len()].clone_from_slice(&self.body_node_edges[..self.body_node_edges.len()]);
        dna_data.body_masteries[..self.body_masteries.len()].clone_from_slice(&self.body_masteries[..self.body_masteries.len()]);
        dna_data.fitness_score = self.fitness_score;

        Dna {
            reference: dna_data
        }
    }

    pub fn mutate(&mut self, rng: &mut ThreadRng, distr: Normal<f64>) {
        // Mutate nodes
        let len = self.body_node_edges.len() as f64;
        for nucl in &mut self.body_node_edges
        {
            let r = rng.gen_range(0.0..1.0);

            if r < 0.1
            {
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

    pub fn combine(&self, dna_data_allocator: &mut Vec<Box<DnaData>>, dna2: &Dna, rng: &mut ThreadRng) -> Dna {
        let crossover_body_start: usize = rng.gen_range(0..self.body_node_edges.len());
        let crossover_body_end: usize = rng.gen_range(0..self.body_node_edges.len());

        let crossover_masteries_start: usize = rng.gen_range(0..self.body_masteries.len());
        let crossover_masteries_end: usize = rng.gen_range(crossover_masteries_start..self.body_masteries.len());

        let range_masteries_nodes = crossover_masteries_start..=crossover_masteries_end;

        if crossover_body_start < crossover_body_end
        {
            Dna::crossover_dna(dna_data_allocator,
                               dna2,
                               self,
                               crossover_body_start..=crossover_body_end,
                               range_masteries_nodes)
        }
        else
        {
            Dna::crossover_dna(dna_data_allocator,
                               self,
                               dna2,
                               crossover_body_end..=crossover_body_start,
                               range_masteries_nodes)
        }
    }

    fn crossover_dna(dna_data_allocator: &mut Vec<Box<DnaData>>,
                     dna1: &Dna,
                     dna2: &Dna,
                     range_body_nodes: RangeInclusive<usize>,
                     range_masteries_nodes: RangeInclusive<usize>) -> Dna
    {
        let mut new_dna = dna1.clone(dna_data_allocator);

        new_dna.body_node_edges[range_body_nodes.clone()].clone_from_slice(&dna2.body_node_edges[range_body_nodes]);
        new_dna.body_masteries[range_masteries_nodes.clone()].clone_from_slice(&dna2.body_masteries[range_masteries_nodes]);

        new_dna
    }
}
