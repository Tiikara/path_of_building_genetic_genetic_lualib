use std::borrow::{Borrow};
use std::cell::{RefCell};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use mlua::{Lua, TableExt, UserData, UserDataMethods};
use mlua::prelude::{LuaResult, LuaString, LuaTable, LuaValue};
use crate::dna::{Dna, LuaDna};

use crate::worker::LuaDnaCommand;

pub struct DnaEncoder
{
    tree_nodes: Vec<RefCell<Node>>,
    masteries: Vec<RefCell<Mastery>>,

    path_indexes_buf: Vec<usize>,
    index_nodes_to_allocate: HashSet<usize>,
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

impl DnaEncoder {
    pub fn convert_dna_to_build(&mut self, build_table: &LuaTable, dna: &Dna, max_number_normal_nodes_to_allocate: usize, max_number_ascend_nodes_to_allocate: usize) -> DnaConvertResult
    {
        let mut queue_indexes = Vec::new();

        std::mem::swap(&mut queue_indexes, &mut self.queue_indexes_buffer);

        for (_node_index, node) in self.tree_nodes.iter().enumerate()
        {
            let mut node = node.borrow_mut();

            node.alloc = node.default_alloc;

            if node.alloc
            {
                node.path_dist = 0;
            }
            else
            {
                node.path_dist = usize::MAX;
            }

            node.path_indexes.clear();
        }

        for mastery in &self.masteries
        {
            let mut mastery = mastery.borrow_mut();

            mastery.effect_next_select_index = 0;
            mastery.effects_indexes_to_select.clear();
        }

        for node in self.tree_nodes.iter()
        {
            let is_allocated =
                {
                    node.borrow().alloc
                };

            if is_allocated
            {
                self.build_path_from_node(&mut queue_indexes, node);
            }
        }

        self.index_nodes_to_allocate.clear();

        for (tree_node_index, nucl) in dna.body_nodes.iter().enumerate()
        {
            if *nucl == 1
            {
                self.index_nodes_to_allocate.insert(tree_node_index);
            }
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
        let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();
        let nodes_table: LuaTable = spec_table.get("nodes").unwrap();
        let alloc_nodes_table: LuaTable = spec_table.get("allocNodes").unwrap();

        let mut allocated_normal_nodes = 0;
        let mut allocated_ascend_nodes = 0;
        while self.index_nodes_to_allocate.is_empty() == false
        {
            let mut smallest_node_index = usize::MAX;
            let mut smallest_node_path_dist = 0;

            for index_node in &self.index_nodes_to_allocate
            {
                let node = self.tree_nodes[index_node.clone()].borrow();

                if smallest_node_index == usize::MAX || smallest_node_path_dist > node.path_dist
                {
                    smallest_node_path_dist = node.path_dist;
                    smallest_node_index = index_node.clone();
                }
            }

            if smallest_node_index == usize::MAX
            {
                break;
            }

            self.index_nodes_to_allocate.remove(&smallest_node_index);

            let node = &self.tree_nodes[smallest_node_index];

            let has_path =
                {
                    let node = node.borrow();

                    node.path_indexes.is_empty() == false
                };

            if has_path == false
            {
                continue;
            }

            let path_indexes = {
                {
                    let mut node = node.borrow_mut();

                    std::mem::swap(&mut node.path_indexes, &mut self.path_indexes_buf);
                }

                self.path_indexes_buf.sort_unstable_by(|a, b| {
                    let a = &self.tree_nodes[a.clone()].borrow();
                    let b = &self.tree_nodes[b.clone()].borrow();

                    a.path_dist.cmp(&b.path_dist)
                });

                &self.path_indexes_buf
            };

            for path_index in path_indexes
            {
                let path_node = &self.tree_nodes[path_index.clone()];

                let (is_allocated, is_ascend) =
                    {
                        let mut path_node = path_node.borrow_mut();

                        if path_node.alloc {
                            continue;
                        }

                        if path_node.ascendancy_id == usize::MAX
                        {
                            if allocated_normal_nodes == max_number_normal_nodes_to_allocate {
                                break;
                            }
                        }
                        else
                        {
                            if allocated_ascend_nodes == max_number_ascend_nodes_to_allocate {
                                break;
                            }
                        }

                        let is_allocated =
                            match path_node.node_type {
                                NodeType::NORMAL => true,
                                NodeType::MASTERY => {
                                    let mut mastery = self.masteries[path_node.mastery_index].borrow_mut();

                                    if mastery.effect_next_select_index < mastery.effects_indexes_to_select.len()
                                    {
                                        let effect_index = mastery.effects_indexes_to_select[mastery.effect_next_select_index];

                                        let effect_id = mastery.effects[effect_index].id;

                                        mastery_selections_table.set(path_node.id, effect_id).unwrap();

                                        let effect_table: LuaTable = mastery_effects_table.get(effect_id).unwrap();

                                        let lua_sd: LuaValue = effect_table.get("sd").unwrap();

                                        let node_table: LuaTable = nodes_table.get(path_node.id).unwrap();

                                        node_table.set("sd", lua_sd).unwrap();
                                        node_table.set("allMasteryOptions", false).unwrap();

                                        let _: LuaValue = tree_table.call_method("ProcessStats", node_table).unwrap();

                                        mastery.effect_next_select_index += 1;

                                        true
                                    }
                                    else
                                    {
                                        false
                                    }
                                },
                                _ => false
                            };

                        (is_allocated, path_node.ascendancy_id != usize::MAX)
                    };

                if is_allocated
                {
                    let need_build_path =
                        {
                            let mut path_node = path_node.borrow_mut();

                            let node_table: LuaTable = nodes_table.get(path_node.id).unwrap();
                            node_table.set("alloc", true).unwrap();
                            alloc_nodes_table.set(path_node.id, node_table).unwrap();

                            path_node.alloc = true;

                            match path_node.node_type {
                                NodeType::MASTERY => false,
                                _ => true
                            }
                        };

                    if need_build_path
                    {
                        self.build_path_from_node(&mut queue_indexes, path_node);
                    }

                    if is_ascend == false
                    {
                        allocated_normal_nodes += 1;
                    }
                    else
                    {
                        allocated_ascend_nodes += 1;
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

    // Perform a breadth-first search of the tree, starting from this node, and determine if it is the closest node to any other nodes
    // alg from PassiveSpec.lua (function PassiveSpecClass:BuildPathFromNode(root))
    fn build_path_from_node(&self, queue_indexes: &mut Vec<usize>, root: &RefCell<Node>)
    {
        {
            let mut root = root.borrow_mut();

            root.path_dist = 0;
            root.path_indexes.clear();

            queue_indexes.clear();
            queue_indexes.push(root.tree_node_index);
        }

        let mut o = 0; // out
        let mut i = 1; // in

        while o < i
        {
            let node = self.tree_nodes[queue_indexes[o.clone()]].borrow();

            o += 1;

            let cur_dist = node.path_dist + 1;

            for linked_index in &node.linked_indexes
            {
                let mut other = self.tree_nodes[*linked_index].borrow_mut();

                match other.node_type {
                    NodeType::NORMAL => {
                        if node.ascendancy_id == other.ascendancy_id || (cur_dist == 1 && other.ascendancy_id == usize::MAX)
                        {
                            if other.path_dist > cur_dist
                            {
                                other.path_dist = cur_dist;
                                other.path_indexes.clear();

                                let other_node_index = other.tree_node_index.clone();

                                other.path_indexes.push(other_node_index);
                                for node_path_index in node.path_indexes.iter()
                                {
                                    other.path_indexes.push(node_path_index.clone())
                                }

                                queue_indexes.push(other.tree_node_index);

                                i += 1;
                            }
                        }
                    }
                    NodeType::MASTERY => {
                        if other.path_dist > cur_dist
                        {
                            other.path_dist = cur_dist;
                            other.path_indexes.clear();

                            let other_node_index = other.tree_node_index.clone();

                            other.path_indexes.push(other_node_index);
                            for node_path_index in node.path_indexes.iter()
                            {
                                other.path_indexes.push(node_path_index.clone())
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

impl UserData for DnaEncoder {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("ConvertDnaToBuild", |lua_context, this, (build_table, dna, max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate): (LuaTable, LuaDna, usize, usize)| {
            Ok(this.convert_dna_to_build(&build_table, dna.reference.borrow(), max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate).get_table(lua_context))
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
    path_indexes: Vec<usize>,
    path_dist: usize,
    alloc: bool,
    ascendancy_id: usize,
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
                    usize::MAX
                }
                Some(ascend_name) => {
                    ascendacy_id_hash
                        .entry(ascend_name)
                        .or_insert_with(|| {
                            let new_id = current_ascendancy_id;
                            current_ascendancy_id += 1;
                            new_id
                        }).clone()
                }
            };

        let node = Node {
            id: node_id,
            name: node_name,
            tree_node_index: 0,
            mastery_index: 0,
            node_type: node_type.clone(),
            linked_indexes: Vec::with_capacity(100),
            path_indexes: Vec::with_capacity(1000),
            path_dist: 0,
            alloc: false,
            ascendancy_id: node_ascend_id,
            default_alloc: node_alloc
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
    }

    let tree_nodes_len = tree_nodes.len().clone();

    DnaEncoder {
        tree_nodes,
        masteries,
        path_indexes_buf: Vec::with_capacity(1000),
        index_nodes_to_allocate: HashSet::with_capacity(tree_nodes_len),
        queue_indexes_buffer: Vec::with_capacity(tree_nodes_len)
    }
}
