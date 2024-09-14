use crate::window;
use js_sys::{Array, Date, Function};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{Element, MutationObserver, MutationObserverInit, MutationRecord, Node};

/// Throttle function to limit the frequency of event handling
pub fn throttle<F, E>(callback: F, delay: u64) -> impl Fn(E)
where
    F: Fn(E) + 'static,
{
    let last_call_time = Rc::new(RefCell::new(0.0));

    move |event: E| {
        let now = Date::now();
        let mut last_call = last_call_time.borrow_mut();

        if now - *last_call > delay as f64 {
            *last_call = now;
            callback(event);
        }
    }
}

pub fn outer_html() -> String {
    window()
        .document()
        .expect("Document to exist")
        .document_element()
        .expect("Document element")
        .outer_html()
}

pub fn dom_changes() {
    let closure = Closure::wrap(Box::new(
        move |mutation_records: Array, _observer: MutationObserver| {
            let records = mutation_records
                .iter()
                .map(|item| item.unchecked_into::<MutationRecord>())
                .collect::<Vec<_>>();
            for record in records {
                let added_nodes = record.added_nodes();
                for node in added_nodes.entries() {
                    let n = node.unwrap();
                    let n = n.dyn_into::<Node>();
                    if let Ok(n) = n {
                        log_node_info(&n).unwrap();
                    }
                }
                let attribute_name = record.attribute_name();
                log(&attribute_name.unwrap_or_default());
                let attribute_namespace = record.attribute_namespace();
                log(&attribute_namespace.unwrap_or_default());
                if let Some(next_sibling) = record.next_sibling() {
                    log_node_info(&next_sibling).unwrap();
                }

                let old_value = record.old_value();
                log(&old_value.unwrap_or_default());
                if let Some(previous_sibling) = record.previous_sibling() {
                    log_node_info(&previous_sibling).unwrap();
                }
                let removed_nodes = record.removed_nodes();
                for node in removed_nodes.entries() {
                    let n = node.unwrap();
                    if let Ok(n) = n.dyn_into::<Node>() {
                        log_node_info(&n).unwrap();
                    }
                }
                if let Some(target) = record.target() {
                    log_node_info(&target).unwrap();
                }
            }
        },
    ) as Box<dyn FnMut(_, _)>);
    let f = closure.into_js_value().unchecked_into::<Function>();
    let mutation_observer = MutationObserver::new(f.as_ref()).expect("Mutation obvserver");
    mutation_observer
        .observe_with_options(window().document().unwrap().body().unwrap().as_ref(), &{
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

fn log_node_info(node: &Node) -> Result<(), JsValue> {
    // Node type (Element, Text, etc.)
    let node_type = node.node_type();
    if node_type == 1 {
        let el = node.dyn_ref::<Element>().unwrap();
        let attribute_names = el
            .get_attribute_names()
            .into_iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<_>>();
        for attribute in attribute_names {
            let attribute_value = el.get_attribute(&attribute).unwrap();
            log(&format!("{attribute}:{attribute_value}"));
        }
    }

    // Node name (e.g., DIV, P, etc.)
    log(&format!("Node Name: {:?}", node.node_name()));

    // Text content (only available if the node has text content)
    log(&format!("Text Content: {:?}", node.text_content()));

    // Child nodes count
    log(&format!(
        "Child Nodes Count: {:?}",
        node.child_nodes().length()
    ));

    // Parent node (if any)
    if let Some(parent) = node.parent_node() {
        log(&format!("Parent Node Name: {:?}", parent.node_name()));
    }

    Ok(())
}

pub fn log<S: AsRef<str>>(s: S) {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(s.as_ref()));
}
