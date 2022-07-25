use axum::{
    extract::Path,
    http::{Response, StatusCode},
    response::{
        sse::{self, KeepAlive},
        Html, IntoResponse, Sse,
    },
    routing::{get, post},
    Extension, Json, Router,
};
use captracker::Subscriptions;
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tokio_stream::StreamExt as _;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let subscriptions = Arc::new(RwLock::new(Subscriptions::new()));

    #[cfg(debug_assertions)]
    let race_page = {
        axum::routing::get_service(tower_http::services::ServeFile::new(std::path::Path::new(
            "src/race.html",
        )))
        .handle_error(handle_error)
    };

    #[cfg(not(debug_assertions))]
    let race_page = { get(|| async { Html(include_str!("race.html")) }) };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/:race", race_page)
        .route("/sse/:race", get(stream))
        // `POST /users` goes to `create_user`
        .layer(Extension(subscriptions));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// async fn race() -> Result<impl IntoResponse, anyhow::Error> {
//     let file = tokio::fs::File::open(std::path::Path::new("src/race.html")).await?;

//     let response = Response::builder()
//         .status(StatusCode::OK)
//         .header(
//             reqwest::header::CONTENT_TYPE,
//             axum::http::HeaderValue::from_str("text/html").unwrap(),
//         )
//         .body(axum::body::boxed(axum::body::StreamBody::new(file.into())))
//         .unwrap();
//     Ok(response)
// }

async fn stream(
    Path(race): Path<String>,
    Extension(subs): Extension<Arc<RwLock<Subscriptions>>>,
) -> sse::Sse<impl futures::stream::Stream<Item = Result<sse::Event, Infallible>>> {
    let rx = {
        let mut lock = subs.write().await;
        lock.get(&race)
    };

    let stream = tokio_stream::wrappers::WatchStream::new(rx)
        .filter(|value| value != &serde_json::Value::Null)
        // .map(|entry| entry.unwrap())
        .map(|update| {
            sse::Event::default()
                .event("race_update")
                .json_data(&update)
                .unwrap()
        })
        .map(Ok);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn handle_error(_err: std::io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
