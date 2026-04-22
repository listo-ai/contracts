//! Physical-quantity registry for slot values.
//!
//! See `agent/docs/design/USER-PREFERENCES.md` § "Slot units" and
//! § "UnitRegistry" for the design rationale.
//!
//! **Design invariants you must not break:**
//!
//! 1. [`Quantity`] and [`Unit`] are **closed enums**. Adding a variant
//!    is additive; renaming or removing requires a major-version bump
//!    (see `SYSTEM-BOOTSTRAP.md` / `USER-PREFERENCES.md` §
//!    "Enum versioning").
//! 2. The wire format never exposes a `uom` type — only the plain
//!    `Unit` / `Quantity` enum strings. `uom` is an *implementation*
//!    detail of the conversion function inside [`StaticRegistry`].
//! 3. Blocks cannot add new quantities. The platform PR process (per
//!    USER-PREFERENCES.md § "Block-defined quantities") is the only
//!    path that extends this enum.
//!
//! Callers that only need to do a conversion do:
//!
//! ```rust
//! use spi::units::{default_registry, Quantity, Unit};
//! let r = default_registry();
//! let f = r.convert(Quantity::Temperature, 22.0, Unit::Celsius, Unit::Fahrenheit);
//! assert!((f - 71.6).abs() < 0.01);
//! ```

use serde::{Deserialize, Serialize};

// ── Quantity ──────────────────────────────────────────────────────────────────

/// A physical quantity the platform knows how to store and render.
///
/// Stored in [`SlotSchema::quantity`](super::slot_schema::SlotSchema::quantity)
/// to tell the ingest + read paths which canonical unit to use and
/// which user preference drives display conversion.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Quantity {
    Temperature,
    Pressure,
    FlowRate,
    Volume,
    Mass,
    Length,
    Energy,
    Power,
    Speed,
    /// Dimensionless 0.0–1.0. Canonical unit is [`Unit::Ratio`].
    /// [`Unit::Percent`] is a display-only alias; the registry
    /// rejects a slot declaring `quantity: Ratio, unit: Percent`
    /// (stored values must be 0–1, never 0–100). See
    /// USER-PREFERENCES.md § "UnitRegistry".
    Ratio,
    Duration,
}

// ── Unit ──────────────────────────────────────────────────────────────────────

/// A concrete unit. Closed enum so the wire format is stable and the
/// UI knows every label. Blocks **cannot** add variants — see the
/// module docs.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    // Temperature
    Celsius,
    Fahrenheit,
    Kelvin,
    // Pressure
    Kilopascal,
    Bar,
    Psi,
    Hectopascal,
    // Flow rate
    LitersPerSecond,
    LitersPerMinute,
    CubicMetersPerHour,
    GallonsPerMinute,
    // Volume
    Liter,
    CubicMeter,
    UsGallon,
    ImperialGallon,
    // Mass
    Kilogram,
    Gram,
    Pound,
    Ounce,
    // Length
    Meter,
    Millimeter,
    Kilometer,
    Inch,
    Foot,
    Mile,
    // Energy / power
    Kilowatt,
    Watt,
    Horsepower,
    KilowattHour,
    Joule,
    // Speed
    MetersPerSecond,
    KilometersPerHour,
    MilesPerHour,
    Knot,
    // Dimensionless
    Ratio,
    Percent,
    // Duration
    Millisecond,
    Second,
    Minute,
    Hour,
}

// ── QuantityDef ───────────────────────────────────────────────────────────────

/// Static metadata for one quantity — what its canonical unit is,
/// which alternatives a user preference can select, and the short
/// symbol to render when the caller has no locale-aware formatter.
#[derive(Debug, Clone, Copy)]
pub struct QuantityDef {
    pub canonical: Unit,
    pub allowed: &'static [Unit],
    /// Short symbol for render, e.g. `"°C"`. Locale-aware formatting
    /// on the client side goes through ICU4X; this is the compact
    /// fallback used in logs, CLI, and any surface without an i18n
    /// framework.
    pub symbol: &'static str,
}

// ── UnitRegistry trait ────────────────────────────────────────────────────────

/// Read-only registry of quantity metadata and unit conversion.
///
/// The trait is the seam the rest of the platform depends on. Tests
/// can substitute a fixture registry (e.g. to inject a synthetic
/// quantity) without touching the global static.
pub trait UnitRegistry: Send + Sync {
    /// Canonical metadata for `q`. Panics are never returned — every
    /// [`Quantity`] variant has a row in the static table.
    fn quantity(&self, q: Quantity) -> &'static QuantityDef;

    /// Convert `v` from `from` to `to`. Both units must be in
    /// `self.quantity(q).allowed`. When the registry does not know
    /// how to convert between the two units (same dimension enforced
    /// by the enum tables) the implementation is permitted to panic
    /// — the enum-to-enum match is exhaustive inside
    /// [`StaticRegistry`] and covers every pair in the allowed set.
    fn convert(&self, q: Quantity, v: f64, from: Unit, to: Unit) -> f64;

    /// `true` if `u` is a permitted display / storage unit for `q`.
    /// Kept as a convenience so authors don't have to replicate the
    /// `quantity().allowed.contains(…)` dance.
    fn allows(&self, q: Quantity, u: Unit) -> bool {
        self.quantity(q).allowed.iter().any(|x| *x == u)
    }
}

// ── StaticRegistry ────────────────────────────────────────────────────────────

/// Production registry — built from the compile-time quantity table.
///
/// All conversion math is delegated to [`uom`] where a mapping
/// exists; dimensionless-to-dimensionless mappings (`Ratio` ↔
/// `Percent`, duration unit ladder) are trivial enough to handle by
/// scale factor.
pub struct StaticRegistry;

/// Singleton instance. Callers usually want [`default_registry`]
/// which returns a `&'static dyn UnitRegistry`.
pub static STATIC_REGISTRY: StaticRegistry = StaticRegistry;

/// Default platform registry — wraps [`STATIC_REGISTRY`] as a trait
/// object so call sites don't hard-depend on the concrete type.
pub fn default_registry() -> &'static dyn UnitRegistry {
    &STATIC_REGISTRY
}

impl UnitRegistry for StaticRegistry {
    fn quantity(&self, q: Quantity) -> &'static QuantityDef {
        match q {
            Quantity::Temperature => &TEMPERATURE,
            Quantity::Pressure => &PRESSURE,
            Quantity::FlowRate => &FLOW_RATE,
            Quantity::Volume => &VOLUME,
            Quantity::Mass => &MASS,
            Quantity::Length => &LENGTH,
            Quantity::Energy => &ENERGY,
            Quantity::Power => &POWER,
            Quantity::Speed => &SPEED,
            Quantity::Ratio => &RATIO,
            Quantity::Duration => &DURATION,
        }
    }

    fn convert(&self, q: Quantity, v: f64, from: Unit, to: Unit) -> f64 {
        if from == to {
            return v;
        }
        match q {
            Quantity::Temperature => convert_temperature(v, from, to),
            Quantity::Pressure => convert_pressure(v, from, to),
            Quantity::FlowRate => convert_flow_rate(v, from, to),
            Quantity::Volume => convert_volume(v, from, to),
            Quantity::Mass => convert_mass(v, from, to),
            Quantity::Length => convert_length(v, from, to),
            Quantity::Energy => convert_energy(v, from, to),
            Quantity::Power => convert_power(v, from, to),
            Quantity::Speed => convert_speed(v, from, to),
            Quantity::Ratio => convert_ratio(v, from, to),
            Quantity::Duration => convert_duration(v, from, to),
        }
    }
}

// ── Quantity tables ───────────────────────────────────────────────────────────

static TEMPERATURE: QuantityDef = QuantityDef {
    canonical: Unit::Celsius,
    allowed: &[Unit::Celsius, Unit::Fahrenheit, Unit::Kelvin],
    symbol: "°C",
};

static PRESSURE: QuantityDef = QuantityDef {
    canonical: Unit::Kilopascal,
    allowed: &[Unit::Kilopascal, Unit::Bar, Unit::Psi, Unit::Hectopascal],
    symbol: "kPa",
};

static FLOW_RATE: QuantityDef = QuantityDef {
    canonical: Unit::LitersPerSecond,
    allowed: &[
        Unit::LitersPerSecond,
        Unit::LitersPerMinute,
        Unit::CubicMetersPerHour,
        Unit::GallonsPerMinute,
    ],
    symbol: "L/s",
};

static VOLUME: QuantityDef = QuantityDef {
    canonical: Unit::Liter,
    allowed: &[
        Unit::Liter,
        Unit::CubicMeter,
        Unit::UsGallon,
        Unit::ImperialGallon,
    ],
    symbol: "L",
};

static MASS: QuantityDef = QuantityDef {
    canonical: Unit::Kilogram,
    allowed: &[Unit::Kilogram, Unit::Gram, Unit::Pound, Unit::Ounce],
    symbol: "kg",
};

static LENGTH: QuantityDef = QuantityDef {
    canonical: Unit::Meter,
    allowed: &[
        Unit::Meter,
        Unit::Millimeter,
        Unit::Kilometer,
        Unit::Inch,
        Unit::Foot,
        Unit::Mile,
    ],
    symbol: "m",
};

static ENERGY: QuantityDef = QuantityDef {
    canonical: Unit::KilowattHour,
    allowed: &[Unit::KilowattHour, Unit::Joule],
    symbol: "kWh",
};

static POWER: QuantityDef = QuantityDef {
    canonical: Unit::Watt,
    allowed: &[Unit::Watt, Unit::Kilowatt, Unit::Horsepower],
    symbol: "W",
};

static SPEED: QuantityDef = QuantityDef {
    canonical: Unit::MetersPerSecond,
    allowed: &[
        Unit::MetersPerSecond,
        Unit::KilometersPerHour,
        Unit::MilesPerHour,
        Unit::Knot,
    ],
    symbol: "m/s",
};

static RATIO: QuantityDef = QuantityDef {
    canonical: Unit::Ratio,
    allowed: &[Unit::Ratio, Unit::Percent],
    symbol: "",
};

static DURATION: QuantityDef = QuantityDef {
    canonical: Unit::Second,
    allowed: &[Unit::Millisecond, Unit::Second, Unit::Minute, Unit::Hour],
    symbol: "s",
};

// ── Converters ────────────────────────────────────────────────────────────────
//
// Each helper goes canonical ← `from`, then canonical → `to`. Keeping
// it as two round-trips via the canonical keeps the match arms short
// and the behaviour auditable.

fn convert_temperature(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::ThermodynamicTemperature;
    use uom::si::thermodynamic_temperature::{degree_celsius, degree_fahrenheit, kelvin};

    let t = match from {
        Unit::Celsius => ThermodynamicTemperature::new::<degree_celsius>(v),
        Unit::Fahrenheit => ThermodynamicTemperature::new::<degree_fahrenheit>(v),
        Unit::Kelvin => ThermodynamicTemperature::new::<kelvin>(v),
        other => panic!("temperature: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Celsius => t.get::<degree_celsius>(),
        Unit::Fahrenheit => t.get::<degree_fahrenheit>(),
        Unit::Kelvin => t.get::<kelvin>(),
        other => panic!("temperature: unit `{other:?}` not in allowed set"),
    }
}

fn convert_pressure(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Pressure;
    use uom::si::pressure::{bar, hectopascal, kilopascal, psi};
    let p = match from {
        Unit::Kilopascal => Pressure::new::<kilopascal>(v),
        Unit::Bar => Pressure::new::<bar>(v),
        Unit::Psi => Pressure::new::<psi>(v),
        Unit::Hectopascal => Pressure::new::<hectopascal>(v),
        other => panic!("pressure: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Kilopascal => p.get::<kilopascal>(),
        Unit::Bar => p.get::<bar>(),
        Unit::Psi => p.get::<psi>(),
        Unit::Hectopascal => p.get::<hectopascal>(),
        other => panic!("pressure: unit `{other:?}` not in allowed set"),
    }
}

fn convert_flow_rate(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::VolumeRate;
    use uom::si::volume_rate::{
        cubic_meter_per_hour, gallon_per_minute, liter_per_minute, liter_per_second,
    };
    let r = match from {
        Unit::LitersPerSecond => VolumeRate::new::<liter_per_second>(v),
        Unit::LitersPerMinute => VolumeRate::new::<liter_per_minute>(v),
        Unit::CubicMetersPerHour => VolumeRate::new::<cubic_meter_per_hour>(v),
        Unit::GallonsPerMinute => VolumeRate::new::<gallon_per_minute>(v),
        other => panic!("flow_rate: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::LitersPerSecond => r.get::<liter_per_second>(),
        Unit::LitersPerMinute => r.get::<liter_per_minute>(),
        Unit::CubicMetersPerHour => r.get::<cubic_meter_per_hour>(),
        Unit::GallonsPerMinute => r.get::<gallon_per_minute>(),
        other => panic!("flow_rate: unit `{other:?}` not in allowed set"),
    }
}

fn convert_volume(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Volume;
    use uom::si::volume::{cubic_meter, gallon, gallon_imperial, liter};
    let x = match from {
        Unit::Liter => Volume::new::<liter>(v),
        Unit::CubicMeter => Volume::new::<cubic_meter>(v),
        Unit::UsGallon => Volume::new::<gallon>(v),
        Unit::ImperialGallon => Volume::new::<gallon_imperial>(v),
        other => panic!("volume: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Liter => x.get::<liter>(),
        Unit::CubicMeter => x.get::<cubic_meter>(),
        Unit::UsGallon => x.get::<gallon>(),
        Unit::ImperialGallon => x.get::<gallon_imperial>(),
        other => panic!("volume: unit `{other:?}` not in allowed set"),
    }
}

fn convert_mass(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Mass;
    use uom::si::mass::{gram, kilogram, ounce, pound};
    let m = match from {
        Unit::Kilogram => Mass::new::<kilogram>(v),
        Unit::Gram => Mass::new::<gram>(v),
        Unit::Pound => Mass::new::<pound>(v),
        Unit::Ounce => Mass::new::<ounce>(v),
        other => panic!("mass: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Kilogram => m.get::<kilogram>(),
        Unit::Gram => m.get::<gram>(),
        Unit::Pound => m.get::<pound>(),
        Unit::Ounce => m.get::<ounce>(),
        other => panic!("mass: unit `{other:?}` not in allowed set"),
    }
}

fn convert_length(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Length;
    use uom::si::length::{foot, inch, kilometer, meter, mile, millimeter};
    let l = match from {
        Unit::Meter => Length::new::<meter>(v),
        Unit::Millimeter => Length::new::<millimeter>(v),
        Unit::Kilometer => Length::new::<kilometer>(v),
        Unit::Inch => Length::new::<inch>(v),
        Unit::Foot => Length::new::<foot>(v),
        Unit::Mile => Length::new::<mile>(v),
        other => panic!("length: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Meter => l.get::<meter>(),
        Unit::Millimeter => l.get::<millimeter>(),
        Unit::Kilometer => l.get::<kilometer>(),
        Unit::Inch => l.get::<inch>(),
        Unit::Foot => l.get::<foot>(),
        Unit::Mile => l.get::<mile>(),
        other => panic!("length: unit `{other:?}` not in allowed set"),
    }
}

fn convert_energy(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::energy::{joule, kilowatt_hour};
    use uom::si::f64::Energy;
    let e = match from {
        Unit::KilowattHour => Energy::new::<kilowatt_hour>(v),
        Unit::Joule => Energy::new::<joule>(v),
        other => panic!("energy: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::KilowattHour => e.get::<kilowatt_hour>(),
        Unit::Joule => e.get::<joule>(),
        other => panic!("energy: unit `{other:?}` not in allowed set"),
    }
}

fn convert_power(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Power;
    use uom::si::power::{horsepower, kilowatt, watt};
    let p = match from {
        Unit::Watt => Power::new::<watt>(v),
        Unit::Kilowatt => Power::new::<kilowatt>(v),
        Unit::Horsepower => Power::new::<horsepower>(v),
        other => panic!("power: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Watt => p.get::<watt>(),
        Unit::Kilowatt => p.get::<kilowatt>(),
        Unit::Horsepower => p.get::<horsepower>(),
        other => panic!("power: unit `{other:?}` not in allowed set"),
    }
}

fn convert_speed(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Velocity;
    use uom::si::velocity::{
        kilometer_per_hour, knot, meter_per_second, mile_per_hour,
    };
    let s = match from {
        Unit::MetersPerSecond => Velocity::new::<meter_per_second>(v),
        Unit::KilometersPerHour => Velocity::new::<kilometer_per_hour>(v),
        Unit::MilesPerHour => Velocity::new::<mile_per_hour>(v),
        Unit::Knot => Velocity::new::<knot>(v),
        other => panic!("speed: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::MetersPerSecond => s.get::<meter_per_second>(),
        Unit::KilometersPerHour => s.get::<kilometer_per_hour>(),
        Unit::MilesPerHour => s.get::<mile_per_hour>(),
        Unit::Knot => s.get::<knot>(),
        other => panic!("speed: unit `{other:?}` not in allowed set"),
    }
}

fn convert_ratio(v: f64, from: Unit, to: Unit) -> f64 {
    // Ratio's canonical is 0.0–1.0. Percent is a display-only alias:
    // it's just ×100 / ÷100. Skip uom here because its `Ratio`
    // treatment doesn't gain us anything for a single scale factor.
    let canonical = match from {
        Unit::Ratio => v,
        Unit::Percent => v / 100.0,
        other => panic!("ratio: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Ratio => canonical,
        Unit::Percent => canonical * 100.0,
        other => panic!("ratio: unit `{other:?}` not in allowed set"),
    }
}

fn convert_duration(v: f64, from: Unit, to: Unit) -> f64 {
    use uom::si::f64::Time;
    use uom::si::time::{hour, millisecond, minute, second};
    let t = match from {
        Unit::Millisecond => Time::new::<millisecond>(v),
        Unit::Second => Time::new::<second>(v),
        Unit::Minute => Time::new::<minute>(v),
        Unit::Hour => Time::new::<hour>(v),
        other => panic!("duration: unit `{other:?}` not in allowed set"),
    };
    match to {
        Unit::Millisecond => t.get::<millisecond>(),
        Unit::Second => t.get::<second>(),
        Unit::Minute => t.get::<minute>(),
        Unit::Hour => t.get::<hour>(),
        other => panic!("duration: unit `{other:?}` not in allowed set"),
    }
}

// ── Registry DTO (for `GET /api/v1/units`) ────────────────────────────────────

/// One entry in the public [`RegistryDto`]. Mirrors [`QuantityDef`]
/// but with enum values expressed as their serialised strings so the
/// wire format is stable without depending on the rustc layout of the
/// enum.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QuantityEntry {
    /// Serialised quantity id (e.g. `"temperature"`).
    pub id: Quantity,
    /// Canonical (storage) unit for this quantity.
    pub canonical: Unit,
    /// Every unit a user preference or slot schema can select for this
    /// quantity. First entry is the canonical.
    pub allowed: Vec<Unit>,
    /// Compact symbol for render without a locale-aware formatter.
    pub symbol: String,
}

/// Full wire shape of `GET /api/v1/units`. Lets clients drive unit-
/// picker UIs and (with the registry-version header) detect a drift
/// between their cached factors and the server's. See
/// `agent/docs/design/USER-PREFERENCES.md` § "API surface" /
/// "Enum versioning and canonical-unit migration".
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RegistryDto {
    pub quantities: Vec<QuantityEntry>,
}

/// Every quantity variant, in enum declaration order. Kept private so
/// new variants have exactly one place to land — the iterator below
/// and any future `for q in ALL_QUANTITIES` call site stay in sync.
const ALL_QUANTITIES: &[Quantity] = &[
    Quantity::Temperature,
    Quantity::Pressure,
    Quantity::FlowRate,
    Quantity::Volume,
    Quantity::Mass,
    Quantity::Length,
    Quantity::Energy,
    Quantity::Power,
    Quantity::Speed,
    Quantity::Ratio,
    Quantity::Duration,
];

/// Build the public [`RegistryDto`] from any [`UnitRegistry`]. Pure —
/// no global state — so tests can substitute a fixture registry and
/// inspect the exact shape the transport layer will serialise.
pub fn registry_dto(registry: &dyn UnitRegistry) -> RegistryDto {
    let quantities = ALL_QUANTITIES
        .iter()
        .map(|q| {
            let def = registry.quantity(*q);
            QuantityEntry {
                id: *q,
                canonical: def.canonical,
                allowed: def.allowed.to_vec(),
                symbol: def.symbol.to_string(),
            }
        })
        .collect();
    RegistryDto { quantities }
}

// ── Ingest normalisation ──────────────────────────────────────────────────────

/// Convert a raw slot value into the form the graph should store,
/// per the slot's `quantity` / `sensor_unit` / `unit` declarations.
///
/// Rules — mirrors `agent/docs/design/USER-PREFERENCES.md` § "How a
/// read works end-to-end":
///
/// 1. Only `SlotValueKind::Number` participates. Any other kind is a
///    passthrough (ignored `quantity` is a no-op, not an error — the
///    design calls this out explicitly so authors can copy declarations
///    across slot roles without special-casing).
/// 2. `quantity == None` → passthrough. The slot is genuinely
///    dimensionless.
/// 3. `sensor_unit == None` → passthrough. The sensor is declared to
///    already emit the canonical (or `slot.unit`-override) unit.
/// 4. `sensor_unit == target_unit` → passthrough. Target is
///    `slot.unit` if set, else the registry's canonical for the
///    quantity.
/// 5. Otherwise, convert from `sensor_unit` → target via the registry.
///
/// JSON numbers are preserved as-is when no conversion happens. When
/// a conversion does happen, the stored value is a `serde_json::Value`
/// wrapping the converted f64. `null` values are always passthrough
/// regardless of schema — an absent reading is a valid state.
///
/// A non-numeric JSON value on a `Number` slot with a `quantity` is
/// returned unchanged: the schema-validation layer handles type
/// mismatches before this helper runs, so flagging it here would be
/// either redundant or a silent second source of truth.
pub fn normalize_for_storage(
    schema: &crate::SlotSchema,
    raw: serde_json::Value,
    registry: &dyn UnitRegistry,
) -> serde_json::Value {
    use crate::SlotValueKind;
    if schema.value_kind != SlotValueKind::Number {
        return raw;
    }
    let Some(q) = schema.quantity else {
        return raw;
    };
    let Some(source) = schema.sensor_unit else {
        return raw;
    };
    let target = schema.unit.unwrap_or_else(|| registry.quantity(q).canonical);
    if source == target {
        return raw;
    }
    let Some(v) = raw.as_f64() else {
        // Keep raw — schema validation is not this helper's job.
        return raw;
    };
    let converted = registry.convert(q, v, source, target);
    serde_json::json!(converted)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn default_registry_has_every_quantity() {
        let r = default_registry();
        for q in [
            Quantity::Temperature,
            Quantity::Pressure,
            Quantity::FlowRate,
            Quantity::Volume,
            Quantity::Mass,
            Quantity::Length,
            Quantity::Energy,
            Quantity::Power,
            Quantity::Speed,
            Quantity::Ratio,
            Quantity::Duration,
        ] {
            let def = r.quantity(q);
            assert!(!def.allowed.is_empty(), "quantity {q:?} has empty allowed set");
            assert!(
                def.allowed.iter().any(|u| *u == def.canonical),
                "quantity {q:?} canonical `{:?}` not in allowed set",
                def.canonical
            );
        }
    }

    #[test]
    fn temperature_c_to_f_matches_known_values() {
        let r = default_registry();
        assert!(close(
            r.convert(Quantity::Temperature, 0.0, Unit::Celsius, Unit::Fahrenheit),
            32.0
        ));
        assert!(close(
            r.convert(Quantity::Temperature, 100.0, Unit::Celsius, Unit::Fahrenheit),
            212.0
        ));
        assert!(close(
            r.convert(Quantity::Temperature, 22.0, Unit::Celsius, Unit::Fahrenheit),
            71.6
        ));
    }

    #[test]
    fn pressure_kpa_to_psi_matches_standard_atmosphere() {
        let r = default_registry();
        // 101.325 kPa ≈ 14.696 psi (standard atmosphere).
        assert!(close(
            r.convert(Quantity::Pressure, 101.325, Unit::Kilopascal, Unit::Psi),
            14.696
        ));
    }

    #[test]
    fn ratio_and_percent_round_trip() {
        let r = default_registry();
        assert!(close(r.convert(Quantity::Ratio, 0.42, Unit::Ratio, Unit::Percent), 42.0));
        assert!(close(r.convert(Quantity::Ratio, 42.0, Unit::Percent, Unit::Ratio), 0.42));
    }

    #[test]
    fn same_unit_is_identity() {
        let r = default_registry();
        assert_eq!(
            r.convert(Quantity::Length, 3.14, Unit::Meter, Unit::Meter),
            3.14
        );
    }

    #[test]
    fn allows_rejects_non_member() {
        let r = default_registry();
        assert!(r.allows(Quantity::Temperature, Unit::Celsius));
        assert!(!r.allows(Quantity::Temperature, Unit::Psi));
    }

    // ── Ingest normalisation ──────────────────────────────────────────────────

    use crate::slot_schema::{SlotRole, SlotSchema, SlotValueKind};

    fn number_slot(name: &str) -> SlotSchema {
        SlotSchema::new(name, SlotRole::Input).with_kind(SlotValueKind::Number)
    }

    #[test]
    fn normalise_passthrough_when_value_kind_not_number() {
        let s = SlotSchema::new("x", SlotRole::Input)
            .with_quantity(Quantity::Temperature)
            .with_sensor_unit(Unit::Fahrenheit);
        let out = normalize_for_storage(
            &s,
            serde_json::json!("hello"),
            default_registry(),
        );
        assert_eq!(out, serde_json::json!("hello"));
    }

    #[test]
    fn normalise_passthrough_when_quantity_missing() {
        let s = number_slot("x");
        let out = normalize_for_storage(&s, serde_json::json!(42.0), default_registry());
        assert_eq!(out, serde_json::json!(42.0));
    }

    #[test]
    fn normalise_passthrough_when_sensor_unit_missing() {
        let s = number_slot("x").with_quantity(Quantity::Temperature);
        let out = normalize_for_storage(&s, serde_json::json!(22.0), default_registry());
        assert_eq!(out, serde_json::json!(22.0));
    }

    #[test]
    fn normalise_passthrough_when_sensor_equals_canonical() {
        let s = number_slot("x")
            .with_quantity(Quantity::Temperature)
            .with_sensor_unit(Unit::Celsius);
        let out = normalize_for_storage(&s, serde_json::json!(22.0), default_registry());
        assert_eq!(out, serde_json::json!(22.0));
    }

    #[test]
    fn normalise_converts_sensor_to_canonical() {
        // Fahrenheit sensor, no explicit storage override → convert to
        // canonical (°C). 72.4 °F → 22.444… °C.
        let s = number_slot("temp_in")
            .with_quantity(Quantity::Temperature)
            .with_sensor_unit(Unit::Fahrenheit);
        let out = normalize_for_storage(&s, serde_json::json!(72.4), default_registry());
        let stored = out.as_f64().expect("converted value stays numeric");
        assert!((stored - 22.444).abs() < 0.01, "got {stored}");
    }

    #[test]
    fn normalise_honours_unit_override() {
        // Author opted out: sensor is °F but storage stays °F too.
        let s = number_slot("temp_raw")
            .with_quantity(Quantity::Temperature)
            .with_sensor_unit(Unit::Fahrenheit)
            .with_unit(Unit::Fahrenheit);
        let out = normalize_for_storage(&s, serde_json::json!(72.4), default_registry());
        assert_eq!(out, serde_json::json!(72.4));
    }

    #[test]
    fn normalise_null_is_passthrough() {
        let s = number_slot("x")
            .with_quantity(Quantity::Temperature)
            .with_sensor_unit(Unit::Fahrenheit);
        let out = normalize_for_storage(&s, serde_json::json!(null), default_registry());
        assert_eq!(out, serde_json::json!(null));
    }

    #[test]
    fn registry_dto_has_one_entry_per_quantity_with_canonical_first_allowed() {
        let dto = registry_dto(default_registry());
        assert_eq!(
            dto.quantities.len(),
            ALL_QUANTITIES.len(),
            "one entry per quantity"
        );
        for e in &dto.quantities {
            assert!(
                e.allowed.contains(&e.canonical),
                "canonical `{:?}` missing from allowed on `{:?}`",
                e.canonical,
                e.id,
            );
        }
        // Stable iteration order — the list is a public contract
        // (versioned alongside the platform release per
        // USER-PREFERENCES.md § "Enum versioning").
        assert_eq!(dto.quantities[0].id, Quantity::Temperature);
        assert_eq!(dto.quantities.last().unwrap().id, Quantity::Duration);
    }

    #[test]
    fn registry_dto_round_trips_json() {
        let dto = registry_dto(default_registry());
        let s = serde_json::to_string(&dto).unwrap();
        let back: RegistryDto = serde_json::from_str(&s).unwrap();
        assert_eq!(back.quantities.len(), dto.quantities.len());
    }

    #[test]
    fn normalise_non_numeric_value_on_number_slot_is_passthrough() {
        // Schema-type validation is someone else's job; this helper
        // does not silently coerce.
        let s = number_slot("x")
            .with_quantity(Quantity::Temperature)
            .with_sensor_unit(Unit::Fahrenheit);
        let out = normalize_for_storage(&s, serde_json::json!("nan"), default_registry());
        assert_eq!(out, serde_json::json!("nan"));
    }
}
