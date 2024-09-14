use crate::window;
use js_sys::Function;
use tokio::sync::mpsc::UnboundedSender;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Event;
use web_sys::MouseEvent;

use crate::utils::throttle;
use crate::CaptureEvent;

pub fn capture_mouse(sender: UnboundedSender<CaptureEvent>) -> Result<(), JsValue> {
    let sender_c = sender.clone();
    let closure = Closure::wrap(Box::new(throttle(
        move |event: MouseEvent| {
            let x = event.client_x();
            let y = event.client_y();
            sender_c
                .send(CaptureEvent::MouseMove { x, y })
                .expect("send to always succeed");
        },
        50, // Throttle delay of 200 milliseconds
    )) as Box<dyn FnMut(_)>);

    window().add_event_listener_with_callback(
        "mousemove",
        &closure.into_js_value().dyn_into::<Function>()?,
    )?;

    let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.client_x();
        let y = event.client_y();
        sender
            .send(CaptureEvent::MouseClick { x, y })
            .expect("send to always succeed");
    }) as Box<dyn FnMut(_)>);

    window().add_event_listener_with_callback(
        "mouseclick",
        &closure.into_js_value().dyn_into::<Function>()?,
    )?;
    Ok(())
}

pub fn capture_window(sender: UnboundedSender<CaptureEvent>) -> Result<(), JsValue> {
    // compress these to first and last in the event stream.
    let closure = Closure::wrap(Box::new(move |_: Event| {
        if let (Some(width), Some(height)) = (
            window()
                .inner_width()
                .ok()
                .and_then(|w| w.as_f64())
                .map(|w| w as u32),
            window()
                .inner_height()
                .ok()
                .and_then(|h| h.as_f64())
                .map(|h| h as u32),
        ) {
            sender
                .send(CaptureEvent::WindowResize { height, width })
                .expect("send to always succeed");
        }
    }) as Box<dyn FnMut(_)>);

    window().add_event_listener_with_callback(
        "resize",
        &closure.into_js_value().dyn_into::<Function>()?,
    )?;
    Ok(())
}
