use std::borrow::{Borrow};
use std::cell::{RefCell};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use mlua::{Lua, TableExt, UserData, UserDataMethods};
use mlua::prelude::{LuaResult, LuaString, LuaTable, LuaValue};
use crate::dna::{Dna, LuaDna};

use crate::worker::LuaDnaCommand;

pub struct DnaEncoder
{
    tree_nodes: Vec<RefCell<Node>>,
    mastery_nodes: Vec<RefCell<MasteryNode>>
}

impl DnaEncoder {
    pub fn convert_dna_to_build<'a>(&mut self, lua_context: &'a Lua, build_table: LuaTable, dna: &Dna, max_number_normal_nodes_to_allocate: usize, max_number_ascend_nodes_to_allocate: usize) -> LuaTable<'a>
    {
        for (_node_index, node) in self.tree_nodes.iter().enumerate()
        {
            let mut node = node.borrow_mut();

            node.planned_alloc = node.default_alloc;

            if node.planned_alloc
            {
                node.path_dist = 0;
            }
            else
            {
                node.path_dist = usize::MAX;
            }

            node.path_indexes.clear();
        }

        for mastery_node in &self.mastery_nodes
        {
            let mut mastery_node = mastery_node.borrow_mut();

            mastery_node.effect_next_select_index = 0;
            mastery_node.effects_indexes_to_select.clear();
        }

        for node in self.tree_nodes.iter()
        {
            let is_allocated =
                {
                    node.borrow().planned_alloc
                };

            if is_allocated
            {
                self.build_path_from_node(node);
            }
        }

        let mut index_nodes_to_allocate = HashSet::new();

        for (tree_node_index, nucl) in dna.body_nodes.iter().enumerate()
        {
            if *nucl == 1
            {
                index_nodes_to_allocate.insert(tree_node_index);
            }
        }

        for (index, nucl) in dna.body_masteries.iter().enumerate()
        {
            if *nucl == 1
            {
                let mastery_node_index = index / 6;
                let effect_index = index % 6;

                let mut mastery_node = self.mastery_nodes[mastery_node_index].borrow_mut();

                if effect_index < mastery_node.effects.len()
                {
                    mastery_node.effects_indexes_to_select.push(effect_index);
                }
            }
        }

        let mut allocated_normal_nodes = 0;
        let mut allocated_ascend_nodes = 0;
        while index_nodes_to_allocate.is_empty() == false
        {
            let mut smallest_node_index = usize::MAX;
            let mut smallest_node_path_dist = 0;

            for index_node in &index_nodes_to_allocate
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

            index_nodes_to_allocate.remove(&smallest_node_index);

            let has_path =
                {
                    let node = self.tree_nodes[smallest_node_index].borrow();

                    node.path_indexes.is_empty() == false
                };

            if has_path == false
            {
                continue;
            }

            let path_indexes = {
                let node = self.tree_nodes[smallest_node_index].borrow();

                node.path_indexes.clone()
            };

            for path_index in path_indexes
            {
                let is_ascend =
                    {
                        let path_node = self.tree_nodes[path_index.clone()].borrow();

                        path_node.is_ascend
                    };

                let is_allocated =
                    {
                        let mut path_node = self.tree_nodes[path_index.clone()].borrow_mut();

                        if is_ascend == false
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

                        match path_node.node_type {
                            NodeType::NORMAL | NodeType::MASTERY => {
                                path_node.planned_alloc = true;

                                true
                            },
                            _ => false
                        }
                    };

                if is_allocated
                {
                    self.build_path_from_node(&self.tree_nodes[path_index.clone()]);

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

        let spec_table: LuaTable = build_table.get("spec").unwrap();
        let mastery_selections_table: LuaTable = spec_table.get("masterySelections").unwrap();
        let tree_table: LuaTable = spec_table.get("tree").unwrap();
        let mastery_effects_table: LuaTable = tree_table.get("masteryEffects").unwrap();
        let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();
        let nodes_table: LuaTable = spec_table.get("nodes").unwrap();
        let alloc_nodes_table: LuaTable = spec_table.get("allocNodes").unwrap();

        for node in &self.tree_nodes
        {
            let node = node.borrow();

            if node.planned_alloc
            {
                let need_allocate =
                    match node.node_type {
                        NodeType::MASTERY => {
                            let mut mastery_node = self.mastery_nodes[node.mastery_node_index].borrow_mut();

                            if mastery_node.effect_next_select_index < mastery_node.effects_indexes_to_select.len()
                            {
                                let effect_index = mastery_node.effects_indexes_to_select[mastery_node.effect_next_select_index];

                                let effect_id = mastery_node.effects[effect_index].id;

                                mastery_selections_table.set(node.id, effect_id).unwrap();

                                let effect_table: LuaTable = mastery_effects_table.get(effect_id).unwrap();

                                let lua_sd: LuaValue = effect_table.get("sd").unwrap();

                                let node_table: LuaTable = nodes_table.get(node.id).unwrap();

                                node_table.set("sd", lua_sd).unwrap();
                                node_table.set("allMasteryOptions", false).unwrap();

                                let _: LuaValue = tree_table.call_method("ProcessStats", node_table).unwrap();

                                mastery_node.effect_next_select_index += 1;

                                true
                            }
                            else
                            {
                                false
                            }
                        }
                        _ => true
                    };

                if need_allocate
                {
                    let node_table: LuaTable = nodes_table.get(node.id).unwrap();
                    node_table.set("alloc", true).unwrap();
                    alloc_nodes_table.set(node.id, node_table).unwrap();
                }
            }
        }

        let res_table = lua_context.create_table().unwrap();

        res_table.set("usedNormalNodeCount", allocated_normal_nodes).unwrap();
        res_table.set("usedAscendancyNodeCount", 6).unwrap();

        res_table
    }

    // Perform a breadth-first search of the tree, starting from this node, and determine if it is the closest node to any other nodes
    // alg from PassiveSpec.lua (function PassiveSpecClass:BuildPathFromNode(root))
    fn build_path_from_node(&self, root: &RefCell<Node>)
    {
        let mut queue_indexes = {
            let mut root = root.borrow_mut();

            root.path_dist = 0;
            root.path_indexes.clear();

            let mut queue_indexes = Vec::with_capacity(1000);

            queue_indexes.push(root.tree_node_index);

            queue_indexes
        };

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
                    NodeType::NORMAL | NodeType::MASTERY => {
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
                    NodeType::ClassStart => {}
                    NodeType::AscendClassStart => {}
                }
            }
        }
    }
}

impl UserData for DnaEncoder {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("ConvertDnaToBuild", |lua_context, this, (build_table, dna, max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate): (LuaTable, LuaDna, usize, usize)| {
            Ok(this.convert_dna_to_build(lua_context, build_table, dna.reference.borrow(), max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate))
        });

        methods.add_method_mut("ConvertDnaCommandHandlerToBuild", |lua_context, this,
                                                                   (build_table, dna_command, max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate): (LuaTable, LuaDnaCommand, usize, usize)| {
            let dna_command = dna_command.reference.deref();

            Ok(match dna_command.borrow().as_ref() {
               Some(dna_command) => {
                   match &dna_command.dna {
                       None => panic!("Dna is not exists in dna command"),
                       Some(dna) => this.convert_dna_to_build(lua_context, build_table, dna, max_number_normal_nodes_to_allocate, max_number_ascend_nodes_to_allocate)
                   }
               },
               None => panic!("Dna command is not exists in handler")
            })
        });

        methods.add_method("GetTreeNodesCount", |_lua_context, this, ()| {
            Ok(this.tree_nodes.len())
        });

        methods.add_method("GetMasteryNodesCount", |_lua_context, this, ()| {
            Ok(this.mastery_nodes.len())
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
    node_type: NodeType,
    tree_node_index: usize,
    mastery_node_index: usize,
    id: i64,
    linked_indexes: Vec<usize>,
    path_indexes: Vec<usize>,
    path_dist: usize,
    planned_alloc: bool,
    is_ascend: bool,
    default_alloc: bool
}

struct MasteryNode
{
    node_index: usize,
    effects: Vec<MasteryEffect>,
    effects_indexes_to_select: Vec<usize>,
    effect_next_select_index: usize
}

struct MasteryEffect
{
    id: i64
}

pub fn create_dna_encoder(_: &Lua, build_table: LuaTable) -> LuaResult<DnaEncoder>
{
    let spec_table: LuaTable = build_table.get("spec").unwrap();

    let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();
    let _: LuaValue = spec_table.call_method("BuildAllDependsAndPaths", 0).unwrap();

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let count_nodes = nodes_table.len().unwrap();

    let mut tree_nodes = Vec::with_capacity(count_nodes as usize);

    let mut node_id_index_map = HashMap::new();

    for node_entry in nodes_table.pairs()
    {
        let (node_id, lua_node_table): (i64, LuaTable) = node_entry.unwrap();

        let lua_node_type: String = lua_node_table.get("type").unwrap();

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

        let lua_node_ascend_name: Option<LuaString> = lua_node_table.get("ascendancyName").unwrap();

        let is_ascend_node =
            match lua_node_ascend_name {
                None => {false}
                Some(_) => {true}
            };

        let node = Node {
            id: node_id,
            tree_node_index: 0,
            mastery_node_index: 0,
            node_type: node_type.clone(),
            linked_indexes: Vec::with_capacity(100),
            path_indexes: Vec::with_capacity(1000),
            path_dist: 0,
            planned_alloc: false,
            is_ascend: is_ascend_node,
            default_alloc: node_alloc
        };

        tree_nodes.push(RefCell::new(node));
    }

    tree_nodes.sort_unstable_by(|a, b| b.borrow().id.cmp(&a.borrow().id));

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let mut mastery_nodes = Vec::new();
    for (node_index, node) in tree_nodes.iter().enumerate()
    {
        let mut node = node.borrow_mut();
        node.tree_node_index = node_index;
        node_id_index_map.insert(node.id, node_index);

        match node.node_type {
            NodeType::MASTERY => {
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

                node.mastery_node_index = mastery_nodes.len();

                mastery_nodes.push(RefCell::new(MasteryNode {
                    node_index,
                    effects: mastery_effects,
                    effects_indexes_to_select: Vec::with_capacity(10),
                    effect_next_select_index: 0,
                }));
            }
            _ => {}
        }
    }

    mastery_nodes.sort_unstable_by(|a, b|
                                    {
                                        let a = tree_nodes[a.borrow().node_index].borrow();
                                        let b = tree_nodes[b.borrow().node_index].borrow();
                                        b.borrow().id.cmp(&a.borrow().id)
                                    });

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

    Ok(DnaEncoder {
        tree_nodes,
        mastery_nodes,
    })
}
