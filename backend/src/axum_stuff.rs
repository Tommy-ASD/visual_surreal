use std::{fs::File, io::Read};

use axum::{
    body::{boxed, Body, BoxBody},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{Request, Response, StatusCode, Uri},
    response::{self, IntoResponse},
    routing::get,
    Router,
};
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub fn config(router: Router) -> Router {
    router
        .route("/", get(root))
        .route("/ws", get(handler))
        .route("/static", get(req_handler))
}

pub async fn handler(ws: WebSocketUpgrade) -> response::Response {
    println!("Websocket connection established!");
    ws.on_upgrade(handle_socket)
}

pub async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            match msg.clone() {
                Message::Text(msg) => {
                    println!("Received: {msg}");
                }
                Message::Binary(bin) => {
                    match String::from_utf8(bin) {
                        Ok(msg) => {
                            println!("Received binary: {msg}");
                        }
                        Err(_) => {
                            println!("Received invalid UTF-8");
                        }
                    }
                    continue;
                }
                Message::Ping(_) | Message::Pong(_) => {
                    // ignore ping/pong
                    continue;
                }
                Message::Close(_) => {
                    // client is closing the connection
                    return;
                }
            }
            msg
        } else {
            // client disconnected
            return;
        };

        if socket.send(msg).await.is_err() {
            // client disconnected
            return;
        }
    }
}

// basic handler that responds with a static string
async fn root() -> Result<Response<BoxBody>, (StatusCode, String)> {
    match format!("/static/index.html").parse() {
        Ok(uri) => get_static_file(uri).await,
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid URI".to_string())),
    }
}

async fn req_handler(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
    let res = get_static_file(uri.clone()).await?;

    if res.status() == StatusCode::NOT_FOUND {
        // try with `.html`
        // TODO: handle if the Uri has query parameters
        match format!("{uri}").parse() {
            Ok(uri_html) => get_static_file(uri_html).await,
            Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid URI".to_string())),
        }
    } else {
        Ok(res)
    }
}

async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    match ServeDir::new(".").oneshot(req).await {
        Ok(res) => Ok(res.map(boxed)),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", err),
        )),
    }
}
