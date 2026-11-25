//! Cloudflare Workers (WASM / `worker-rs`) inbound HTTP & Supabase outbound adapters.
//!
//! Because `domain` and `application` depend on zero runtime-specific crates,
//! the exact same `StoreApplicationService` runs unmodified inside Cloudflare Workers
//! when compiled with `--features cloudflare --no-default-features`.

#[cfg(feature = "cloudflare")]
pub mod edge {
    use crate::application::dtos::{
        CatalogFilter, PlaceOrderCommand, QuoteCartCommand, RestockProductCommand,
        UpdateOrderStatusCommand,
    };
    use crate::application::use_cases::StoreApplicationService;
    use crate::domain::entities::{Order, RiceProduct};
    use crate::domain::ports::{OrderRepository, ProductRepository};
    use crate::domain::value_objects::{DomainError, OrderId, RiceGrade};
    use crate::infrastructure::persistence::{
        InMemoryEventPublisher, InMemoryOrderRepository, InMemoryProductRepository,
    };
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use worker::{
        event, Context, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result,
        Router,
    };

    #[derive(Clone)]
    struct SupabaseConfig {
        url: String,
        service_key: String,
    }

    impl SupabaseConfig {
        fn from_env(env: &Env) -> Option<Self> {
            let url = env.var("SUPABASE_URL").ok()?.to_string();
            let service_key = env.secret("SUPABASE_SERVICE_ROLE_KEY").ok()?.to_string();
            if url.is_empty() || service_key.is_empty() {
                None
            } else {
                Some(Self { url, service_key })
            }
        }

        async fn rest_request(
            &self,
            method: Method,
            path: &str,
            body_json: Option<String>,
            prefer: Option<&str>,
        ) -> std::result::Result<String, String> {
            let headers = Headers::new();
            headers
                .set("apikey", &self.service_key)
                .map_err(|e| e.to_string())?;
            headers
                .set("Authorization", &format!("Bearer {}", self.service_key))
                .map_err(|e| e.to_string())?;
            headers
                .set("Content-Type", "application/json")
                .map_err(|e| e.to_string())?;
            if let Some(p) = prefer {
                headers.set("Prefer", p).map_err(|e| e.to_string())?;
            }

            let mut init = RequestInit::new();
            init.with_method(method).with_headers(headers);
            if let Some(body) = body_json {
                init.with_body(Some(body.into()));
            }

            let full_url = format!("{}/rest/v1/{}", self.url.trim_end_matches('/'), path);
            let req = Request::new_with_init(&full_url, &init).map_err(|e| e.to_string())?;
            let mut resp = Fetch::Request(req)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = resp.status_code();
            let text = resp.text().await.map_err(|e| e.to_string())?;
            if status >= 400 {
                return Err(format!("Supabase HTTP {status}: {text}"));
            }
            Ok(text)
        }
    }

    #[derive(Serialize, Deserialize)]
    struct ProductRow {
        id: String,
        sort_order: usize,
        payload: RiceProduct,
    }

    #[derive(Serialize, Deserialize)]
    struct OrderRow {
        id: String,
        created_at: String,
        payload: Order,
    }

    struct SupabaseProductRepository {
        cfg: SupabaseConfig,
    }

    impl SupabaseProductRepository {
        async fn ensure_seeded(&self) -> Vec<RiceProduct> {
            let defaults = InMemoryProductRepository::seeded().list_all().await;
            let rows: Vec<ProductRow> = defaults
                .iter()
                .enumerate()
                .map(|(idx, p)| ProductRow {
                    id: p.id.clone(),
                    sort_order: idx,
                    payload: p.clone(),
                })
                .collect();
            if let Ok(body) = serde_json::to_string(&rows) {
                let _ = self
                    .cfg
                    .rest_request(
                        Method::Post,
                        "rice_products?on_conflict=id",
                        Some(body),
                        Some("resolution=merge-duplicates"),
                    )
                    .await;
            }
            defaults
        }
    }

    #[async_trait::async_trait(?Send)]
    impl ProductRepository for SupabaseProductRepository {
        async fn list_all(&self) -> Vec<RiceProduct> {
            match self
                .cfg
                .rest_request(
                    Method::Get,
                    "rice_products?select=id,sort_order,payload&order=sort_order.asc",
                    None,
                    None,
                )
                .await
            {
                Ok(text) => match serde_json::from_str::<Vec<ProductRow>>(&text) {
                    Ok(rows) if !rows.is_empty() => rows.into_iter().map(|r| r.payload).collect(),
                    _ => self.ensure_seeded().await,
                },
                Err(_) => InMemoryProductRepository::seeded().list_all().await,
            }
        }

        async fn find_by_id(&self, id: &str) -> Option<RiceProduct> {
            let path = format!("rice_products?id=eq.{id}&select=id,sort_order,payload");
            if let Ok(text) = self.cfg.rest_request(Method::Get, &path, None, None).await {
                if let Ok(rows) = serde_json::from_str::<Vec<ProductRow>>(&text) {
                    if let Some(found) = rows.into_iter().next() {
                        return Some(found.payload);
                    }
                }
            }
            let seeded = self.ensure_seeded().await;
            seeded.into_iter().find(|p| p.id == id || p.slug == id)
        }

        async fn save(&self, product: RiceProduct) -> std::result::Result<(), DomainError> {
            let row = serde_json::json!({
                "id": product.id,
                "payload": product
            });
            let body = serde_json::to_string(&row)
                .map_err(|e| DomainError::InvalidCustomerInfo(e.to_string()))?;
            let path = format!("rice_products?id=eq.{}", product.id);
            self.cfg
                .rest_request(Method::Patch, &path, Some(body), None)
                .await
                .map_err(DomainError::InvalidCustomerInfo)?;
            Ok(())
        }
    }

    struct SupabaseOrderRepository {
        cfg: SupabaseConfig,
    }

    #[async_trait::async_trait(?Send)]
    impl OrderRepository for SupabaseOrderRepository {
        async fn list_all(&self) -> Vec<Order> {
            match self
                .cfg
                .rest_request(
                    Method::Get,
                    "rice_orders?select=id,created_at,payload&order=created_at.desc",
                    None,
                    None,
                )
                .await
            {
                Ok(text) => serde_json::from_str::<Vec<OrderRow>>(&text)
                    .map(|rows| rows.into_iter().map(|r| r.payload).collect())
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        }

        async fn find_by_id(&self, id: &OrderId) -> Option<Order> {
            let path = format!("rice_orders?id=eq.{}&select=id,created_at,payload", id.0);
            if let Ok(text) = self.cfg.rest_request(Method::Get, &path, None, None).await {
                if let Ok(rows) = serde_json::from_str::<Vec<OrderRow>>(&text) {
                    return rows.into_iter().next().map(|r| r.payload);
                }
            }
            None
        }

        async fn save(&self, order: Order) -> std::result::Result<(), DomainError> {
            let row = OrderRow {
                id: order.id.0.clone(),
                created_at: order.created_at.clone(),
                payload: order,
            };
            let body = serde_json::to_string(&row)
                .map_err(|e| DomainError::InvalidCustomerInfo(e.to_string()))?;
            self.cfg
                .rest_request(
                    Method::Post,
                    "rice_orders?on_conflict=id",
                    Some(body),
                    Some("resolution=merge-duplicates"),
                )
                .await
                .map_err(DomainError::InvalidCustomerInfo)?;
            Ok(())
        }
    }

    fn with_cors(mut resp: Response) -> Result<Response> {
        let headers = resp.headers_mut();
        headers.set("Access-Control-Allow-Origin", "*")?;
        headers.set("Access-Control-Allow-Methods", "GET, POST, PATCH, OPTIONS")?;
        headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization")?;
        Ok(resp)
    }

    fn json_status<T: Serialize>(value: &T, status: u16) -> Result<Response> {
        let resp = Response::from_json(value)?.with_status(status);
        with_cors(resp)
    }

    fn err_response(err: DomainError) -> Result<Response> {
        let (status, code) = match &err {
            DomainError::ProductNotFound(_) | DomainError::OrderNotFound(_) => (404, "NOT_FOUND"),
            DomainError::InsufficientStock { .. }
            | DomainError::InvalidStatusTransition { .. } => (409, "CONFLICT"),
            _ => (400, "BAD_REQUEST"),
        };
        json_status(
            &serde_json::json!({ "error": err.to_string(), "code": code }),
            status,
        )
    }

    #[derive(Deserialize)]
    struct ProductsQuery {
        grade: Option<RiceGrade>,
        q: Option<String>,
    }

    #[derive(Deserialize)]
    struct CookingQuery {
        portions: Option<u16>,
    }

    #[event(fetch)]
    pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
        if req.method() == Method::Options {
            return with_cors(Response::empty()?.with_status(204));
        }

        let (products, orders): (Arc<dyn ProductRepository>, Arc<dyn OrderRepository>) =
            if let Some(cfg) = SupabaseConfig::from_env(&env) {
                (
                    Arc::new(SupabaseProductRepository { cfg: cfg.clone() }),
                    Arc::new(SupabaseOrderRepository { cfg }),
                )
            } else {
                (
                    Arc::new(InMemoryProductRepository::seeded()),
                    Arc::new(InMemoryOrderRepository::new()),
                )
            };

        let events = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(StoreApplicationService::new(products, orders, events));

        let resp = Router::with_data(service)
            .get_async("/api/health", |_, _| async move {
                json_status(
                    &serde_json::json!({
                        "status": "ok",
                        "service": "padinusa-api",
                        "runtime": "cloudflare-workers",
                        "architecture": "ddd-hexagonal"
                    }),
                    200,
                )
            })
            .get_async("/api/products", |req, ctx| async move {
                let query = req.query::<ProductsQuery>().unwrap_or(ProductsQuery {
                    grade: None,
                    q: None,
                });
                let items = ctx
                    .data
                    .list_products(CatalogFilter {
                        grade: query.grade,
                        query: query.q,
                    })
                    .await;
                json_status(&serde_json::json!({ "products": items }), 200)
            })
            .get_async("/api/products/:id", |_, ctx| async move {
                let id = ctx.param("id").map(|s| s.as_str()).unwrap_or("");
                match ctx.data.get_product(id).await {
                    Ok(product) => json_status(&serde_json::json!({ "product": product }), 200),
                    Err(e) => err_response(e),
                }
            })
            .post_async("/api/products/:id/restock", |mut req, ctx| async move {
                let id = ctx.param("id").map(|s| s.as_str()).unwrap_or("").to_string();
                let cmd: RestockProductCommand = match req.json().await {
                    Ok(c) => c,
                    Err(e) => return json_status(&serde_json::json!({ "error": e.to_string() }), 400),
                };
                match ctx.data.restock_product(&id, cmd).await {
                    Ok(product) => json_status(&serde_json::json!({ "product": product }), 200),
                    Err(e) => err_response(e),
                }
            })
            .get_async("/api/products/:id/cooking-guide", |req, ctx| async move {
                let id = ctx.param("id").map(|s| s.as_str()).unwrap_or("").to_string();
                let portions = req
                    .query::<CookingQuery>()
                    .ok()
                    .and_then(|q| q.portions)
                    .unwrap_or(4);
                match ctx.data.calculate_cooking_guide(&id, portions).await {
                    Ok(guide) => json_status(&serde_json::json!({ "guide": guide }), 200),
                    Err(e) => err_response(e),
                }
            })
            .post_async("/api/cart/quote", |mut req, ctx| async move {
                let cmd: QuoteCartCommand = match req.json().await {
                    Ok(c) => c,
                    Err(e) => return json_status(&serde_json::json!({ "error": e.to_string() }), 400),
                };
                match ctx.data.quote_cart(cmd).await {
                    Ok(quote) => json_status(&serde_json::json!({ "quote": quote }), 200),
                    Err(e) => err_response(e),
                }
            })
            .get_async("/api/orders", |_, ctx| async move {
                let orders = ctx.data.list_orders().await;
                json_status(&serde_json::json!({ "orders": orders }), 200)
            })
            .post_async("/api/orders", |mut req, ctx| async move {
                let cmd: PlaceOrderCommand = match req.json().await {
                    Ok(c) => c,
                    Err(e) => return json_status(&serde_json::json!({ "error": e.to_string() }), 400),
                };
                match ctx.data.place_order(cmd).await {
                    Ok(order) => json_status(&serde_json::json!({ "order": order }), 201),
                    Err(e) => err_response(e),
                }
            })
            .get_async("/api/orders/:id", |_, ctx| async move {
                let id = ctx.param("id").map(|s| s.as_str()).unwrap_or("");
                match ctx.data.get_order(id).await {
                    Ok(order) => json_status(&serde_json::json!({ "order": order }), 200),
                    Err(e) => err_response(e),
                }
            })
            .patch_async("/api/orders/:id/status", |mut req, ctx| async move {
                let id = ctx.param("id").map(|s| s.as_str()).unwrap_or("").to_string();
                let cmd: UpdateOrderStatusCommand = match req.json().await {
                    Ok(c) => c,
                    Err(e) => return json_status(&serde_json::json!({ "error": e.to_string() }), 400),
                };
                match ctx.data.update_order_status(&id, cmd).await {
                    Ok(order) => json_status(&serde_json::json!({ "order": order }), 200),
                    Err(e) => err_response(e),
                }
            })
            .run(req, env)
            .await?;

        with_cors(resp)
    }
}
