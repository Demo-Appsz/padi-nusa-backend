use crate::domain::entities::{Order, RiceProduct};
use crate::domain::events::DomainEvent;
use crate::domain::ports::{EventPublisher, OrderRepository, ProductRepository};
use crate::domain::value_objects::{
    DomainError, MoneyIdr, OrderId, RiceGrade, RiceTextureProfile, WeightKg,
};
use std::collections::HashMap;
use std::sync::RwLock;

/// Thread-safe, runtime-agnostic in-memory product repository seeded with curated Indonesian rice cultivars.
pub struct InMemoryProductRepository {
    store: RwLock<Vec<RiceProduct>>,
}

impl InMemoryProductRepository {
    pub fn seeded() -> Self {
        let products = vec![
            RiceProduct {
                id: "pandan-wangi-cianjur".to_string(),
                slug: "pandan-wangi-cianjur".to_string(),
                name: "Pandan Wangi Cianjur".to_string(),
                tagline: "Aroma pandan alami dengan bulir pendek pulen memikat".to_string(),
                description: "Dipanen dari lereng subur Warungkondang, Cianjur. Digiling presisi suhu rendah untuk mempertahankan minyak esensial pandan alami dan kelembutan tekstur khas hidangan keluarga Nusantara.".to_string(),
                origin_region: "Cianjur, Jawa Barat".to_string(),
                harvest_batch: "PANEN-26-CJR04".to_string(),
                milled_date: "19 Nov 2026".to_string(),
                grade: RiceGrade::Artisanal,
                base_price_per_kg: MoneyIdr::new(22_000),
                stock_kg: 480,
                available_packs: vec![
                    WeightKg::try_new(2).unwrap(),
                    WeightKg::try_new(5).unwrap(),
                    WeightKg::try_new(10).unwrap(),
                    WeightKg::try_new(25).unwrap(),
                ],
                texture: RiceTextureProfile {
                    pulen_score: 9,
                    aroma_score: 10,
                    glycemic_label: "Sedang (IG 58)".to_string(),
                    water_ratio: 1.25,
                    soak_minutes: 10,
                    cook_minutes: 22,
                },
                tasting_notes: vec![
                    "Pandan Segar".to_string(),
                    "Manis Susu".to_string(),
                    "Pulen Lembut".to_string(),
                ],
                image_url: "/images/pandan-wangi.jpg".to_string(),
                badge: Some("Best Seller".to_string()),
            },
            RiceProduct {
                id: "rojolele-delanggu".to_string(),
                slug: "rojolele-delanggu".to_string(),
                name: "Rojolele Delanggu".to_string(),
                tagline: "Warisan Keraton Klaten yang putih bening dan wangi lembut".to_string(),
                description: "Varietas legendaris Delanggu yang diairi mata air pegunungan Merapi. Menghasilkan nasi putih berkilau, tidak mudah basi, dan tekstur seimbang untuk lauk bersantan maupun panggang.".to_string(),
                origin_region: "Delanggu, Klaten".to_string(),
                harvest_batch: "PANEN-26-KLT11".to_string(),
                milled_date: "20 Nov 2026".to_string(),
                grade: RiceGrade::Heritage,
                base_price_per_kg: MoneyIdr::new(21_000),
                stock_kg: 350,
                available_packs: vec![
                    WeightKg::try_new(2).unwrap(),
                    WeightKg::try_new(5).unwrap(),
                    WeightKg::try_new(10).unwrap(),
                    WeightKg::try_new(25).unwrap(),
                ],
                texture: RiceTextureProfile {
                    pulen_score: 8,
                    aroma_score: 8,
                    glycemic_label: "Sedang (IG 56)".to_string(),
                    water_ratio: 1.30,
                    soak_minutes: 15,
                    cook_minutes: 24,
                },
                tasting_notes: vec![
                    "Bersih Berkilau".to_string(),
                    "Gurih Halus".to_string(),
                    "Tahan Seharian".to_string(),
                ],
                image_url: "/images/rojolele-delanggu.jpg".to_string(),
                badge: Some("Pusaka Klaten".to_string()),
            },
            RiceProduct {
                id: "mentik-susu-organik".to_string(),
                slug: "mentik-susu-organik".to_string(),
                name: "Mentik Susu Organik".to_string(),
                tagline: "Putih susu ekstra pulen bebas pestisida untuk MPASI & sushi".to_string(),
                description: "Dibudidayakan dengan sistem SRI organik di lereng Magelang tanpa pupuk kimia sintetis. Bulirnya bulat putih susu dengan tingkat kepulenan tertinggi dan rasa manis alami.".to_string(),
                origin_region: "Magelang, Jawa Tengah".to_string(),
                harvest_batch: "PANEN-26-MGL02".to_string(),
                milled_date: "22 Nov 2026".to_string(),
                grade: RiceGrade::Organik,
                base_price_per_kg: MoneyIdr::new(26_500),
                stock_kg: 260,
                available_packs: vec![
                    WeightKg::try_new(2).unwrap(),
                    WeightKg::try_new(5).unwrap(),
                    WeightKg::try_new(10).unwrap(),
                ],
                texture: RiceTextureProfile {
                    pulen_score: 10,
                    aroma_score: 8,
                    glycemic_label: "Sedang (IG 55)".to_string(),
                    water_ratio: 1.15,
                    soak_minutes: 15,
                    cook_minutes: 20,
                },
                tasting_notes: vec![
                    "Ekstra Pulen".to_string(),
                    "Creamy Alami".to_string(),
                    "Bebas Residu".to_string(),
                ],
                image_url: "/images/mentik-susu.jpg".to_string(),
                badge: Some("100% Organik".to_string()),
            },
            RiceProduct {
                id: "beras-merah-cempo".to_string(),
                slug: "beras-merah-cempo".to_string(),
                name: "Beras Merah Cempo Sleman".to_string(),
                tagline: "Kaya serat dan antosianin dengan tekstur tetap empuk".to_string(),
                description: "Beras merah pecah kulit (whole grain) varietas Cempo Merah yang kaya vitamin B dan indeks glikemik rendah. Diproses higienis sehingga tetap lembut tanpa terasa keras saat dikunyah.".to_string(),
                origin_region: "Sleman, Yogyakarta".to_string(),
                harvest_batch: "PANEN-26-SLM09".to_string(),
                milled_date: "18 Nov 2026".to_string(),
                grade: RiceGrade::Organik,
                base_price_per_kg: MoneyIdr::new(25_000),
                stock_kg: 210,
                available_packs: vec![
                    WeightKg::try_new(2).unwrap(),
                    WeightKg::try_new(5).unwrap(),
                    WeightKg::try_new(10).unwrap(),
                ],
                texture: RiceTextureProfile {
                    pulen_score: 6,
                    aroma_score: 7,
                    glycemic_label: "Rendah (IG 48)".to_string(),
                    water_ratio: 1.75,
                    soak_minutes: 30,
                    cook_minutes: 35,
                },
                tasting_notes: vec![
                    "Nutty Panggang".to_string(),
                    "Tinggi Serat".to_string(),
                    "Indeks Glikemik Rendah".to_string(),
                ],
                image_url: "/images/beras-merah.jpg".to_string(),
                badge: Some("Low GI".to_string()),
            },
            RiceProduct {
                id: "beras-hitam-wulung".to_string(),
                slug: "beras-hitam-wulung".to_string(),
                name: "Beras Hitam Wulung".to_string(),
                tagline: "Superfood antosianin tinggi dengan aroma rempah bumi yang eksotis".to_string(),
                description: "Beras hitam langka hasil panen terbatas petani mitra Karanganyar. Berwarna ungu pekat saat dimasak dengan kadar antioksidan tertinggi dibanding varietas beras lainnya.".to_string(),
                origin_region: "Karanganyar, Solo".to_string(),
                harvest_batch: "PANEN-26-KRG01".to_string(),
                milled_date: "17 Nov 2026".to_string(),
                grade: RiceGrade::Heritage,
                base_price_per_kg: MoneyIdr::new(32_000),
                stock_kg: 120,
                available_packs: vec![
                    WeightKg::try_new(2).unwrap(),
                    WeightKg::try_new(5).unwrap(),
                ],
                texture: RiceTextureProfile {
                    pulen_score: 5,
                    aroma_score: 9,
                    glycemic_label: "Sangat Rendah (IG 42)".to_string(),
                    water_ratio: 1.85,
                    soak_minutes: 40,
                    cook_minutes: 40,
                },
                tasting_notes: vec![
                    "Kacang Sangrai".to_string(),
                    "Antioksidan Tinggi".to_string(),
                    "Panen Terbatas".to_string(),
                ],
                image_url: "/images/beras-hitam.jpg".to_string(),
                badge: Some("Limited Batch".to_string()),
            },
            RiceProduct {
                id: "setra-ramos-premium".to_string(),
                slug: "setra-ramos-premium".to_string(),
                name: "Setra Ramos Harian".to_string(),
                tagline: "Butir panjang bersih untuk nasi goreng, uduk, dan hidangan harian".to_string(),
                description: "Beras putih serbaguna dari Karawang yang melalui 3x pemolesan optik (color sorter) tanpa pemutih dan tanpa pewangi buatan. Pulen sedang dan tetap terpisah rapi saat diolah.".to_string(),
                origin_region: "Karawang, Jawa Barat".to_string(),
                harvest_batch: "PANEN-26-KRW18".to_string(),
                milled_date: "23 Nov 2026".to_string(),
                grade: RiceGrade::Premium,
                base_price_per_kg: MoneyIdr::new(17_500),
                stock_kg: 820,
                available_packs: vec![
                    WeightKg::try_new(5).unwrap(),
                    WeightKg::try_new(10).unwrap(),
                    WeightKg::try_new(25).unwrap(),
                ],
                texture: RiceTextureProfile {
                    pulen_score: 7,
                    aroma_score: 6,
                    glycemic_label: "Sedang (IG 60)".to_string(),
                    water_ratio: 1.35,
                    soak_minutes: 5,
                    cook_minutes: 20,
                },
                tasting_notes: vec![
                    "Pulen Seimbang".to_string(),
                    "Bulir Utuh 98%".to_string(),
                    "Favorit Dapur".to_string(),
                ],
                image_url: "/images/hero-rice.jpg".to_string(),
                badge: Some("Hemat Keluarga".to_string()),
            },
        ];

        Self {
            store: RwLock::new(products),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ProductRepository for InMemoryProductRepository {
    async fn list_all(&self) -> Vec<RiceProduct> {
        self.store.read().unwrap().clone()
    }

    async fn find_by_id(&self, id: &str) -> Option<RiceProduct> {
        self.store
            .read()
            .unwrap()
            .iter()
            .find(|p| p.id == id || p.slug == id)
            .cloned()
    }

    async fn save(&self, product: RiceProduct) -> Result<(), DomainError> {
        let mut guard = self.store.write().unwrap();
        if let Some(existing) = guard.iter_mut().find(|p| p.id == product.id) {
            *existing = product;
        } else {
            guard.push(product);
        }
        Ok(())
    }
}

/// Thread-safe, runtime-agnostic in-memory order repository.
pub struct InMemoryOrderRepository {
    orders: RwLock<HashMap<String, Order>>,
}

impl InMemoryOrderRepository {
    pub fn new() -> Self {
        Self {
            orders: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl OrderRepository for InMemoryOrderRepository {
    async fn list_all(&self) -> Vec<Order> {
        let mut list: Vec<Order> = self.orders.read().unwrap().values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    async fn find_by_id(&self, id: &OrderId) -> Option<Order> {
        self.orders.read().unwrap().get(&id.0).cloned()
    }

    async fn save(&self, order: Order) -> Result<(), DomainError> {
        self.orders.write().unwrap().insert(order.id.0.clone(), order);
        Ok(())
    }
}

/// In-memory event collector useful for observability and TDD verification.
pub struct InMemoryEventPublisher {
    events: RwLock<Vec<DomainEvent>>,
}

impl InMemoryEventPublisher {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
        }
    }

    pub fn recorded_events(&self) -> Vec<DomainEvent> {
        self.events.read().unwrap().clone()
    }
}

impl Default for InMemoryEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl EventPublisher for InMemoryEventPublisher {
    async fn publish(&self, event: DomainEvent) {
        self.events.write().unwrap().push(event);
    }
}
