use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use wasm_bindgen_futures::spawn_local;

use crate::{timestamp, utils::log, MutationVariant};

pub struct MutationStream<S: AsRef<str>> {
    pub sender: UnboundedSender<MutationVariant>,
    receiver: UnboundedReceiver<MutationVariant>,
    mutation_endpoint: S,
    interval_millis: f64,
    chunk: Vec<MutationVariant>,
}

impl<S> MutationStream<S>
where
    S: AsRef<str>,
{
    pub fn new(mutation_endpoint: S, interval_millis: f64) -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            sender,
            receiver,
            mutation_endpoint,
            interval_millis,
            chunk: Vec::new(),
        }
    }
    /// Will aggregate Mutations and then post them to the digest endpoint at the given interval.
    pub async fn receive_and_post(&mut self) {
        let mut ts = timestamp();
        while let Some(mutation) = self.receiver.recv().await {
            self.chunk.push(mutation);
            let now = timestamp();
            // if current timestamp is 500 millis greater than last timestamp
            if now > ts + self.interval_millis {
                let chunk = bincode::serialize(&self.chunk).unwrap();
                let endpoint = self.mutation_endpoint.as_ref().to_string();
                spawn_local(async move {
                    gloo_net::http::Request::post(&endpoint)
                        .body(chunk)
                        .expect("body ody ody")
                        .send()
                        .await
                        .expect("Server to receive request.");
                });
                self.chunk = Vec::new();
                ts = now;
            }
        }
    }
}
