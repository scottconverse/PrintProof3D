use axum::{
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use std::net::SocketAddr;

async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok());

    let target_token = std::env::var("PRINTPROOF3D_API_TOKEN")
        .unwrap_or_else(|_| "secret_print_token".to_string());

    let expected_auth = format!("Bearer {}", target_token);

    if let Some(auth) = auth_header {
        if auth == expected_auth {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn home() -> &'static str {
    "PrintProof3D REST API"
}

async fn protected_ping() -> &'static str {
    "pong"
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(home))
        .route(
            "/protected",
            get(protected_ping).route_layer(middleware::from_fn(auth_middleware)),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
