#![allow(dead_code)]
// Experimental cache feature

use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::dna::{Dna};

#[derive(Hash)]
pub struct DnaKey<'a>(&'a [u8], &'a [u8]);

pub struct DnaCacheFitness
{
    pub(crate) cache_map: HashMap<u64, f64>
}

impl DnaCacheFitness {
    pub fn try_get_fitness_score_by_dna(&self, dna: &Dna) -> Option<f64>
    {
        let mut hasher = DefaultHasher::new();

        DnaKey{
            0: dna.reference.body_nodes.borrow(),
            1: dna.reference.body_masteries.borrow()
        }.hash(&mut hasher);

        let key = hasher.finish();

        let res = self.cache_map.get(&key);

        match res {
            Some(res) => Some(res.clone()),
            None => None
        }
    }

    pub fn set_fitness_score_by_dna(&mut self, dna: &Dna, fitness_score: f64)
    {
        let mut hasher = DefaultHasher::new();

        DnaKey{
            0: dna.reference.body_nodes.borrow(),
            1: dna.reference.body_masteries.borrow()
        }.hash(&mut hasher);

        let key = hasher.finish();

        self.cache_map.insert(key, fitness_score);
    }
}
