// use alloy::primitives::U256;
use axum::{routing::get, Router};
use ethers::types::U256;
use pipegate::{channel::ChannelState, middleware::auth_middleware};
use tower_service::Service;
use worker::*;

fn router() -> Router {
    let state = ChannelState::new("wss://base-sepolia-rpc.publicnode.com".parse().unwrap());

    Router::new()
        .route("/", get(root))
        .layer(axum::middleware::from_fn(move |req, next| {
            let state = state.clone();
            auth_middleware(state, U256::from(1000), req, next)
        }))
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    console_error_panic_hook::set_once();
    Ok(router().call(req).await?)
}

pub async fn root() -> &'static str {
    "Hello Axum!"
}
