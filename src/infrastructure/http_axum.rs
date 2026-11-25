use crate::application::dtos::{
    CatalogFilter, PlaceOrderCommand, QuoteCartCommand, RestockProductCommand,
    UpdateOrderStatusCommand,
};
use crate::application::use_cases::StoreApplicationService;
use crate::domain::value_objects::{DomainError, RiceGrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<StoreApplicationService>,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
    code: &'static str,
}

struct ApiError(DomainError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self.0 {
            DomainError::ProductNotFound(_) | DomainError::OrderNotFound(_) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND")
            }
            DomainError::InsufficientStock { .. }
            | DomainError::InvalidStatusTransition { .. } => {
                (StatusCode::CONFLICT, "CONFLICT")
            }
            _ => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
        };

        let body = ApiErrorBody {
            error: self.0.to_string(),
            code,
        };
        (status, Json(body)).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct ProductsQuery {
    pub grade: Option<RiceGrade>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CookingQuery {
    pub portions: Option<u16>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    architecture: &'static str,
}

/// Builds the Axum router for VPS / container deployment.
pub fn build_router(service: Arc<StoreApplicationService>) -> Router {
    let state = AppState { service };
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/products", get(list_products_handler))
        .route("/api/products/{id}", get(get_product_handler))
        .route("/api/products/{id}/restock", post(restock_product_handler))
        .route(
            "/api/products/{id}/cooking-guide",
            get(cooking_guide_handler),
        )
        .route("/api/cart/quote", post(quote_cart_handler))
        .route("/api/orders", get(list_orders_handler).post(place_order_handler))
        .route("/api/orders/{id}", get(get_order_handler))
        .route("/api/orders/{id}/status", patch(update_order_status_handler))
        .layer(cors)
        .with_state(state)
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "padinusa-api",
        architecture: "ddd-hexagonal",
    })
}

async fn list_products_handler(
    State(state): State<AppState>,
    Query(params): Query<ProductsQuery>,
) -> Json<serde_json::Value> {
    let items = state
        .service
        .list_products(CatalogFilter {
            grade: params.grade,
            query: params.q,
        })
        .await;
    Json(serde_json::json!({ "products": items }))
}

async fn get_product_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let product = state.service.get_product(&id).await.map_err(ApiError)?;
    Ok(Json(serde_json::json!({ "product": product })))
}

async fn cooking_guide_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<CookingQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let portions = params.portions.unwrap_or(4);
    let guide = state
        .service
        .calculate_cooking_guide(&id, portions)
        .await
        .map_err(ApiError)?;
    Ok(Json(serde_json::json!({ "guide": guide })))
}

async fn quote_cart_handler(
    State(state): State<AppState>,
    Json(cmd): Json<QuoteCartCommand>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let quote = state.service.quote_cart(cmd).await.map_err(ApiError)?;
    Ok(Json(serde_json::json!({ "quote": quote })))
}

async fn place_order_handler(
    State(state): State<AppState>,
    Json(cmd): Json<PlaceOrderCommand>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let order = state.service.place_order(cmd).await.map_err(ApiError)?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "order": order }))))
}

async fn list_orders_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let orders = state.service.list_orders().await;
    Json(serde_json::json!({ "orders": orders }))
}

async fn get_order_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let order = state.service.get_order(&id).await.map_err(ApiError)?;
    Ok(Json(serde_json::json!({ "order": order })))
}

async fn update_order_status_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(cmd): Json<UpdateOrderStatusCommand>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let order = state
        .service
        .update_order_status(&id, cmd)
        .await
        .map_err(ApiError)?;
    Ok(Json(serde_json::json!({ "order": order })))
}

async fn restock_product_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(cmd): Json<RestockProductCommand>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let product = state
        .service
        .restock_product(&id, cmd)
        .await
        .map_err(ApiError)?;
    Ok(Json(serde_json::json!({ "product": product })))
}
