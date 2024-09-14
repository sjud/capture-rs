use crate::utils::log;
use crate::{types::*, SERIALIZED_NODE_MAP_REPLAY};
use crate::{window, NODE_MAP_REPLAY};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use web_sys::{DomException, HtmlIFrameElement, Node};

pub fn rebuild<S: AsRef<str>>(
    iframe_id: S,
    serialized_node_map: HashMap<u32, SerializedNode>,
) -> Result<(), String> {
    let iframe = window()
        .document()
        .expect("document")
        .get_element_by_id(iframe_id.as_ref())
        .expect("iframe_id to be an element on the page")
        .dyn_into::<HtmlIFrameElement>()
        .expect("iframe_id to be the id of an HtmlIFrameElement");
    let iframe_document = iframe.content_document().unwrap().unchecked_into::<Node>();
    while let Some(child) = iframe_document.last_child() {
        iframe_document
            .remove_child(&child)
            .map_err(|err| err.unchecked_into::<DomException>().message())?;
    }
    let serialized_root = serialized_node_map
        .get(&0)
        .expect("Node to be at 0 idx")
        .clone();
    SERIALIZED_NODE_MAP_REPLAY.with(|map| {
        map.borrow_mut().extend(serialized_node_map);
    });

    let (root, root_children) = serialized_root
        .build(&iframe_document, None, None)
        .map_err(|err| err.message())?;
    add_dom_tree(root, root_children, 0)?;
    Ok(())
}

pub fn add_dom_tree(root: Node, root_children: Vec<u32>, root_id: u32) -> Result<(), String> {
    // insert root
    NODE_MAP_REPLAY.with(|node_map| node_map.borrow_mut().insert(root_id, root.clone()));
    let mut stack: Vec<(Node, Vec<u32>)> = vec![(root, root_children)];
    // insert all children iteratively
    while let Some((node, mut children)) = stack.pop() {
        while let Some(child) = children.pop() {
            let serialized_child = SERIALIZED_NODE_MAP_REPLAY.with(|node_map| {
                node_map
                    .borrow()
                    .get(&child)
                    .expect(&format!("not fouond {child} in {:#?}", node_map.borrow()))
                    .clone()
            });

            let (node, node_children) = serialized_child
                .build(&node, None, None)
                .map_err(|err| err.message())?;
            NODE_MAP_REPLAY.with(|node_map| node_map.borrow_mut().insert(child, node.clone()));
            stack.push((node, node_children));
        }
    }
    Ok(())
}
