use std::collections::HashMap;

pub struct AdjustSpace
{
    pub(crate) space_indexes: HashMap<(usize, usize), usize>,
    pub(crate) count_indexes: usize
}

impl AdjustSpace {
    pub fn try_add_nodes(&mut self, mut node_id1: usize, mut node_id2: usize)
    {
        if node_id1 < node_id2
        {
            std::mem::swap(&mut node_id1, &mut node_id2);
        }

        match self.space_indexes.get(&(node_id1, node_id2))
        {
            None => {
                self.space_indexes.insert((node_id1, node_id2), self.count_indexes);

                self.count_indexes += 1;
            },
            _ => {}
        }
    }

    pub fn allocate_vector_data(&self) -> Vec<u8>
    {
        vec![0; self.count_indexes]
    }

    pub fn get_adjust_value_from_data(&self, data: &Vec<u8>, mut node_id1: usize, mut node_id2: usize) -> u8
    {
        if node_id1 < node_id2
        {
            std::mem::swap(&mut node_id1, &mut node_id2);
        }

        data[*self.space_indexes.get(&(node_id1, node_id2)).unwrap()]
    }

    pub fn set_adjust_value_to_data(&self, data: &mut Vec<u8>, mut node_id1: usize, mut node_id2: usize, value: u8)
    {
        if node_id1 < node_id2
        {
            std::mem::swap(&mut node_id1, &mut node_id2);
        }

        data[*self.space_indexes.get(&(node_id1, node_id2)).unwrap()] = value;
    }
}
