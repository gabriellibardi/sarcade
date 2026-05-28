use axum::{
    body::Body,
    http::{HeaderValue, Response, StatusCode, header},
    response::IntoResponse,
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../frontend/build"]
struct Assets;

pub async fn serve(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Tenta servir o arquivo exato
    if let Some(file) = Assets::get(path) {
        return build_response(path, file);
    }

    if let Some(file) = Assets::get("index.html") {
        return build_response("index.html", file);
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
}

fn build_response(path: &str, file: rust_embed::EmbeddedFile) -> Response<Body> {
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap())
        .body(Body::from(file.data))
        .unwrap()
}
