use std::borrow::{Borrow, BorrowMut};
use std::cell::{RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut};
use mlua::{Lua, TableExt, UserData, UserDataMethods};
use mlua::prelude::{LuaResult, LuaString, LuaTable, LuaValue};
use ouroboros::self_referencing;
use crate::dna::{Dna, LuaDna};

use crate::worker::LuaDnaCommand;

enum NodeType<'a>
{
    NORMAL,
    MASTERY(MasteryVariant<'a>),
    ClassStart,
    AscendClassStart
}

struct MasteryVariant<'a> {
    mastery: &'a mut Mastery
}

impl<'a> Deref for NodeWrapper<'a> {
    type Target = Node;
    fn deref(&self) -> &Node { &self.node }
}

impl<'a> DerefMut for NodeWrapper<'a> {
    fn deref_mut(&mut self) -> &mut Node { &mut self.node }
}

struct NodeWrapper<'a>
{
    node: &'a mut Node,
    node_type: NodeType<'a>,
    linked_nodes: Vec<&'a mut NodeWrapper<'a>>,
    path_nodes: Vec<&'a mut NodeWrapper<'a>>
}

struct Node
{
    name: String,
    id: i64,
    path_dist: usize,
    planned_alloc: bool,
    is_ascend: bool,
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

pub struct DnaEncoderData2<'this>
{
    tree_nodes: Vec<&'this mut NodeWrapper<'this>>,
    masteries: Vec<&'this mut Mastery>,
    path_nodes_buf: Vec<&'this mut NodeWrapper<'this>>,
    nodes_to_allocate: HashMap<usize, &'this mut NodeWrapper<'this>>,
    queue_nodes_buffer: VecDeque<&'this mut NodeWrapper<'this>>
}

#[self_referencing]
pub struct DnaEncoderData<'a>
{
    tree_nodes_wrapper_buffer: Vec<NodeWrapper<'a>>,

    #[borrows(mut tree_nodes_wrapper_buffer)]
    #[covariant]
    data: DnaEncoderData2<'this>
}

#[self_referencing]
pub struct DnaEncoder
{
    tree_nodes_buffer: Vec<Node>,
    masteries_buffer: Vec<Mastery>,

    #[borrows(mut tree_nodes_buffer, mut masteries_buffer)]
    #[covariant]
    data: DnaEncoderData<'this>
}

impl DnaEncoder {
    pub fn convert_dna_to_build<'a>(&mut self, lua_context: &'a Lua, build_table: LuaTable, dna: &Dna, max_number_normal_nodes_to_allocate: usize, max_number_ascend_nodes_to_allocate: usize) -> LuaTable<'a>
    {
        self.with_data_mut(|dna_encoder_data: &mut DnaEncoderData|{
            dna_encoder_data.with_data_mut(|dna_encoder_data: &mut DnaEncoderData2|{
                let mut queue_nodes = VecDeque::new();

                std::mem::swap(&mut queue_nodes, &mut dna_encoder_data.queue_nodes_buffer);

                for mut node in &dna_encoder_data.tree_nodes
                {
                    node.planned_alloc = node.default_alloc;

                    if node.planned_alloc
                    {
                        node.path_dist = 0;
                    }
                    else
                    {
                        node.path_dist = usize::MAX;
                    }

                    node.path_nodes.clear();
                }

                for mastery in dna_encoder_data.masteries
                {
                    mastery.effect_next_select_index = 0;
                    mastery.effects_indexes_to_select.clear();
                }

                for node in dna_encoder_data.tree_nodes
                {
                    if node.planned_alloc
                    {
                        build_path_from_node(&mut queue_nodes, node);
                    }
                }

                dna_encoder_data.nodes_to_allocate.clear();

                for (tree_node_index, nucl) in dna.body_nodes.iter().enumerate()
                {
                    if *nucl == 1
                    {
                        dna_encoder_data.nodes_to_allocate.insert( tree_node_index, dna_encoder_data.tree_nodes[tree_node_index]);
                    }
                }

                for (index, nucl) in dna.body_masteries.iter().enumerate()
                {
                    if *nucl == 1
                    {
                        let mastery_index = index / 6;
                        let effect_index = index % 6;

                        let mut mastery = dna_encoder_data.masteries[mastery_index];

                        if effect_index < mastery.effects.len()
                        {
                            mastery.effects_indexes_to_select.push(effect_index);
                        }
                    }
                }

                let mut allocated_normal_nodes = 0;
                let mut allocated_ascend_nodes = 0;
                while dna_encoder_data.nodes_to_allocate.is_empty() == false
                {
                    let mut smallest_node_index = usize::MAX;
                    let mut smallest_node = None;
                    let mut smallest_node_path_dist = 0;

                    for (index_node, node) in dna_encoder_data.nodes_to_allocate
                    {
                        if smallest_node_index == usize::MAX || smallest_node_path_dist > node.path_dist
                        {
                            smallest_node_path_dist = node.path_dist;
                            smallest_node_index = index_node.clone();
                            smallest_node = Some(node);
                        }
                    }

                    let node =
                        match smallest_node {
                            None => break,
                            Some(smallest_node) => smallest_node
                        };

                    dna_encoder_data.nodes_to_allocate.remove(&smallest_node_index);

                    if node.path_nodes.is_empty() == false
                    {
                        continue;
                    }

                    std::mem::swap(&mut node.path_nodes, &mut dna_encoder_data.path_nodes_buf);

                    for path_node in &dna_encoder_data.path_nodes_buf
                    {
                        let (is_allocated, is_ascend) =
                            {
                                if path_node.is_ascend == false
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
                                        NodeType::NORMAL | NodeType::MASTERY(_) => {
                                            path_node.planned_alloc = true;

                                            true
                                        },
                                        _ => false
                                    };

                                (is_allocated, path_node.is_ascend)
                            };

                        if is_allocated
                        {
                            build_path_from_node(&mut queue_nodes, *path_node);

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

                for node in dna_encoder_data.tree_nodes
                {
                    if node.planned_alloc
                    {
                        let need_allocate =
                            match &node.node_type {
                                NodeType::MASTERY(mastery_variant) => {
                                    let mastery = mastery_variant.mastery;

                                    if mastery.effect_next_select_index < mastery.effects_indexes_to_select.len()
                                    {
                                        let effect_index = mastery.effects_indexes_to_select[mastery.effect_next_select_index];

                                        let effect_id = mastery.effects[effect_index].id;

                                        mastery_selections_table.set(node.id, effect_id).unwrap();

                                        let effect_table: LuaTable = mastery_effects_table.get(effect_id).unwrap();

                                        let lua_sd: LuaValue = effect_table.get("sd").unwrap();

                                        let node_table: LuaTable = nodes_table.get(node.id).unwrap();

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
                res_table.set("usedAscendancyNodeCount", allocated_ascend_nodes).unwrap();

                // restore buffers
                std::mem::swap(&mut queue_nodes, &mut dna_encoder_data.queue_nodes_buffer);

                res_table
            })
        });

        lua_context.create_table().unwrap()
    }
}

// Perform a breadth-first search of the tree, starting from this node, and determine if it is the closest node to any other nodes
// alg from PassiveSpec.lua (function PassiveSpecClass:BuildPathFromNode(root))
fn build_path_from_node<'a>(queue: &mut VecDeque<&'a mut NodeWrapper<'a>>, root: &'a mut NodeWrapper<'a>)
{
    {
        root.path_dist = 0;
        root.path_nodes.clear();

        queue.clear();
        queue.push_back(root);
    }

    let mut o = 0; // out
    let mut i = 1; // in

    while o < i
    {
        let node = queue.pop_front().unwrap();

        o += 1;

        let cur_dist = node.path_dist + 1;

        for other in &node.linked_nodes
        {
            match other.node_type {
                NodeType::NORMAL | NodeType::MASTERY(_) => {
                    if other.path_dist > cur_dist
                    {
                        other.path_dist = cur_dist;
                        other.path_nodes.clear();

                        other.path_nodes.push(*other);
                        for node_path in node.path_nodes
                        {
                            other.path_nodes.push(node_path)
                        }

                        queue.push_back(*other);

                        i += 1;
                    }
                }
                NodeType::ClassStart => {}
                NodeType::AscendClassStart => {}
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
            Ok(this.borrow_data().borrow_data().tree_nodes.len())
        });

        methods.add_method("GetMasteryCount", |_lua_context, this, ()| {
            Ok(this.borrow_data().borrow_data().masteries.len())
        });
    }
}

pub fn create_dna_encoder(_: &Lua, build_table: LuaTable) -> LuaResult<DnaEncoder>
{
    let spec_table: LuaTable = build_table.get("spec").unwrap();

    let _: LuaValue = spec_table.call_method("ResetNodes", 0).unwrap();
    let _: LuaValue = spec_table.call_method("BuildAllDependsAndPaths", 0).unwrap();

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let count_nodes = nodes_table.len().unwrap();

    let mut tree_nodes = Vec::with_capacity(count_nodes as usize);

    for node_entry in nodes_table.pairs()
    {
        let (node_id, lua_node_table): (i64, LuaTable) = node_entry.unwrap();

        let lua_node_type: String = lua_node_table.get("type").unwrap();
        let node_name: String = lua_node_table.get("name").unwrap();

        let node_alloc: bool = lua_node_table.get("alloc").unwrap();

        let lua_node_ascend_name: Option<LuaString> = lua_node_table.get("ascendancyName").unwrap();

        let is_ascend_node =
            match lua_node_ascend_name {
                None => {false}
                Some(_) => {true}
            };

        let node = Node {
            id: node_id,
            name: node_name,
            path_dist: 0,
            planned_alloc: false,
            is_ascend: is_ascend_node,
            default_alloc: node_alloc
        };

        tree_nodes.push(node);
    }

    tree_nodes.sort_unstable_by(|a, b| b.id.cmp(&a.id));

    let nodes_table: LuaTable = spec_table.get("nodes").unwrap();

    let mut dna_encoder = DnaEncoderBuilder {
        tree_nodes_buffer: tree_nodes,
        masteries_buffer: Vec::new(),
        data_builder: |tree_nodes_buffer: &mut Vec<Node>, masteries_buffer: &mut Vec<Mastery>| {

            DnaEncoderDataBuilder {
                tree_nodes_wrapper_buffer: Vec::new(),
                data_builder: |tree_nodes_wrapper_buffer: &mut Vec<NodeWrapper>| {

                        let mut nodes_wrapper_buffer = Vec::with_capacity(tree_nodes_buffer.len());
                        let mut masteries = Vec::new();
                        let mut nodes_hash = HashMap::new();
                        let mut masteries_hash = HashMap::new();

                        for node in tree_nodes_buffer.iter_mut()
                        {
                            let node_table: LuaTable = nodes_table.get(node.id).unwrap();
                            let lua_node_type: String = node_table.get("type").unwrap();

                            if lua_node_type == "Mastery"
                            {
                                match masteries_hash.get_mut(&node.name)
                                {
                                    None => {
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

                                        masteries_buffer.push(Mastery {
                                            name: node.name.clone(),
                                            effects: mastery_effects,
                                            effects_indexes_to_select: Vec::with_capacity(10),
                                            effect_next_select_index: 0,
                                        });

                                        masteries_hash.insert(node.name.clone(), masteries_buffer.len());

                                    }
                                    Some(_) => {}
                                }
                            };

                            nodes_wrapper_buffer.push(NodeWrapper {
                                node,
                                node_type: NodeType::NORMAL,
                                linked_nodes: Vec::with_capacity(100),
                                path_nodes: Vec::with_capacity(100)
                            });
                        }

                        for mastery in masteries_buffer.iter_mut() {
                            masteries.push(mastery);
                        }

                        for node in nodes_wrapper_buffer.iter_mut()
                        {
                            let node_table: LuaTable = nodes_table.get(node.id).unwrap();
                            let lua_node_type: String = node_table.get("type").unwrap();

                            let node_type =
                                if lua_node_type == "Mastery"
                                {
                                    let mastery = &mut masteries_buffer[*masteries_hash.get_mut(&node.name).unwrap()];

                                    NodeType::MASTERY(MasteryVariant {
                                        mastery
                                    })
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

                            node.node_type = node_type;

                            nodes_hash.insert(node.id, node);
                        }

                        let nodes_table: LuaTable = spec_table.get("nodes").unwrap();
                        for node_entry in nodes_table.pairs()
                        {
                            let (_, lua_node_table): (i64, LuaTable) = node_entry.unwrap();

                            let table_linked: LuaTable = lua_node_table.get("linked").unwrap();

                            let node_id: i64 = lua_node_table.get("id").unwrap();

                            let node = nodes_hash.get(&node_id).unwrap();

                            for linked_node_entry in table_linked.pairs()
                            {
                                let (_, lua_linked_node_table): (i64, LuaTable) = linked_node_entry.unwrap();

                                let linked_node_id: i64 = lua_linked_node_table.get("id").unwrap();

                                let linked_node = nodes_hash.get(&linked_node_id).unwrap();

                                node.linked_nodes.push(*linked_node);
                            }
                        }

                        masteries.sort_unstable_by(|a, b|
                            {
                                b.name.cmp(&a.name)
                            });

                        let mut tree_nodes = Vec::new();

                        for node in tree_nodes_wrapper_buffer.iter_mut()
                        {
                            tree_nodes.push(node);
                        }

                        DnaEncoderData2 {
                            tree_nodes,
                            masteries,
                            path_nodes_buf: Vec::with_capacity(tree_nodes_buffer.len()),
                            nodes_to_allocate: HashMap::with_capacity(tree_nodes_buffer.len()),
                            queue_nodes_buffer: VecDeque::with_capacity(tree_nodes_buffer.len())
                        }
                }
            }.build()
        }
    }.build();

    let tree_nodes_len = tree_nodes.len().clone();

    Ok(dna_encoder)
}
