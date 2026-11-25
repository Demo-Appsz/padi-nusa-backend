use crate::domain::entities::{OrderItem, RiceProduct};
use crate::domain::value_objects::{DomainError, MoneyIdr, PromoCode, RiceTextureProfile, WeightKg};
use serde::{Deserialize, Serialize};

/// Result of quoting a cart or order before placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceQuote {
    pub items: Vec<OrderItem>,
    pub subtotal: MoneyIdr,
    pub subscription_discount: MoneyIdr,
    pub promo_discount: MoneyIdr,
    pub total_discount: MoneyIdr,
    pub shipping_fee: MoneyIdr,
    pub free_shipping_threshold: MoneyIdr,
    pub total: MoneyIdr,
    pub applied_promo: Option<String>,
}

/// Pure domain service handling pricing rules, pack tiers, subscriptions, promos, and shipping.
pub struct PricingService;

impl PricingService {
    pub const SUBSCRIPTION_DISCOUNT_BPS: u64 = 1000; // 10%
    pub const JABODETABEK_SHIPPING_IDR: u64 = 18_000;
    pub const OUTSIDE_SHIPPING_IDR: u64 = 32_000;
    pub const JABODETABEK_FREE_THRESHOLD_IDR: u64 = 150_000;
    pub const OUTSIDE_FREE_THRESHOLD_IDR: u64 = 350_000;

    /// Calculates unit pack price after bulk pack discount.
    pub fn unit_pack_price(product: &RiceProduct, pack: WeightKg) -> Result<MoneyIdr, DomainError> {
        if !product.supports_pack(pack) {
            return Err(DomainError::UnsupportedPackForProduct(
                product.name.clone(),
                pack.kg(),
            ));
        }
        let gross = product.base_price_per_kg.multiply(u64::from(pack.kg()));
        let discount = gross.discount_by_bps(pack.bulk_discount_bps());
        Ok(gross.saturating_sub(discount))
    }

    pub fn is_jabodetabek(city: &str) -> bool {
        let normalized = city.trim().to_lowercase();
        [
            "jakarta",
            "bogor",
            "depok",
            "tangerang",
            "bekasi",
            "jabodetabek",
        ]
        .iter()
        .any(|k| normalized.contains(k))
    }

    pub fn quote_order(
        lines: &[(&RiceProduct, WeightKg, u16)],
        city: &str,
        is_subscription: bool,
        promo_raw: Option<&str>,
    ) -> Result<PriceQuote, DomainError> {
        if lines.is_empty() {
            return Err(DomainError::EmptyCart);
        }

        let mut items = Vec::with_capacity(lines.len());
        let mut subtotal = MoneyIdr::ZERO;

        for &(product, pack, qty) in lines {
            if qty == 0 {
                return Err(DomainError::InvalidQuantity);
            }
            let unit_price = Self::unit_pack_price(product, pack)?;
            let line_total = unit_price.multiply(u64::from(qty));
            subtotal = subtotal.saturating_add(line_total);
            items.push(OrderItem {
                product_id: product.id.clone(),
                product_name: product.name.clone(),
                pack_kg: pack,
                quantity: qty,
                unit_price,
                line_total,
            });
        }

        let subscription_discount = if is_subscription {
            subtotal.discount_by_bps(Self::SUBSCRIPTION_DISCOUNT_BPS)
        } else {
            MoneyIdr::ZERO
        };

        let after_sub = subtotal.saturating_sub(subscription_discount);

        let parsed_promo = match promo_raw {
            Some(code) if !code.trim().is_empty() => Some(PromoCode::parse(code)?),
            _ => None,
        };

        let promo_discount = parsed_promo
            .as_ref()
            .map(|p| after_sub.discount_by_bps(p.subtotal_discount_bps()))
            .unwrap_or(MoneyIdr::ZERO);

        let total_discount = subscription_discount.saturating_add(promo_discount);
        let net_merchandise = subtotal.saturating_sub(total_discount);

        let jabodetabek = Self::is_jabodetabek(city);
        let free_threshold = MoneyIdr::new(if jabodetabek {
            Self::JABODETABEK_FREE_THRESHOLD_IDR
        } else {
            Self::OUTSIDE_FREE_THRESHOLD_IDR
        });

        let waives_shipping = parsed_promo
            .as_ref()
            .map(|p| p.waives_shipping())
            .unwrap_or(false);

        let shipping_fee = if waives_shipping || net_merchandise >= free_threshold {
            MoneyIdr::ZERO
        } else if jabodetabek {
            MoneyIdr::new(Self::JABODETABEK_SHIPPING_IDR)
        } else {
            MoneyIdr::new(Self::OUTSIDE_SHIPPING_IDR)
        };

        let total = net_merchandise.saturating_add(shipping_fee);

        Ok(PriceQuote {
            items,
            subtotal,
            subscription_discount,
            promo_discount,
            total_discount,
            shipping_fee,
            free_shipping_threshold: free_threshold,
            total,
            applied_promo: parsed_promo.map(|p| p.code().to_string()),
        })
    }
}

/// Output of the interactive rice-to-water cooking ratio calculator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CookingGuideResult {
    pub portions: u16,
    pub rice_grams: u32,
    pub water_ml: u32,
    pub rice_cups: f32,
    pub water_cups: f32,
    pub soak_minutes: u16,
    pub cook_minutes: u16,
    pub rest_minutes: u16,
    pub chef_tip: String,
}

/// Pure domain service computing exact water and timing ratios per rice cultivar.
pub struct CookingCalculatorService;

impl CookingCalculatorService {
    pub const GRAMS_PER_PORTION: u32 = 75;
    pub const GRAMS_PER_CUP: f32 = 180.0;
    pub const ML_PER_CUP: f32 = 240.0;

    pub fn calculate(
        texture: &RiceTextureProfile,
        portions: u16,
    ) -> Result<CookingGuideResult, DomainError> {
        if portions == 0 || portions > 50 {
            return Err(DomainError::InvalidPortions);
        }

        let rice_grams = u32::from(portions) * Self::GRAMS_PER_PORTION;
        let rice_cups = ((rice_grams as f32 / Self::GRAMS_PER_CUP) * 10.0).round() / 10.0;
        let water_cups = ((rice_cups * texture.water_ratio) * 10.0).round() / 10.0;
        let water_ml = ((rice_grams as f32 / Self::GRAMS_PER_CUP)
            * texture.water_ratio
            * Self::ML_PER_CUP)
            .round() as u32;

        let chef_tip = if texture.soak_minutes >= 25 {
            format!(
                "Rendam {} menit sebelum dimasak agar kulit ari melunak dan nutrisi terjaga optimal.",
                texture.soak_minutes
            )
        } else if texture.aroma_score >= 9 {
            "Bilas maksimal 2 kali dengan air dingin agar aroma alami pandan dan minyak esensial bulir tetap utuh."
                .to_string()
        } else {
            "Setelah matang, diamkan 10 menit dalam mode warm sebelum diaduk perlahan dengan centong kayu."
                .to_string()
        };

        Ok(CookingGuideResult {
            portions,
            rice_grams,
            water_ml,
            rice_cups,
            water_cups,
            soak_minutes: texture.soak_minutes,
            cook_minutes: texture.cook_minutes,
            rest_minutes: 10,
            chef_tip,
        })
    }
}
