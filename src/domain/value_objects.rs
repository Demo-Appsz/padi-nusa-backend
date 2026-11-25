use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Domain-level invariant errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("Ukuran kemasan tidak valid: {0} kg. Pilihan tersedia: 2, 5, 10, atau 25 kg")]
    InvalidPackWeight(u16),

    #[error("Jumlah item harus minimal 1")]
    InvalidQuantity,

    #[error("Jumlah porsi memasak harus antara 1 hingga 50")]
    InvalidPortions,

    #[error("Produk '{0}' tidak mendukung kemasan {1} kg")]
    UnsupportedPackForProduct(String, u16),

    #[error("Stok produk '{product}' tidak mencukupi (tersedia {available_kg} kg, diminta {requested_kg} kg)")]
    InsufficientStock {
        product: String,
        available_kg: u32,
        requested_kg: u32,
    },

    #[error("Kode promo '{0}' tidak ditemukan atau sudah kedaluwarsa")]
    InvalidPromoCode(String),

    #[error("Keranjang belanja tidak boleh kosong")]
    EmptyCart,

    #[error("Data pelanggan tidak lengkap: {0}")]
    InvalidCustomerInfo(String),

    #[error("Transisi status pesanan tidak diizinkan dari {from} ke {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("Produk '{0}' tidak ditemukan")]
    ProductNotFound(String),

    #[error("Pesanan '{0}' tidak ditemukan")]
    OrderNotFound(String),
}

/// Monetary value in Indonesian Rupiah (IDR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MoneyIdr(pub u64);

impl MoneyIdr {
    pub const ZERO: MoneyIdr = MoneyIdr(0);

    pub fn new(amount: u64) -> Self {
        Self(amount)
    }

    pub fn amount(&self) -> u64 {
        self.0
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub fn multiply(self, factor: u64) -> Self {
        Self(self.0.saturating_mul(factor))
    }

    /// Applies a percentage discount in basis points (e.g., 1000 bps = 10%).
    pub fn discount_by_bps(self, bps: u64) -> Self {
        let clamped = bps.min(10_000);
        let discount = (self.0.saturating_mul(clamped)) / 10_000;
        Self(discount)
    }
}

impl fmt::Display for MoneyIdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rp {}", self.0)
    }
}

/// Validated rice pack weight in kilograms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeightKg(u16);

impl WeightKg {
    pub const ALLOWED: [u16; 4] = [2, 5, 10, 25];

    pub fn try_new(kg: u16) -> Result<Self, DomainError> {
        if Self::ALLOWED.contains(&kg) {
            Ok(Self(kg))
        } else {
            Err(DomainError::InvalidPackWeight(kg))
        }
    }

    pub fn kg(&self) -> u16 {
        self.0
    }

    /// Bulk pack discount in basis points (100 bps = 1%).
    pub fn bulk_discount_bps(&self) -> u64 {
        match self.0 {
            2 => 0,
            5 => 300,   // 3%
            10 => 600,  // 6%
            25 => 1200, // 12%
            _ => 0,
        }
    }
}

/// Classification of rice cultivars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiceGrade {
    Artisanal,
    Organik,
    Heritage,
    Premium,
}

/// Culinary texture and cooking specification for a rice variety.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiceTextureProfile {
    pub pulen_score: u8,
    pub aroma_score: u8,
    pub glycemic_label: String,
    pub water_ratio: f32,
    pub soak_minutes: u16,
    pub cook_minutes: u16,
}

/// Validated promotional code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromoCode {
    PanenBaru,
    Langganan15,
    OngkirGratis,
}

impl PromoCode {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_uppercase().as_str() {
            "PANENBARU" => Ok(Self::PanenBaru),
            "LANGGANAN15" => Ok(Self::Langganan15),
            "ONGKIRGRATIS" => Ok(Self::OngkirGratis),
            _ => Err(DomainError::InvalidPromoCode(raw.trim().to_string())),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::PanenBaru => "PANENBARU",
            Self::Langganan15 => "LANGGANAN15",
            Self::OngkirGratis => "ONGKIRGRATIS",
        }
    }

    pub fn subtotal_discount_bps(&self) -> u64 {
        match self {
            Self::PanenBaru => 1000,   // 10%
            Self::Langganan15 => 1500, // 15%
            Self::OngkirGratis => 0,
        }
    }

    pub fn waives_shipping(&self) -> bool {
        matches!(self, Self::OngkirGratis)
    }
}

/// Lifecycle state of a customer order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    MenungguKonfirmasi,
    DiprosesPenggilingan,
    Dikirim,
    Selesai,
    Dibatalkan,
}

impl OrderStatus {
    pub fn transition_to(self, next: Self) -> Result<Self, DomainError> {
        let valid = matches!(
            (self, next),
            (Self::MenungguKonfirmasi, Self::DiprosesPenggilingan)
                | (Self::MenungguKonfirmasi, Self::Dibatalkan)
                | (Self::DiprosesPenggilingan, Self::Dikirim)
                | (Self::DiprosesPenggilingan, Self::Dibatalkan)
                | (Self::Dikirim, Self::Selesai)
        );

        if valid {
            Ok(next)
        } else {
            Err(DomainError::InvalidStatusTransition {
                from: format!("{self:?}"),
                to: format!("{next:?}"),
            })
        }
    }
}

/// Strongly-typed Order identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub String);
