use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, Debug)]
pub enum CaptureEvent {
    /// X Y position of a mousemove event.
    MouseMove {
        x: i32,
        y: i32,
    },
    /// X Y position where a mouseclick event occurred.
    MouseClick {
        x: i32,
        y: i32,
    },
    /// The height and width of window.inner_ respectively.
    WindowResize {
        height: u32,
        width: u32,
    },
    TouchMove {
        x: i32,
        y: i32,
    },
    Scoll {},
}

pub struct EventStream {
    sender: UnboundedSender<CaptureEvent>,
}

impl EventStream {
    pub fn new() -> Self {
        let (sender, mut receiver) = unbounded_channel();
        spawn_local(async move {
            let mut chunk = Vec::new();
            while let Some(event) = receiver.recv().await {
                chunk.push(event);
                if chunk.len() > 5 {
                    // web_sys::console::log_1(&format!("{chunk:#?}").into());
                    chunk = Vec::new();
                }
            }
        });
        EventStream { sender }
    }
    pub fn send(&self, event: CaptureEvent) {
        self.sender.send(event).expect("Send to always be ok.")
    }
}
