use gloo_timers::future::TimeoutFuture;
use js_sys::Function;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::Element;

use crate::{window, MutationVariant, NODE_MAP_REPLAY, SERIALIZED_NODE_MAP_REPLAY};
pub async fn replay(mutations: Vec<MutationVariant>) {
    let mut mutations = mutations;
    // make sure mutations are sorted in chronological order, we're are assuring that millis is unique in our mutation new code by adding a fractional increment
    // to every mutation when they share the exact timestamp.
    mutations.sort_by(|a, b| {
        a.millis()
            .partial_cmp(&b.millis())
            .expect("millis should always be real numbers")
    });
    let mut last_millis = 0.;
    for mutation in mutations {
        let timeout = (mutation.millis() - last_millis).floor();
        // this might be 0 but thats okay
        TimeoutFuture::new(timeout as u32).await;
        last_millis = mutation.millis();
        let closure = Closure::new(Box::new(move || mutation.replay()) as Box<dyn FnMut()>);
        // request animation frame will always run in the sequence it was called
        window()
            .request_animation_frame(&closure.into_js_value().dyn_into::<Function>().unwrap())
            .expect("set animation frame");
    }
}
impl MutationVariant {
    pub fn replay(&self) {
        let target_id = self.target_id();
        match self {
            MutationVariant::ChildListAdded((mutation, added_map)) => {
                SERIALIZED_NODE_MAP_REPLAY.with(|map| map.borrow_mut().extend(added_map.clone()));
                for id in mutation.nodes.clone() {
                    let serialized_node = added_map
                        .get(&id)
                        .expect("id from mutation.nodes to be in added_map");
                    SERIALIZED_NODE_MAP_REPLAY.with(|serialized_node_map| {
                        serialized_node_map
                            .borrow_mut()
                            .insert(id, serialized_node.clone())
                    });

                    let parent = NODE_MAP_REPLAY
                        .with(|node_map| node_map.borrow().get(&target_id).cloned())
                        .expect(&format!(
                            "Didnt find {} in \n {:#?}",
                            target_id,
                            NODE_MAP_REPLAY.with(|s| s.borrow().clone())
                        ));
                    let prev_sibling = mutation.prev_sibling.map(|id| {
                        NODE_MAP_REPLAY
                            .with(|node_map| node_map.borrow().get(&id).cloned())
                            .expect("prev sibling id to be in node map")
                    });
                    let next_sibling = mutation.prev_sibling.map(|id| {
                        NODE_MAP_REPLAY
                            .with(|node_map| node_map.borrow().get(&id).cloned())
                            .expect("next sibling id to be in node map")
                    });
                    let (node, children) = serialized_node
                        .build(&parent, prev_sibling, next_sibling)
                        .expect("to build or not to build");
                    crate::rebuild::add_dom_tree(node, children, id).unwrap();
                }
            }
            MutationVariant::ChildListRemoved(mutation) => {
                for id in mutation.nodes.iter() {
                    let parent = NODE_MAP_REPLAY
                        .with(|node_map| node_map.borrow().get(&target_id).cloned())
                        .expect("Parent to resolve to a node");
                    let this = NODE_MAP_REPLAY
                        .with(|node_map| node_map.borrow_mut().remove(id))
                        .expect(&format!("removed node id:{id} to exist in node map"));
                    parent
                        .remove_child(&this)
                        .expect("remove child to be valid");
                    SERIALIZED_NODE_MAP_REPLAY
                        .with(|node_map| node_map.borrow_mut().remove(id))
                        .expect("node to exist in serialized node map too.");
                }
            }
            MutationVariant::CharacterData(mutation) => {
                let id = target_id;
                NODE_MAP_REPLAY.with(|node_map| {
                    node_map
                        .borrow()
                        .get(&id)
                        .expect("valid node")
                        .set_text_content(mutation.text_content.as_ref().map(|s| s.as_str()))
                });
                SERIALIZED_NODE_MAP_REPLAY.with(|node_map| {
                    node_map
                        .borrow_mut()
                        .get_mut(&id)
                        .expect("valid serialized node")
                        .set_text_content(mutation.text_content.clone())
                })
            }
            MutationVariant::Attributes(mutation) => {
                let (name, value) = mutation
                    .attribute
                    .clone()
                    .expect("Attribute variant to have attribute value");
                let id = target_id;
                NODE_MAP_REPLAY.with(|node_map| {
                    node_map
                        .borrow()
                        .get(&id)
                        .expect("valid node")
                        .unchecked_ref::<Element>()
                        .set_attribute(&name, &value)
                        .expect("set attribute to work.")
                });
                SERIALIZED_NODE_MAP_REPLAY.with(|node_map| {
                    node_map
                        .borrow_mut()
                        .get_mut(&id)
                        .expect("valid serialized node")
                        .set_attribute(name, value)
                });
            }
        }
    }
}
