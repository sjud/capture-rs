pub mod event_stream;
use std::{cell::RefCell, collections::HashMap};

pub use event_stream::*;
pub mod user_events;
pub use user_events::*;
pub mod snapshot;
pub mod types;
pub mod utils;
pub use types::*;
pub mod mutation_stream;
pub mod observer;
pub mod rebuild;
pub use mutation_stream::*;
pub mod replay;
pub use replay::*;

use web_sys::{Node, Window};

pub fn window() -> Window {
    WINDOW.with(Clone::clone)
}

fn timestamp() -> f64 {
    window().performance().expect("performance").now()
}

thread_local! {
    pub static NODE_MAP: RefCell<HashMap<u32, Node>> = RefCell::new(HashMap::new());
    pub static NODE_MAP_REPLAY : RefCell<HashMap<u32,Node>> = RefCell::new(HashMap::new());
    pub static REVERSE_NODE_MAP : js_sys::Map = js_sys::Map::new();
    pub static SERIALIZED_NODE_MAP: RefCell<HashMap<u32, SerializedNode>> = RefCell::new(HashMap::new());
    pub static SERIALIZED_NODE_MAP_REPLAY: RefCell<HashMap<u32, SerializedNode>> = RefCell::new(HashMap::new());
    pub static NODE_ID : RefCell<u32> = RefCell::new(0);
    pub static ROOTS : RefCell<Vec<(Node,u32)>> = RefCell::new(Vec::new());
    pub static WINDOW: web_sys::Window = web_sys::window().expect("valid window");
    pub static SNAPSHOT_TIME : f64 = timestamp();
    pub static TIME_OF_LAST_MUTATION : RefCell<f64> = RefCell::new(0.);
}
