use std::borrow::{Borrow, BorrowMut};
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet, LinkedList};
use std::ops::Deref;
use std::rc::Rc;
use mlua::{Lua, TableExt, UserData, UserDataMethods};
use mlua::prelude::{LuaResult, LuaString, LuaTable, LuaValue};
use crate::dna::{Dna, LuaDna};
use crate::genetic::DnaCommand;
use crate::worker::LuaDnaCommand;

pub struct DnaEncoder
{
    tree_nodes: Vec<RefCell<Node>>,
    alloc_nodes_indexes: Vec<usize>
}

struct NodeChangeInfo
{
    node_index: usize,
    path_indexes: Vec<usize>,
    path_dist: usize,
}

impl DnaEncoder {
    pub fn convert_dna_to_build<'a>(&mut self, lua_context: &'a Lua, build_table: LuaTable, dna: &Dna) -> LuaTable<'a>
    {
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

        for node in self.tree_nodes.iter()
        {
            let is_allocated =
                {
                    node.borrow().alloc
                };

            if is_allocated
            {
                self.build_path_from_node(node);
            }
        }

        let count_normal_nodes_to_allocate = 107;
        let count_ascend_nodes_to_allocate = 6;

        let mut index_nodes_to_allocate = HashSet::new();

        for (tree_node_index, nucl) in dna.body_nodes.iter().enumerate()
        {
            if *nucl == 1
            {
                index_nodes_to_allocate.insert(tree_node_index);
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
                            if allocated_normal_nodes == count_normal_nodes_to_allocate {
                                break;
                            }
                        }
                        else
                        {
                            if allocated_ascend_nodes == count_ascend_nodes_to_allocate {
                                break;
                            }
                        }

                        match path_node.node_type {
                            NodeType::NORMAL => {
                                path_node.alloc = true;

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
        let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();
        let nodes_table: LuaTable = spec_table.get("nodes").unwrap();
        let alloc_nodes_table: LuaTable = spec_table.get("allocNodes").unwrap();

        for node in &self.tree_nodes
        {
            let node = node.borrow();

            if node.alloc
            {
                let node_table: LuaTable = nodes_table.get(node.id).unwrap();
                node_table.set("alloc", true).unwrap();
                alloc_nodes_table.set(node.id, node_table).unwrap();
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
                    NodeType::NORMAL => {
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
                    NodeType::MASTERY => {}
                    NodeType::ClassStart => {}
                    NodeType::AscendClassStart => {}
                }
            }
        }
    }
}

impl UserData for DnaEncoder {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("ConvertDnaToBuild", |lua_context, this, (build_table, dna): (LuaTable, LuaDna)| {
            Ok(this.convert_dna_to_build(lua_context, build_table, dna.reference.borrow()))
        });

        methods.add_method_mut("ConvertDnaCommandHandlerToBuild", |lua_context, this, (build_table, mut dna_command): (LuaTable, LuaDnaCommand)| {
            let dna_command = dna_command.reference.deref();

            Ok(match dna_command.borrow().as_ref() {
               Some(dna_command) => {
                   match &dna_command.dna {
                       None => panic!("Dna is not exists in dna command"),
                       Some(dna) => this.convert_dna_to_build(lua_context, build_table, dna)
                   }
               },
               None => panic!("Dna command is not exists in handler")
            })
        });

        methods.add_method("GetTreeNodesCount", |lua_context, this, ()| {
            Ok(this.tree_nodes.len())
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
    id: i64,
    linked_indexes: Vec<usize>,
    path_indexes: Vec<usize>,
    path_dist: usize,
    alloc: bool,
    is_ascend: bool,
    default_alloc: bool
}

struct MasteryNode
{
    node_index: usize
}

pub fn create_dna_encoder(lua_context: &Lua, build_table: LuaTable) -> LuaResult<DnaEncoder>
{
    let spec_table: LuaTable = build_table.get("spec").unwrap();

    let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let count_nodes = nodes_table.len().unwrap();

    let mut tree_nodes = Vec::with_capacity(count_nodes as usize);

    // TODO
    //let mut mastery_nodes = Vec::new();

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
            node_type: node_type.clone(),
            linked_indexes: vec![],
            path_indexes: vec![],
            path_dist: 0,
            alloc: false,
            is_ascend: is_ascend_node,
            default_alloc: node_alloc
        };

        match node_type {
            NodeType::MASTERY => {
                // TODO
                /*mastery_nodes.push(MasteryNode {
                    0
                })*/
            }
            _ => {}
        }



        tree_nodes.push(RefCell::new(node));
    }

    tree_nodes.sort_unstable_by(|a, b| b.borrow().id.cmp(&a.borrow().id));

    for (node_index, node) in tree_nodes.iter().enumerate()
    {
        let mut node = node.borrow_mut();
        node.tree_node_index = node_index;
        node_id_index_map.insert(node.id, node_index);
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

    let tree_nodes_len = tree_nodes.len();

    Ok(DnaEncoder {
        tree_nodes,
        alloc_nodes_indexes: Vec::with_capacity(tree_nodes_len)
    })
}
