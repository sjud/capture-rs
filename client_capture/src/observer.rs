use std::{borrow::Borrow, collections::HashMap};

use js_sys::{Array, Function};
use tokio::sync::mpsc::UnboundedSender;
use wasm_bindgen::prelude::*;
use web_sys::{MutationObserver, MutationObserverInit, MutationRecord, Node};

use crate::{
    MutationVariant, SerializedNode, NODE_ID, NODE_MAP, REVERSE_NODE_MAP, SERIALIZED_NODE_MAP,
};

pub fn observe(sender: UnboundedSender<MutationVariant>, target: &Node) {
    let closure = Closure::wrap(
        Box::new(move |mutation_records: Array, _: MutationObserver| {
            let records = mutation_records
                .iter()
                .map(|item| item.unchecked_into::<MutationRecord>())
                .collect::<Vec<_>>();
            for record in records {
                web_sys::console::log_1(record.as_ref());
                sender
                    .send(MutationVariant::new(record))
                    .expect("mutation send to always succeed");
            }
        }) as Box<dyn FnMut(_, _)>,
    );
    let f = closure.into_js_value().unchecked_into::<Function>();
    let mutation_observer = MutationObserver::new(f.as_ref()).expect("Mutation obvserver");
    mutation_observer
        .observe_with_options(target, &{
            let init = MutationObserverInit::new();
            init.set_animations(true);
            init.set_attribute_old_value(true);
            init.set_attributes(true);
            init.set_character_data(true);
            init.set_subtree(true);
            init.set_character_data_old_value(true);
            init.set_child_list(true);
            init
        })
        .expect("observe");
}

pub(crate) fn mutation_parse_added_nodes(
    added_nodes: Vec<Node>,
    target: Node,
) -> HashMap<u32, SerializedNode> {
    let id = || NODE_ID.with(|id| *id.borrow());
    let target_id = REVERSE_NODE_MAP.with(|map| {
        map.get(target.as_ref())
            .as_f64()
            .expect("reverse map to have id of target node") as u32
    });
    // create a stack of nodes to parse from MutationRecord's added nodes,
    // where each node has as its parent id the associated id (from reverse node map) given the target node in mutation record
    let mut stack = added_nodes
        .into_iter()
        .map(|node| (node, target_id))
        .collect::<Vec<_>>();
    let mut serialized_nodes: HashMap<u32, SerializedNode> = HashMap::new();
    while let Some((current_node, parent_id)) = stack.pop() {
        // add child id to the serialized target
        SERIALIZED_NODE_MAP.with(|serialized_node_map| {
            serialized_node_map
                .borrow_mut()
                .get_mut(&parent_id)
                .unwrap()
                .push_child(id());
        });
        for node in serialized_nodes.values_mut() {
            if node.id() == parent_id {
                node.push_child(id());
            }
        }

        NODE_MAP.with(|node_map| node_map.borrow_mut().insert(id(), current_node.clone()));
        REVERSE_NODE_MAP.with(|reverse_node_map| {
            reverse_node_map.set(current_node.as_ref(), &JsValue::from_f64(id() as f64))
        });
        let serialized_node = SerializedNode::new(&current_node, id());
        serialized_nodes.insert(id(), serialized_node.clone());
        SERIALIZED_NODE_MAP.with(|node_map| node_map.borrow_mut().insert(id(), serialized_node));
        // Push the child nodes onto the stack in reverse order so that
        // the first child is processed first.
        let child_nodes = current_node.child_nodes();
        let length = child_nodes.length();

        for i in (0..length).rev() {
            if let Some(child) = child_nodes.item(i) {
                stack.push((child, id()));
            }
        }
        NODE_ID.with(|id| *id.borrow_mut() += 1);
    }
    serialized_nodes
}
