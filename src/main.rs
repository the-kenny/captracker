use axum::http::StatusCode;
use axum::{
    extract::Path,
    response::{
        sse::{self, KeepAlive},
        IntoResponse, Sse,
    },
    routing::get,
    Extension, Router,
};
use captracker::Subscriptions;
use futures::stream::StreamExt as _;
use std::{convert::Infallible, future, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
// use tokio_stream::StreamExt as _;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let subscriptions = Arc::new(RwLock::new(Subscriptions::new()));

    #[cfg(not(debug_assertions))]
    use axum::response::Html;
    #[cfg(not(debug_assertions))]
    let race_page = { get(|| async { Html(include_str!("race.html")) }) };
    #[cfg(not(debug_assertions))]
    let location_page = { get(|| async { Html(include_str!("location.html")) }) };

    #[cfg(debug_assertions)]
    let race_page = {
        axum::routing::get_service(tower_http::services::ServeFile::new(std::path::Path::new(
            "src/race.html",
        )))
        .handle_error(handle_error)
    };

    #[cfg(debug_assertions)]
    let location_page = {
        axum::routing::get_service(tower_http::services::ServeFile::new(std::path::Path::new(
            "src/location.html",
        )))
        .handle_error(handle_error)
    };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/:race", race_page)
        .route("/:race/:cap", location_page)
        .route("/sse/:race", get(race_stream))
        .route("/sse/:race/:cap/location", get(location_stream))
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

async fn race_stream(
    Path(race): Path<String>,
    Extension(subs): Extension<Arc<RwLock<Subscriptions>>>,
) -> sse::Sse<impl futures::stream::Stream<Item = Result<sse::Event, Infallible>>> {
    let rx = {
        let mut lock = subs.write().await;
        lock.race(&race)
    };

    let stream = tokio_stream::wrappers::WatchStream::new(rx)
        .filter(|value| future::ready(value != &serde_json::Value::Null))
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

async fn location_stream(
    Path((race, cap)): Path<(String, u64)>,
    Extension(subs): Extension<Arc<RwLock<Subscriptions>>>,
) -> sse::Sse<impl futures::stream::Stream<Item = Result<sse::Event, Infallible>>> {
    let rx = {
        let mut lock = subs.write().await;
        lock.cap(&race, cap)
    };

    let stream = tokio_stream::wrappers::WatchStream::new(rx)
        .filter_map(|value| future::ready(value.into_inner()))
        // .map(|entry| entry.unwrap())
        .map(|update| {
            sse::Event::default()
                .event("location_update")
                .json_data(update)
                .unwrap()
        })
        .map(Ok);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn handle_error(_err: std::io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
