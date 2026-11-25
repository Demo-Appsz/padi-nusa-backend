use padinusa_backend::application::use_cases::StoreApplicationService;
use padinusa_backend::infrastructure::http_axum::build_router;
use padinusa_backend::infrastructure::persistence::{
    InMemoryEventPublisher, InMemoryOrderRepository, InMemoryProductRepository,
};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let products = Arc::new(InMemoryProductRepository::seeded());
    let orders = Arc::new(InMemoryOrderRepository::new());
    let events = Arc::new(InMemoryEventPublisher::new());
    let service = Arc::new(StoreApplicationService::new(products, orders, events));

    let app = build_router(service);
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    println!("PadiNusa API server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");
    axum::serve(listener, app)
        .await
        .expect("Axum server exited with error");
}
