use crate::application::dtos::{
    CatalogFilter, CookingGuideDto, OrderDto, PlaceOrderCommand, ProductDto, QuoteCartCommand,
    QuoteResponseDto, RestockProductCommand, UpdateOrderStatusCommand,
};
use crate::domain::entities::{Order, RiceProduct};
use crate::domain::events::DomainEvent;
use crate::domain::ports::{EventPublisher, OrderRepository, ProductRepository};
use crate::domain::services::{CookingCalculatorService, PricingService};
use crate::domain::value_objects::{DomainError, OrderId, OrderStatus, WeightKg};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

/// Application service orchestrating all PadiNusa store use cases via outbound ports.
pub struct StoreApplicationService {
    products: Arc<dyn ProductRepository>,
    orders: Arc<dyn OrderRepository>,
    events: Arc<dyn EventPublisher>,
}

impl StoreApplicationService {
    pub fn new(
        products: Arc<dyn ProductRepository>,
        orders: Arc<dyn OrderRepository>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            products,
            orders,
            events,
        }
    }

    pub async fn list_products(&self, filter: CatalogFilter) -> Vec<ProductDto> {
        let all = self.products.list_all().await;
        all.iter()
            .filter(|p| filter.grade.map_or(true, |g| p.grade == g))
            .filter(|p| {
                filter
                    .query
                    .as_ref()
                    .map(|q| q.trim().to_lowercase())
                    .filter(|q| !q.is_empty())
                    .map_or(true, |q| {
                        p.name.to_lowercase().contains(&q)
                            || p.origin_region.to_lowercase().contains(&q)
                            || p.tagline.to_lowercase().contains(&q)
                    })
            })
            .map(ProductDto::from)
            .collect()
    }

    pub async fn get_product(&self, id: &str) -> Result<ProductDto, DomainError> {
        self.products
            .find_by_id(id)
            .await
            .map(|p| ProductDto::from(&p))
            .ok_or_else(|| DomainError::ProductNotFound(id.to_string()))
    }

    pub async fn calculate_cooking_guide(
        &self,
        product_id: &str,
        portions: u16,
    ) -> Result<CookingGuideDto, DomainError> {
        let product = self
            .products
            .find_by_id(product_id)
            .await
            .ok_or_else(|| DomainError::ProductNotFound(product_id.to_string()))?;
        CookingCalculatorService::calculate(&product.texture, portions)
    }

    pub async fn quote_cart(&self, cmd: QuoteCartCommand) -> Result<QuoteResponseDto, DomainError> {
        let resolved = self.resolve_products(&cmd.items).await?;
        let line_refs: Vec<(&RiceProduct, WeightKg, u16)> = resolved
            .iter()
            .map(|(p, w, qty)| (p, *w, *qty))
            .collect();
        let quote = PricingService::quote_order(
            &line_refs,
            &cmd.city,
            cmd.is_subscription,
            cmd.promo_code.as_deref(),
        )?;
        Ok(QuoteResponseDto::from(&quote))
    }

    pub async fn place_order(&self, cmd: PlaceOrderCommand) -> Result<OrderDto, DomainError> {
        if cmd.customer_name.trim().is_empty() {
            return Err(DomainError::InvalidCustomerInfo(
                "Nama penerima wajib diisi".to_string(),
            ));
        }
        if cmd.customer_phone.trim().len() < 8 {
            return Err(DomainError::InvalidCustomerInfo(
                "Nomor WhatsApp/telepon tidak valid".to_string(),
            ));
        }
        if cmd.delivery_address.trim().len() < 8 {
            return Err(DomainError::InvalidCustomerInfo(
                "Alamat pengiriman terlalu singkat".to_string(),
            ));
        }
        if cmd.city.trim().is_empty() {
            return Err(DomainError::InvalidCustomerInfo(
                "Kota tujuan wajib diisi".to_string(),
            ));
        }

        let resolved = self.resolve_products(&cmd.items).await?;
        let line_refs: Vec<(&RiceProduct, WeightKg, u16)> = resolved
            .iter()
            .map(|(p, w, qty)| (p, *w, *qty))
            .collect();

        // Validate pricing and promo first before mutating stock
        let quote = PricingService::quote_order(
            &line_refs,
            &cmd.city,
            cmd.is_subscription,
            cmd.promo_code.as_deref(),
        )?;

        // Aggregate requested kg per product so multi-pack orders for the same cultivar validate total stock
        let mut required_kg_by_product: HashMap<String, u32> = HashMap::new();
        for (product, weight, qty) in &resolved {
            let kg = u32::from(weight.kg()) * u32::from(*qty);
            *required_kg_by_product.entry(product.id.clone()).or_insert(0) += kg;
        }

        let mut updated_products = Vec::with_capacity(required_kg_by_product.len());
        for (product_id, requested_kg) in &required_kg_by_product {
            let mut product = self
                .products
                .find_by_id(product_id)
                .await
                .ok_or_else(|| DomainError::ProductNotFound(product_id.clone()))?;
            product.reserve_stock(*requested_kg)?;
            updated_products.push((product, *requested_kg));
        }

        for (product, reserved_kg) in updated_products {
            let remaining_kg = product.stock_kg;
            let pid = product.id.clone();
            self.products.save(product).await?;
            self.events
                .publish(DomainEvent::StockReserved {
                    product_id: pid,
                    reserved_kg,
                    remaining_kg,
                })
                .await;
        }

        let short_uuid = uuid::Uuid::new_v4().simple().to_string();
        let order_id = OrderId(format!(
            "PN-{}-{}",
            Utc::now().format("%Y%m%d"),
            short_uuid[..6].to_uppercase()
        ));

        let order = Order {
            id: order_id.clone(),
            customer_name: cmd.customer_name.trim().to_string(),
            customer_phone: cmd.customer_phone.trim().to_string(),
            delivery_address: cmd.delivery_address.trim().to_string(),
            city: cmd.city.trim().to_string(),
            is_subscription: cmd.is_subscription,
            items: quote.items,
            subtotal: quote.subtotal,
            discount: quote.total_discount,
            shipping_fee: quote.shipping_fee,
            total: quote.total,
            promo_code: quote.applied_promo,
            status: OrderStatus::MenungguKonfirmasi,
            created_at: Utc::now().to_rfc3339(),
        };

        self.orders.save(order.clone()).await?;
        self.events
            .publish(DomainEvent::OrderPlaced {
                order_id,
                total_idr: order.total,
                item_count: order.items.len(),
            })
            .await;

        Ok(OrderDto::from(&order))
    }

    pub async fn list_orders(&self) -> Vec<OrderDto> {
        self.orders
            .list_all()
            .await
            .iter()
            .map(OrderDto::from)
            .collect()
    }

    pub async fn get_order(&self, id: &str) -> Result<OrderDto, DomainError> {
        self.orders
            .find_by_id(&OrderId(id.to_string()))
            .await
            .map(|o| OrderDto::from(&o))
            .ok_or_else(|| DomainError::OrderNotFound(id.to_string()))
    }

    pub async fn update_order_status(
        &self,
        id: &str,
        cmd: UpdateOrderStatusCommand,
    ) -> Result<OrderDto, DomainError> {
        let mut order = self
            .orders
            .find_by_id(&OrderId(id.to_string()))
            .await
            .ok_or_else(|| DomainError::OrderNotFound(id.to_string()))?;

        order.advance_status(cmd.status)?;
        self.orders.save(order.clone()).await?;
        self.events
            .publish(DomainEvent::OrderStatusChanged {
                order_id: order.id.clone(),
                new_status: order.status,
            })
            .await;
        Ok(OrderDto::from(&order))
    }

    pub async fn restock_product(
        &self,
        id: &str,
        cmd: RestockProductCommand,
    ) -> Result<ProductDto, DomainError> {
        let mut product = self
            .products
            .find_by_id(id)
            .await
            .ok_or_else(|| DomainError::ProductNotFound(id.to_string()))?;
        product.restock(cmd.added_kg);
        let dto = ProductDto::from(&product);
        self.products.save(product).await?;
        Ok(dto)
    }

    async fn resolve_products(
        &self,
        items: &[crate::application::dtos::CartLineInput],
    ) -> Result<Vec<(RiceProduct, WeightKg, u16)>, DomainError> {
        if items.is_empty() {
            return Err(DomainError::EmptyCart);
        }
        let mut resolved = Vec::with_capacity(items.len());
        for line in items {
            let weight = WeightKg::try_new(line.pack_kg)?;
            if line.quantity == 0 {
                return Err(DomainError::InvalidQuantity);
            }
            let product = self
                .products
                .find_by_id(&line.product_id)
                .await
                .ok_or_else(|| DomainError::ProductNotFound(line.product_id.clone()))?;
            resolved.push((product, weight, line.quantity));
        }
        Ok(resolved)
    }
}
