use crate::domain::entities::{Order, RiceProduct};
use crate::domain::events::DomainEvent;
use crate::domain::value_objects::{DomainError, OrderId};

/// Outbound port for rice catalog persistence.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait ProductRepository: Send + Sync {
    async fn list_all(&self) -> Vec<RiceProduct>;
    async fn find_by_id(&self, id: &str) -> Option<RiceProduct>;
    async fn save(&self, product: RiceProduct) -> Result<(), DomainError>;
}

/// Outbound port for order persistence.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait OrderRepository: Send + Sync {
    async fn list_all(&self) -> Vec<Order>;
    async fn find_by_id(&self, id: &OrderId) -> Option<Order>;
    async fn save(&self, order: Order) -> Result<(), DomainError>;
}

/// Outbound port for publishing domain events.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: DomainEvent);
}
