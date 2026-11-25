use crate::domain::value_objects::{MoneyIdr, OrderId, OrderStatus};
use serde::{Deserialize, Serialize};

/// Domain events emitted by aggregates and use cases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    OrderPlaced {
        order_id: OrderId,
        total_idr: MoneyIdr,
        item_count: usize,
    },
    StockReserved {
        product_id: String,
        reserved_kg: u32,
        remaining_kg: u32,
    },
    OrderStatusChanged {
        order_id: OrderId,
        new_status: OrderStatus,
    },
}
