use control_plane::{build_router, AppState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind = std::env::var("CONTROL_PLANE_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let addr: SocketAddr = bind.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("control-plane listening on http://{addr}");
    axum::serve(listener, build_router(AppState::from_env().await?)).await?;
    Ok(())
}
