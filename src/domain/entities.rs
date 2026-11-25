use crate::domain::value_objects::{
    DomainError, MoneyIdr, OrderId, OrderStatus, RiceGrade, RiceTextureProfile, WeightKg,
};
use serde::{Deserialize, Serialize};

/// Rice catalog aggregate root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiceProduct {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub origin_region: String,
    pub harvest_batch: String,
    pub milled_date: String,
    pub grade: RiceGrade,
    pub base_price_per_kg: MoneyIdr,
    pub stock_kg: u32,
    pub available_packs: Vec<WeightKg>,
    pub texture: RiceTextureProfile,
    pub tasting_notes: Vec<String>,
    pub image_url: String,
    pub badge: Option<String>,
}

impl RiceProduct {
    pub fn supports_pack(&self, pack: WeightKg) -> bool {
        self.available_packs.contains(&pack)
    }

    pub fn reserve_stock(&mut self, requested_kg: u32) -> Result<(), DomainError> {
        if self.stock_kg < requested_kg {
            return Err(DomainError::InsufficientStock {
                product: self.name.clone(),
                available_kg: self.stock_kg,
                requested_kg,
            });
        }
        self.stock_kg -= requested_kg;
        Ok(())
    }

    pub fn restock(&mut self, added_kg: u32) {
        self.stock_kg = self.stock_kg.saturating_add(added_kg);
    }
}

/// Line item inside a placed order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub product_name: String,
    pub pack_kg: WeightKg,
    pub quantity: u16,
    pub unit_price: MoneyIdr,
    pub line_total: MoneyIdr,
}

/// Customer order aggregate root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub customer_name: String,
    pub customer_phone: String,
    pub delivery_address: String,
    pub city: String,
    pub is_subscription: bool,
    pub items: Vec<OrderItem>,
    pub subtotal: MoneyIdr,
    pub discount: MoneyIdr,
    pub shipping_fee: MoneyIdr,
    pub total: MoneyIdr,
    pub promo_code: Option<String>,
    pub status: OrderStatus,
    pub created_at: String,
}

impl Order {
    pub fn advance_status(&mut self, next: OrderStatus) -> Result<(), DomainError> {
        self.status = self.status.transition_to(next)?;
        Ok(())
    }
}
