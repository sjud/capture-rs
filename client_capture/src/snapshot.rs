use crate::{types::*, NODE_ID, NODE_MAP, REVERSE_NODE_MAP, ROOTS, SERIALIZED_NODE_MAP};
use crate::{window, SNAPSHOT_TIME};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::Node;

//. if no node is given will snapshot the document
pub async fn snapshot<S: AsRef<str>>(
    node: Option<Node>,
    digest_endpoint: S,
) -> Result<(), JsValue> {
    let node = node.unwrap_or_else(|| {
        window()
            .document()
            .expect("A document on the window")
            .unchecked_into::<Node>()
    });
    snapshot_parse_dom(&node, 0);
    //initialize snapshot time...
    _ = SNAPSHOT_TIME.with(|time| *time);
    let body = bincode::serialize(&SERIALIZED_NODE_MAP.with(|node_map| node_map.borrow().clone()))
        .unwrap();

    gloo_net::http::Request::post(digest_endpoint.as_ref())
        .body(body)
        .expect("body ody ody")
        .send()
        .await
        .expect("Server to receive request.");
    Ok(())
}

// Returns a list of serialized nodes that were created after parsing DOM tree beginning at node.
fn snapshot_parse_dom(initial_node: &Node, parent_id: u32) {
    let id = || NODE_ID.with(|id| *id.borrow());
    let mut stack = vec![(initial_node.clone(), parent_id)];
    while let Some((current_node, parent_id)) = stack.pop() {
        // if not root, add child id to parent
        if id() != 0 {
            SERIALIZED_NODE_MAP.with(|serialized_node_map| {
                serialized_node_map
                    .borrow_mut()
                    .get_mut(&parent_id)
                    .unwrap()
                    .push_child(id());
            });
        }

        NODE_MAP.with(|node_map| node_map.borrow_mut().insert(id(), current_node.clone()));
        REVERSE_NODE_MAP.with(|reverse_node_map| {
            reverse_node_map.set(current_node.as_ref(), &JsValue::from_f64(id() as f64))
        });
        let serialized_node = SerializedNode::new(&current_node, id());
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
}
/// Don't call this from replay code.
pub fn map_node_to_id(node: &Node) -> Option<u32> {
    REVERSE_NODE_MAP
        .with(|reverse_node_map| reverse_node_map.get(node.as_ref()).as_f64())
        .map(|id| id as u32)
}

/// Will only return None if the root is not in the root map.
/// This is O(N) but it will only iterate over roots.
pub fn find_root_id(node: &Node) -> Option<u32> {
    ROOTS.with(|root_map| {
        let root = node.get_root_node();
        for (node, node_id) in root_map.borrow().iter() {
            if node.loose_eq(root.as_ref()) {
                return Some(*node_id);
            }
        }
        None
    })
}
