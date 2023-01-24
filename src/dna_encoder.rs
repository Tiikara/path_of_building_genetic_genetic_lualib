use std::borrow::{Borrow, BorrowMut};
use std::cell::{RefCell};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use mlua::{Lua, TableExt, UserData, UserDataMethods};
use mlua::prelude::{LuaResult, LuaString, LuaTable, LuaValue};
use rand::prelude::{SliceRandom, ThreadRng};
use rand::Rng;
use rand_distr::{Normal, Distribution};
use crate::adjust_space::AdjustSpace;
use crate::dna::{Dna, LuaDna};

use crate::worker::LuaDnaCommand;

pub struct DnaEncoder
{
    tree_nodes: Vec<RefCell<Node>>,
    masteries: Vec<RefCell<Mastery>>,

    pub adjust_space: AdjustSpace,
    queue_indexes_buffer: Vec<usize>
}

pub struct DnaConvertResult
{
    pub allocated_normal_nodes: usize,
    pub allocated_ascend_nodes: usize
}

impl DnaConvertResult {
    pub fn get_table<'a>(&self, lua_context: &'a Lua) -> LuaTable<'a>
    {
        let res_table = lua_context.create_table().unwrap();

        res_table.set("usedNormalNodeCount", self.allocated_normal_nodes).unwrap();
        res_table.set("usedAscendancyNodeCount", self.allocated_ascend_nodes).unwrap();

        res_table
    }
}

struct BuildNodeAllocator<'a>
{
    dna_encoder: &'a DnaEncoder,
    spec_table: &'a LuaTable<'a>,
    mastery_selections_table: &'a LuaTable<'a>,
    tree_table: &'a LuaTable<'a>,
    mastery_effects_table: &'a LuaTable<'a>,
    nodes_table: &'a LuaTable<'a>,
    alloc_nodes_table: &'a LuaTable<'a>
}

impl<'a> BuildNodeAllocator<'a> {
    pub fn init(&self)
    {
        let _: LuaValue = self.spec_table.call_method("ResetNodes", 0).unwrap();
    }

    fn allocate_node_on_alloc_nodes_table(&self, node: &mut Node) {
        let node_table: LuaTable = self.nodes_table.get(node.id).unwrap();
        node_table.set("alloc", true).unwrap();
        self.alloc_nodes_table.set(node.id, node_table).unwrap();
    }

    pub fn try_allocate_node(&self, node: &mut Node)
    {
        match node.node_type {
            NodeType::NORMAL => {
                node.alloc = true;
                self.allocate_node_on_alloc_nodes_table(node);
            },
            NodeType::MASTERY => {
                let mut mastery = self.dna_encoder.masteries[node.mastery_index].borrow_mut();

                if mastery.effect_next_select_index < mastery.effects_indexes_to_select.len()
                {
                    let effect_index = mastery.effects_indexes_to_select[mastery.effect_next_select_index];

                    let effect_id = mastery.effects[effect_index].id;

                    self.mastery_selections_table.set(node.id, effect_id).unwrap();

                    let effect_table: LuaTable = self.mastery_effects_table.get(effect_id).unwrap();

                    let lua_sd: LuaValue = effect_table.get("sd").unwrap();

                    let node_table: LuaTable = self.nodes_table.get(node.id).unwrap();

                    node_table.set("sd", lua_sd).unwrap();
                    node_table.set("allMasteryOptions", false).unwrap();

                    let _: LuaValue = self.tree_table.call_method("ProcessStats", node_table).unwrap();

                    mastery.effect_next_select_index += 1;

                    node.alloc = true;
                    self.allocate_node_on_alloc_nodes_table(node);
                }
            },
            _ => {}
        }
    }
}

impl DnaEncoder {
    pub fn mutate_node_edges_from_dna(&mut self, rng: &mut ThreadRng, dna: &mut Dna, max_number_normal_nodes_to_allocate: usize, max_number_ascend_nodes_to_allocate: usize)
    {
        let mut queue_indexes = Vec::new();

        std::mem::swap(&mut queue_indexes, &mut self.queue_indexes_buffer);

        for (_node_index, node) in self.tree_nodes.iter().enumerate()
        {
            let mut node = node.borrow_mut();

            node.alloc = node.default_alloc;
            node.path_index = usize::MAX;
        }

        let mut allocated_normal_nodes = 0;
        let mut allocated_ascend_nodes = 0;

        let mut heads_nodes_indexes = Vec::new();

        for root in self.tree_nodes.iter()
        {
            let start_path = {
                let node = root.borrow();
                node.default_alloc
            };

            if start_path
            {
                {
                    let mut root = root.borrow_mut();

                    queue_indexes.clear();
                    queue_indexes.push(root.tree_node_index);
                }

                let mut o = 0; // out
                let mut i = 1; // in

                while o < i
                {
                    let mut node = self.tree_nodes[queue_indexes[o.clone()]].borrow_mut();

                    o += 1;

                    let mut has_unlinked_nodes = false;

                    for (linked_index, linked_node_index) in node.linked_indexes.iter().enumerate()
                    {
                        let node_nucl = self.adjust_space.get_adjust_value_from_data(
                            &dna.body_node_adj,
                            node.tree_node_index,
                            *linked_node_index
                        );

                        if node_nucl == 1
                        {
                            let mut other = self.tree_nodes[*linked_node_index].borrow_mut();

                            if rng.gen_range(0.0..1.0) < 1.0 / (max_number_normal_nodes_to_allocate + max_number_ascend_nodes_to_allocate) as f64
                            {
                                self.adjust_space.set_adjust_value_to_data(
                                    &mut dna.body_node_adj,
                                    node.tree_node_index,
                                    *linked_node_index,
                                    0
                                );
                            }
                            else
                            {
                                if other.alloc == false
                                {
                                    match other.ascendancy_id {
                                        None => {
                                            if allocated_normal_nodes != max_number_normal_nodes_to_allocate
                                            {
                                                other.alloc = true;
                                                allocated_normal_nodes += 1;

                                                queue_indexes.push(other.tree_node_index);

                                                i += 1;
                                            }
                                        }
                                        Some(_) => {
                                            if allocated_ascend_nodes != max_number_ascend_nodes_to_allocate
                                            {
                                                other.alloc = true;
                                                allocated_ascend_nodes += 1;

                                                queue_indexes.push(other.tree_node_index);

                                                i += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if has_unlinked_nodes == false
                        {
                            let mut other = self.tree_nodes[*linked_node_index].borrow();
                            if other.alloc == false
                            {
                                has_unlinked_nodes = true;
                            }
                        }
                    }

                    if has_unlinked_nodes
                    {
                        heads_nodes_indexes.push(node.tree_node_index);
                    }
                }
            }
        }

        let target_count_add_new_nodes = rng.gen_range(1..=max_number_normal_nodes_to_allocate);
        let mut count_added_nodes = 0;

        while target_count_add_new_nodes != count_added_nodes &&
            (allocated_normal_nodes != max_number_normal_nodes_to_allocate || allocated_ascend_nodes != max_number_ascend_nodes_to_allocate)
        {
            heads_nodes_indexes.shuffle(rng);

            let unlinked_node_index = heads_nodes_indexes.pop().unwrap();

            let mut check_allocated_node_indexes = Vec::new();

            {
                let node  = self.tree_nodes[unlinked_node_index].borrow();

                let mut has_unlinked_nodes = false;
                let mut allocated_linked_node = false;

                for (linked_index, linked_node_index) in node.linked_indexes.iter().enumerate()
                {
                    let mut other = self.tree_nodes[*linked_node_index].borrow_mut();

                    if other.alloc == false && allocated_linked_node == false && rng.gen_range(0.0..1.0) < 1.0 / node.linked_indexes.len() as f64
                    {
                        match other.ascendancy_id {
                            None => {
                                if allocated_normal_nodes != max_number_normal_nodes_to_allocate
                                {
                                    other.alloc = true;
                                    allocated_normal_nodes += 1;
                                }
                            }
                            Some(_) => {
                                if allocated_ascend_nodes != max_number_ascend_nodes_to_allocate
                                {
                                    other.alloc = true;
                                    allocated_ascend_nodes += 1;
                                }
                            }
                        };

                        if other.alloc
                        {
                            self.adjust_space.set_adjust_value_to_data(
                                &mut dna.body_node_adj,
                                node.tree_node_index,
                                *linked_node_index,
                                1
                            );

                            check_allocated_node_indexes.push(*linked_node_index);

                            count_added_nodes += 1;

                            allocated_linked_node = true;
                        }
                    }

                    if has_unlinked_nodes == false
                    {
                        if other.alloc == false
                        {
                            has_unlinked_nodes = true;
                        }
                    }
                }

                if has_unlinked_nodes
                {
                    heads_nodes_indexes.push(node.tree_node_index);
                }
            }

            for node_index in check_allocated_node_indexes
            {
                let node = &self.tree_nodes[node_index].borrow();

                let mut has_unlinked_nodes = false;

                for (linked_index, linked_node_index) in node.linked_indexes.iter().enumerate()
                {
                    let mut other = self.tree_nodes[*linked_node_index].borrow_mut();
                    if other.alloc == false
                    {
                        has_unlinked_nodes = true;
                        break;
                    }
                }

                if has_unlinked_nodes
                {
                    heads_nodes_indexes.push(node.tree_node_index);
                }
            }
        }

        std::mem::swap(&mut queue_indexes, &mut self.queue_indexes_buffer);
    }

    pub fn convert_dna_to_build(&mut self, build_table: &LuaTable, dna: &mut Dna, max_number_normal_nodes_to_allocate: usize, max_number_ascend_nodes_to_allocate: usize) -> DnaConvertResult
    {
        let mut queue_indexes = Vec::new();

        std::mem::swap(&mut queue_indexes, &mut self.queue_indexes_buffer);

        for (_node_index, node) in self.tree_nodes.iter().enumerate()
        {
            let mut node = node.borrow_mut();

            node.alloc = node.default_alloc;
        }

        for mastery in &self.masteries
        {
            let mut mastery = mastery.borrow_mut();

            mastery.effect_next_select_index = 0;
            mastery.effects_indexes_to_select.clear();
        }

        for (index, nucl) in dna.body_masteries.iter().enumerate()
        {
            if *nucl == 1
            {
                let mastery_index = index / 6;
                let effect_index = index % 6;

                let mut mastery = self.masteries[mastery_index].borrow_mut();

                if effect_index < mastery.effects.len()
                {
                    mastery.effects_indexes_to_select.push(effect_index);
                }
            }
        }

        let spec_table: LuaTable = build_table.get("spec").unwrap();
        let mastery_selections_table: LuaTable = spec_table.get("masterySelections").unwrap();
        let tree_table: LuaTable = spec_table.get("tree").unwrap();
        let mastery_effects_table: LuaTable = tree_table.get("masteryEffects").unwrap();
        let nodes_table: LuaTable = spec_table.get("nodes").unwrap();
        let alloc_nodes_table: LuaTable = spec_table.get("allocNodes").unwrap();

        let build_node_allocator = BuildNodeAllocator {
            dna_encoder: self,
            spec_table: &spec_table,
            mastery_selections_table: &mastery_selections_table,
            tree_table: &tree_table,
            mastery_effects_table: &mastery_effects_table,
            nodes_table: &nodes_table,
            alloc_nodes_table: &alloc_nodes_table
        };

        build_node_allocator.init();

        let mut allocated_normal_nodes = 0;
        let mut allocated_ascend_nodes = 0;

        for root in self.tree_nodes.iter()
        {
            let start_path = {
                let node = root.borrow();
                node.default_alloc
            };

            if start_path
            {
                {
                    let mut root = root.borrow_mut();

                    queue_indexes.clear();
                    queue_indexes.push(root.tree_node_index);
                }

                let mut o = 0; // out
                let mut i = 1; // in

                while o < i
                {
                    let node = self.tree_nodes[queue_indexes[o.clone()]].borrow();

                    o += 1;

                    for (_linked_index, linked_node_index) in node.linked_indexes.iter().enumerate()
                    {
                        let node_nucl = self.adjust_space.get_adjust_value_from_data(
                            &dna.body_node_adj,
                            node.tree_node_index,
                            *linked_node_index
                        );

                        if node_nucl == 1
                        {
                            let mut other = self.tree_nodes[*linked_node_index].borrow_mut();

                            if other.alloc == false
                            {
                                match other.ascendancy_id {
                                    None => {
                                        if allocated_normal_nodes != max_number_normal_nodes_to_allocate
                                        {
                                            build_node_allocator.try_allocate_node(other.deref_mut());

                                            if other.alloc
                                            {
                                                allocated_normal_nodes += 1;
                                            }
                                        }
                                    }
                                    Some(_) => {
                                        if allocated_ascend_nodes != max_number_ascend_nodes_to_allocate
                                        {
                                            build_node_allocator.try_allocate_node(other.deref_mut());

                                            if other.alloc
                                            {
                                                allocated_ascend_nodes += 1;
                                            }
                                        }
                                    }
                                }

                                if other.alloc
                                {
                                    match other.node_type {
                                        NodeType::NORMAL => {
                                            queue_indexes.push(other.tree_node_index);
                                            i += 1;
                                        }
                                        NodeType::MASTERY => {}
                                        NodeType::ClassStart => {}
                                        NodeType::AscendClassStart => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // restore buffers
        std::mem::swap(&mut queue_indexes, &mut self.queue_indexes_buffer);

        DnaConvertResult {
            allocated_normal_nodes,
            allocated_ascend_nodes
        }
    }
}

impl UserData for DnaEncoder {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("ConvertDnaToBuild", |lua_context, this, (build_table, mut dna, max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate): (LuaTable, LuaDna, usize, usize)| {

            let mut dna = Dna {
                reference: dna.reference.deref().reference.clone()
            };
            Ok(this.convert_dna_to_build(&build_table, &mut dna, max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate).get_table(lua_context))
        });

        methods.add_method("GetTreeNodesCount", |_lua_context, this, ()| {
            Ok(this.tree_nodes.len())
        });

        methods.add_method("GetMasteryCount", |_lua_context, this, ()| {
            Ok(this.masteries.len())
        });
    }
}

#[derive(Clone)]
enum NodeType
{
    NORMAL,
    MASTERY,
    ClassStart,
    AscendClassStart
}

struct Node
{
    name: String,
    node_type: NodeType,
    tree_node_index: usize,
    mastery_index: usize,
    id: i64,
    linked_indexes: Vec<usize>,
    path_index: usize,
    alloc: bool,
    ascendancy_id: Option<usize>,
    default_alloc: bool
}

struct Mastery
{
    name: String,
    effects: Vec<MasteryEffect>,
    effects_indexes_to_select: Vec<usize>,
    effect_next_select_index: usize
}

struct MasteryEffect
{
    id: i64
}

pub fn lua_create_dna_encoder(_: &Lua, build_table: LuaTable) -> LuaResult<DnaEncoder>
{
    Ok(create_dna_encoder(&build_table))
}

pub fn create_dna_encoder(build_table: &LuaTable) -> DnaEncoder
{
    let spec_table: LuaTable = build_table.get("spec").unwrap();

    let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();
    let _: LuaValue = spec_table.call_method("BuildAllDependsAndPaths", 0).unwrap();

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let count_nodes = nodes_table.len().unwrap();

    let mut tree_nodes = Vec::with_capacity(count_nodes as usize);

    let mut node_id_index_map = HashMap::new();

    let mut ascendacy_id_hash = HashMap::new();
    let mut current_ascendancy_id = 0;

    let current_ascend_class_name: String = spec_table.get("curAscendClassName").unwrap();
    let selected_ascendancy_id =
            ascendacy_id_hash
                .entry(current_ascend_class_name)
                .or_insert_with(|| {
                    let new_id = current_ascendancy_id;
                    current_ascendancy_id += 1;
                    new_id
                }).clone();

    for node_entry in nodes_table.pairs()
    {
        let (node_id, lua_node_table): (i64, LuaTable) = node_entry.unwrap();

        let lua_node_type: String = lua_node_table.get("type").unwrap();
        let node_name: String = lua_node_table.get("name").unwrap();

        let node_type =
            if lua_node_type == "Mastery"
            {
                NodeType::MASTERY
            }
            else if lua_node_type == "ClassStart"
            {
                NodeType::ClassStart
            }
            else if lua_node_type == "AscendClassStart"
            {
                NodeType::AscendClassStart
            }
            else
            {
                NodeType::NORMAL
            };

        let node_alloc: bool = lua_node_table.get("alloc").unwrap();

        let lua_node_ascend_name: Option<String> = lua_node_table.get("ascendancyName").unwrap();

        let node_ascend_id =
            match lua_node_ascend_name {
                None => {
                    None
                }
                Some(ascend_name) => {
                    Some(ascendacy_id_hash
                        .entry(ascend_name)
                        .or_insert_with(|| {
                            let new_id = current_ascendancy_id;
                            current_ascendancy_id += 1;
                            new_id
                        }).clone())
                }
            };

        let node = Node {
            id: node_id,
            name: node_name,
            tree_node_index: 0,
            mastery_index: 0,
            node_type: node_type.clone(),
            linked_indexes: Vec::with_capacity(100),
            alloc: node_alloc,
            ascendancy_id: node_ascend_id,
            default_alloc: node_alloc,
            path_index: usize::MAX
        };

        tree_nodes.push(RefCell::new(node));
    }

    tree_nodes.sort_unstable_by(|a, b| b.borrow().id.cmp(&a.borrow().id));

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let mut masteries = Vec::new();
    let mut masteries_hash_node_indexes = HashMap::new();
    for (node_index, node) in tree_nodes.iter().enumerate()
    {
        let mut node = node.borrow_mut();
        node.tree_node_index = node_index;
        node_id_index_map.insert(node.id, node_index);

        match node.node_type {
            NodeType::MASTERY => {
                let masteries_node_indexes = {
                    match masteries_hash_node_indexes.get_mut(&node.name)
                    {
                        None => {
                            let mut masteries_node_indexes = Vec::new();

                            let node_table: LuaTable = nodes_table.get(node.id).unwrap();
                            let mastery_effects_table: LuaTable = node_table.get("masteryEffects").unwrap();
                            let mut mastery_effects = Vec::new();

                            for entry_effect in mastery_effects_table.pairs()
                            {
                                let (_, effect_table): (LuaValue, LuaTable) = entry_effect.unwrap();

                                let effect_id: i64 = effect_table.get("effect").unwrap();

                                mastery_effects.push(MasteryEffect {
                                    id: effect_id
                                });
                            }

                            mastery_effects.sort_unstable_by(|a, b| b.id.cmp(&a.id));

                            masteries_node_indexes.push(node_index);

                            masteries.push(RefCell::new(Mastery {
                                name: node.name.clone(),
                                effects: mastery_effects,
                                effects_indexes_to_select: Vec::with_capacity(10),
                                effect_next_select_index: 0,
                            }));

                            masteries_hash_node_indexes.insert(node.name.clone(), masteries_node_indexes);

                            masteries_hash_node_indexes.get_mut(&node.name).unwrap()
                        }
                        Some(masteries_node_indexes) => masteries_node_indexes
                    }
                };

                masteries_node_indexes.push(node_index);
            }
            _ => {}
        }
    }

    masteries.sort_unstable_by(|a, b|
                                    {
                                        let a = a.borrow();
                                        let b = b.borrow();
                                        b.name.cmp(&a.name)
                                    });

    for (mastery_index, mastery) in masteries.iter().enumerate()
    {
        let mastery = mastery.borrow();

        let node_indexes = masteries_hash_node_indexes.get(&mastery.name).unwrap();

        for node_index in node_indexes
        {
            let mut node = tree_nodes[node_index.clone()].borrow_mut();

            node.mastery_index = mastery_index;
        }
    }

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();
    for node_entry in nodes_table.pairs()
    {
        let (_, lua_node_table): (i64, LuaTable) = node_entry.unwrap();

        let table_linked: LuaTable = lua_node_table.get("linked").unwrap();

        let node_id: i64 = lua_node_table.get("id").unwrap();

        let node_index = node_id_index_map.get(&node_id).unwrap();

        let node = &mut tree_nodes[node_index.clone()].borrow_mut();

        for linked_node_entry in table_linked.pairs()
        {
            let (_, lua_linked_node_table): (i64, LuaTable) = linked_node_entry.unwrap();

            let linked_node_id: i64 = lua_linked_node_table.get("id").unwrap();

            let linked_node_index = node_id_index_map.get(&linked_node_id).unwrap();

            node.linked_indexes.push(linked_node_index.clone());
        }

        node.linked_indexes.sort_unstable_by(|a, b| {
            let a = &tree_nodes[a.clone()].borrow();
            let b = &tree_nodes[b.clone()].borrow();

            b.borrow().id.cmp(&a.borrow().id)
        });
    }

    let tree_nodes_len = tree_nodes.len().clone();

    let mut adjust_space = AdjustSpace {
        space_indexes: HashMap::new(),
        count_indexes: 0
    };

    let mut queue_indexes = Vec::new();

    for root in &tree_nodes
    {
        {
            let mut root = root.borrow_mut();

            if root.alloc == false
            {
                continue
            }

            queue_indexes.clear();
            queue_indexes.push(root.tree_node_index);
        }

        let mut o = 0; // out
        let mut i = 1; // in

        while o < i
        {
            let mut node = tree_nodes[queue_indexes[o.clone()]].borrow_mut();

            o += 1;

            node.alloc = true;

            for linked_index in &node.linked_indexes
            {
                let mut linked_node = tree_nodes[*linked_index].borrow_mut();

                adjust_space.try_add_nodes(node.tree_node_index, *linked_index);

                if linked_node.alloc == false
                {
                    linked_node.alloc = true;

                    queue_indexes.push(linked_node.tree_node_index);

                    i += 1;
                }
            }
        }
    }

    DnaEncoder {
        tree_nodes,
        masteries,
        adjust_space,
        queue_indexes_buffer: Vec::with_capacity(tree_nodes_len)
    }
}
