#[cfg(feature = "ssr")]
pub mod server {
    pub use axum::body::Bytes;
    pub use axum::routing::post;
    pub use axum::{Extension, Json, Router};
    pub use client_capture::{MutationVariant, SerializedNode};
    pub use http::StatusCode;
    pub use leptos::prelude::*;
    pub use leptos_axum::{generate_route_list, LeptosRoutes};
    pub use replay_server::app::*;
    pub use std::collections::HashMap;
    pub use std::sync::{Arc, RwLock};

    pub async fn ingest_snapshot(
        Extension(state): Extension<Arc<RwLock<HashMap<u32, SerializedNode>>>>,
        body: Bytes,
    ) -> Result<(), StatusCode> {
        let body = bincode::deserialize(&body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        *state
            .write()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? = body;
        Ok(())
    }

    pub async fn ingest_mutation(
        Extension(state): Extension<Arc<RwLock<Vec<MutationVariant>>>>,
        body: Bytes,
    ) -> Result<(), StatusCode> {
        let body = bincode::deserialize::<Vec<MutationVariant>>(&body)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        state
            .write()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .extend(body);
        Ok(())
    }
}
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use server::*;
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/ingest_snapshot", post(ingest_snapshot))
        .route("/api/ingest_mutation", post(ingest_mutation))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .layer(Extension(Arc::new(RwLock::new(HashMap::<
            u32,
            SerializedNode,
        >::new()))))
        .layer(Extension(Arc::new(RwLock::new(
            Vec::<MutationVariant>::new(),
        ))));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
