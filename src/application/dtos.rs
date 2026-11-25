use crate::domain::entities::{Order, OrderItem, RiceProduct};
use crate::domain::services::{CookingGuideResult, PriceQuote};
use crate::domain::value_objects::{OrderStatus, RiceGrade};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CartLineInput {
    pub product_id: String,
    pub pack_kg: u16,
    pub quantity: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuoteCartCommand {
    pub items: Vec<CartLineInput>,
    pub city: String,
    pub is_subscription: bool,
    pub promo_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaceOrderCommand {
    pub customer_name: String,
    pub customer_phone: String,
    pub delivery_address: String,
    pub city: String,
    pub is_subscription: bool,
    pub promo_code: Option<String>,
    pub items: Vec<CartLineInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOrderStatusCommand {
    pub status: OrderStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RestockProductCommand {
    pub added_kg: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogFilter {
    pub grade: Option<RiceGrade>,
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductPackOptionDto {
    pub weight_kg: u16,
    pub unit_price_idr: u64,
    pub discount_percent: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub origin_region: String,
    pub harvest_batch: String,
    pub milled_date: String,
    pub grade: RiceGrade,
    pub base_price_per_kg_idr: u64,
    pub stock_kg: u32,
    pub pack_options: Vec<ProductPackOptionDto>,
    pub pulen_score: u8,
    pub aroma_score: u8,
    pub glycemic_label: String,
    pub water_ratio: f32,
    pub soak_minutes: u16,
    pub cook_minutes: u16,
    pub tasting_notes: Vec<String>,
    pub image_url: String,
    pub badge: Option<String>,
}

impl From<&RiceProduct> for ProductDto {
    fn from(p: &RiceProduct) -> Self {
        let pack_options = p
            .available_packs
            .iter()
            .filter_map(|pack| {
                crate::domain::services::PricingService::unit_pack_price(p, *pack)
                    .ok()
                    .map(|unit_price| ProductPackOptionDto {
                        weight_kg: pack.kg(),
                        unit_price_idr: unit_price.amount(),
                        discount_percent: pack.bulk_discount_bps() / 100,
                    })
            })
            .collect();

        Self {
            id: p.id.clone(),
            slug: p.slug.clone(),
            name: p.name.clone(),
            tagline: p.tagline.clone(),
            description: p.description.clone(),
            origin_region: p.origin_region.clone(),
            harvest_batch: p.harvest_batch.clone(),
            milled_date: p.milled_date.clone(),
            grade: p.grade,
            base_price_per_kg_idr: p.base_price_per_kg.amount(),
            stock_kg: p.stock_kg,
            pack_options,
            pulen_score: p.texture.pulen_score,
            aroma_score: p.texture.aroma_score,
            glycemic_label: p.texture.glycemic_label.clone(),
            water_ratio: p.texture.water_ratio,
            soak_minutes: p.texture.soak_minutes,
            cook_minutes: p.texture.cook_minutes,
            tasting_notes: p.tasting_notes.clone(),
            image_url: p.image_url.clone(),
            badge: p.badge.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteItemDto {
    pub product_id: String,
    pub product_name: String,
    pub pack_kg: u16,
    pub quantity: u16,
    pub unit_price_idr: u64,
    pub line_total_idr: u64,
}

impl From<&OrderItem> for QuoteItemDto {
    fn from(item: &OrderItem) -> Self {
        Self {
            product_id: item.product_id.clone(),
            product_name: item.product_name.clone(),
            pack_kg: item.pack_kg.kg(),
            quantity: item.quantity,
            unit_price_idr: item.unit_price.amount(),
            line_total_idr: item.line_total.amount(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteResponseDto {
    pub items: Vec<QuoteItemDto>,
    pub subtotal_idr: u64,
    pub subscription_discount_idr: u64,
    pub promo_discount_idr: u64,
    pub total_discount_idr: u64,
    pub shipping_fee_idr: u64,
    pub free_shipping_threshold_idr: u64,
    pub total_idr: u64,
    pub applied_promo: Option<String>,
}

impl From<&PriceQuote> for QuoteResponseDto {
    fn from(q: &PriceQuote) -> Self {
        Self {
            items: q.items.iter().map(QuoteItemDto::from).collect(),
            subtotal_idr: q.subtotal.amount(),
            subscription_discount_idr: q.subscription_discount.amount(),
            promo_discount_idr: q.promo_discount.amount(),
            total_discount_idr: q.total_discount.amount(),
            shipping_fee_idr: q.shipping_fee.amount(),
            free_shipping_threshold_idr: q.free_shipping_threshold.amount(),
            total_idr: q.total.amount(),
            applied_promo: q.applied_promo.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderDto {
    pub id: String,
    pub customer_name: String,
    pub customer_phone: String,
    pub delivery_address: String,
    pub city: String,
    pub is_subscription: bool,
    pub items: Vec<QuoteItemDto>,
    pub subtotal_idr: u64,
    pub discount_idr: u64,
    pub shipping_fee_idr: u64,
    pub total_idr: u64,
    pub promo_code: Option<String>,
    pub status: OrderStatus,
    pub created_at: String,
}

impl From<&Order> for OrderDto {
    fn from(o: &Order) -> Self {
        Self {
            id: o.id.0.clone(),
            customer_name: o.customer_name.clone(),
            customer_phone: o.customer_phone.clone(),
            delivery_address: o.delivery_address.clone(),
            city: o.city.clone(),
            is_subscription: o.is_subscription,
            items: o.items.iter().map(QuoteItemDto::from).collect(),
            subtotal_idr: o.subtotal.amount(),
            discount_idr: o.discount.amount(),
            shipping_fee_idr: o.shipping_fee.amount(),
            total_idr: o.total.amount(),
            promo_code: o.promo_code.clone(),
            status: o.status,
            created_at: o.created_at.clone(),
        }
    }
}

pub type CookingGuideDto = CookingGuideResult;
