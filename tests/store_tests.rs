use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use padinusa_backend::application::dtos::{
    CartLineInput, CatalogFilter, PlaceOrderCommand,
};
use padinusa_backend::application::use_cases::StoreApplicationService;
use padinusa_backend::domain::events::DomainEvent;
use padinusa_backend::domain::ports::ProductRepository;
use padinusa_backend::domain::services::{CookingCalculatorService, PricingService};
use padinusa_backend::domain::value_objects::{
    DomainError, MoneyIdr, OrderStatus, PromoCode, RiceGrade, WeightKg,
};
use padinusa_backend::infrastructure::http_axum::build_router;
use padinusa_backend::infrastructure::persistence::{
    InMemoryEventPublisher, InMemoryOrderRepository, InMemoryProductRepository,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn make_service() -> (
    Arc<StoreApplicationService>,
    Arc<InMemoryProductRepository>,
    Arc<InMemoryEventPublisher>,
) {
    let products = Arc::new(InMemoryProductRepository::seeded());
    let orders = Arc::new(InMemoryOrderRepository::new());
    let events = Arc::new(InMemoryEventPublisher::new());
    let service = Arc::new(StoreApplicationService::new(
        products.clone(),
        orders,
        events.clone(),
    ));
    (service, products, events)
}

#[test]
fn weight_kg_accepts_only_supported_pack_sizes() {
    for valid in [2, 5, 10, 25] {
        assert!(WeightKg::try_new(valid).is_ok());
    }
    assert_eq!(
        WeightKg::try_new(3),
        Err(DomainError::InvalidPackWeight(3))
    );
    assert_eq!(
        WeightKg::try_new(50),
        Err(DomainError::InvalidPackWeight(50))
    );
}

#[test]
fn promo_code_and_status_transitions_enforce_domain_invariants() {
    assert_eq!(PromoCode::parse(" panenbaru "), Ok(PromoCode::PanenBaru));
    assert_eq!(
        PromoCode::parse("FAKE"),
        Err(DomainError::InvalidPromoCode("FAKE".to_string()))
    );

    let s = OrderStatus::MenungguKonfirmasi;
    let s = s.transition_to(OrderStatus::DiprosesPenggilingan).unwrap();
    let s = s.transition_to(OrderStatus::Dikirim).unwrap();
    let s = s.transition_to(OrderStatus::Selesai).unwrap();
    assert!(s.transition_to(OrderStatus::Dibatalkan).is_err());
}

#[tokio::test]
async fn pricing_service_computes_bulk_subscription_promo_and_shipping() {
    let repo = InMemoryProductRepository::seeded();
    let pandan = repo.find_by_id("pandan-wangi-cianjur").await.unwrap();
    // 22_000/kg * 5kg = 110_000, 5kg bulk discount 3% = 3_300 -> 106_700 per pack
    let unit_5kg = PricingService::unit_pack_price(&pandan, WeightKg::try_new(5).unwrap()).unwrap();
    assert_eq!(unit_5kg, MoneyIdr::new(106_700));

    // Qty 2 -> subtotal 213_400; subscription 10% -> 21_340; after_sub = 192_060
    // Promo PANENBARU 10% of 192_060 = 19_206; net = 172_854 >= 150_000 Jabodetabek -> Free shipping!
    let quote = PricingService::quote_order(
        &[(&pandan, WeightKg::try_new(5).unwrap(), 2)],
        "Jakarta Selatan",
        true,
        Some("PANENBARU"),
    )
    .unwrap();

    assert_eq!(quote.subtotal, MoneyIdr::new(213_400));
    assert_eq!(quote.subscription_discount, MoneyIdr::new(21_340));
    assert_eq!(quote.promo_discount, MoneyIdr::new(19_206));
    assert_eq!(quote.shipping_fee, MoneyIdr::ZERO);
    assert_eq!(quote.total, MoneyIdr::new(172_854));

    // Outside Jabodetabek below 350_000 pays 32_000 unless ONGKIRGRATIS
    let quote_bandung = PricingService::quote_order(
        &[(&pandan, WeightKg::try_new(2).unwrap(), 1)],
        "Bandung",
        false,
        None,
    )
    .unwrap();
    assert_eq!(quote_bandung.shipping_fee, MoneyIdr::new(32_000));

    let quote_bandung_free = PricingService::quote_order(
        &[(&pandan, WeightKg::try_new(2).unwrap(), 1)],
        "Bandung",
        false,
        Some("ONGKIRGRATIS"),
    )
    .unwrap();
    assert_eq!(quote_bandung_free.shipping_fee, MoneyIdr::ZERO);
}

#[tokio::test]
async fn cooking_calculator_validates_portions_and_computes_water_ratio() {
    let repo = InMemoryProductRepository::seeded();
    let merah = repo.find_by_id("beras-merah-cempo").await.unwrap();

    assert_eq!(
        CookingCalculatorService::calculate(&merah.texture, 0),
        Err(DomainError::InvalidPortions)
    );
    assert_eq!(
        CookingCalculatorService::calculate(&merah.texture, 51),
        Err(DomainError::InvalidPortions)
    );

    let guide = CookingCalculatorService::calculate(&merah.texture, 4).unwrap();
    assert_eq!(guide.rice_grams, 300);
    assert!(guide.water_ml > 650);
    assert_eq!(guide.soak_minutes, 30);
}

#[tokio::test]
async fn place_order_reserves_stock_and_emits_domain_events() {
    let (service, products, events) = make_service();
    let before = products.find_by_id("beras-hitam-wulung").await.unwrap().stock_kg;

    // Request exceeds available stock (120 kg) -> fails without mutating stock
    let err = service
        .place_order(PlaceOrderCommand {
            customer_name: "Budi Santoso".to_string(),
            customer_phone: "081234567890".to_string(),
            delivery_address: "Jl. Kemang Raya No. 12".to_string(),
            city: "Jakarta Selatan".to_string(),
            is_subscription: false,
            promo_code: None,
            items: vec![CartLineInput {
                product_id: "beras-hitam-wulung".to_string(),
                pack_kg: 5,
                quantity: 30, // 150 kg > 120 kg
            }],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::InsufficientStock { .. }));
    assert_eq!(
        products.find_by_id("beras-hitam-wulung").await.unwrap().stock_kg,
        before
    );

    // Valid order of 2 x 5kg = 10kg
    let order = service
        .place_order(PlaceOrderCommand {
            customer_name: "Rina Wijaya".to_string(),
            customer_phone: "081298765432".to_string(),
            delivery_address: "Jl. Senopati No. 88, Kebayoran Baru".to_string(),
            city: "Jakarta Selatan".to_string(),
            is_subscription: true,
            promo_code: Some("PANENBARU".to_string()),
            items: vec![CartLineInput {
                product_id: "beras-hitam-wulung".to_string(),
                pack_kg: 5,
                quantity: 2,
            }],
        })
        .await
        .unwrap();

    assert_eq!(order.status, OrderStatus::MenungguKonfirmasi);
    assert_eq!(
        products.find_by_id("beras-hitam-wulung").await.unwrap().stock_kg,
        before - 10
    );

    let recorded = events.recorded_events();
    assert!(recorded
        .iter()
        .any(|e| matches!(e, DomainEvent::StockReserved { reserved_kg: 10, .. })));
    assert!(recorded
        .iter()
        .any(|e| matches!(e, DomainEvent::OrderPlaced { .. })));
}

#[tokio::test]
async fn catalog_filtering_and_http_api_end_to_end() {
    let (service, _, _) = make_service();
    let organik = service
        .list_products(CatalogFilter {
            grade: Some(RiceGrade::Organik),
            query: None,
        })
        .await;
    assert_eq!(organik.len(), 2);

    let app = build_router(service);

    // Health check
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Quote cart
    let quote_req = Request::builder()
        .method("POST")
        .uri("/api/cart/quote")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "items": [{ "product_id": "pandan-wangi-cianjur", "pack_kg": 5, "quantity": 1 }],
                "city": "Jakarta",
                "is_subscription": false,
                "promo_code": "PANENBARU"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(quote_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json_val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_val["quote"]["applied_promo"], "PANENBARU");

    // Place order via HTTP
    let order_req = Request::builder()
        .method("POST")
        .uri("/api/orders")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "customer_name": "Andi Pratama",
                "customer_phone": "081311223344",
                "delivery_address": "Jl. Sudirman Kav 52, SCBD",
                "city": "Jakarta Selatan",
                "is_subscription": false,
                "promo_code": null,
                "items": [{ "product_id": "rojolele-delanggu", "pack_kg": 10, "quantity": 1 }]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(order_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let order_id = created["order"]["id"].as_str().unwrap().to_string();

    // Advance status validly
    let patch_ok = Request::builder()
        .method("PATCH")
        .uri(format!("/api/orders/{order_id}/status"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "status": "diproses_penggilingan" }).to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(patch_ok).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Invalid status jump (DiprosesPenggilingan -> Selesai directly) returns 409 Conflict
    let patch_conflict = Request::builder()
        .method("PATCH")
        .uri(format!("/api/orders/{order_id}/status"))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "status": "selesai" }).to_string()))
        .unwrap();
    let res = app.oneshot(patch_conflict).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}
