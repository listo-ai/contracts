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

impl Quantity {
    /// Stable lower-snake identifier — matches `serde(rename_all =
    /// "snake_case")`. Useful when constructing wire payloads by
    /// hand, building preference-claim names, or keying a UI lookup
    /// table against the same string the wire uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::Pressure => "pressure",
            Self::FlowRate => "flow_rate",
            Self::Volume => "volume",
            Self::Mass => "mass",
            Self::Length => "length",
            Self::Energy => "energy",
            Self::Power => "power",
            Self::Speed => "speed",
            Self::Ratio => "ratio",
            Self::Duration => "duration",
        }
    }

    /// Human-friendly English name for UI labels. Locale-aware
    /// rendering goes through the client's i18n layer (ICU4X /
    /// `react-intl`) — this is the compact fallback for CLI output
    /// and logs where no formatter is available.
    pub fn label(self) -> &'static str {
        match self {
            Self::Temperature => "Temperature",
            Self::Pressure => "Pressure",
            Self::FlowRate => "Flow rate",
            Self::Volume => "Volume",
            Self::Mass => "Mass",
            Self::Length => "Length",
            Self::Energy => "Energy",
            Self::Power => "Power",
            Self::Speed => "Speed",
            Self::Ratio => "Ratio",
            Self::Duration => "Duration",
        }
    }
}

impl std::str::FromStr for Quantity {
    type Err = UnknownQuantity;

    /// Parse from the snake-case wire form (inverse of [`Self::as_str`]).
    ///
    /// Matches the `#[serde(rename_all = "snake_case")]` form so
    /// callers can accept CLI args / YAML values without routing
    /// through `serde_json::from_str` just to parse one token.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "temperature" => Self::Temperature,
            "pressure" => Self::Pressure,
            "flow_rate" => Self::FlowRate,
            "volume" => Self::Volume,
            "mass" => Self::Mass,
            "length" => Self::Length,
            "energy" => Self::Energy,
            "power" => Self::Power,
            "speed" => Self::Speed,
            "ratio" => Self::Ratio,
            "duration" => Self::Duration,
            other => return Err(UnknownQuantity(other.to_string())),
        })
    }
}

/// Returned by [`Quantity::from_str`] when the input doesn't match a
/// known variant. Kept small + concrete so CLI / config code can map
/// it to a readable error without carrying the full `serde` error
/// machinery.
#[derive(Debug, thiserror::Error)]
#[error("unknown quantity `{0}`")]
pub struct UnknownQuantity(pub String);

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

impl Unit {
    /// Stable lower-snake identifier — matches `serde(rename_all =
    /// "snake_case")`. Wire form is always this string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
            Self::Kelvin => "kelvin",
            Self::Kilopascal => "kilopascal",
            Self::Bar => "bar",
            Self::Psi => "psi",
            Self::Hectopascal => "hectopascal",
            Self::LitersPerSecond => "liters_per_second",
            Self::LitersPerMinute => "liters_per_minute",
            Self::CubicMetersPerHour => "cubic_meters_per_hour",
            Self::GallonsPerMinute => "gallons_per_minute",
            Self::Liter => "liter",
            Self::CubicMeter => "cubic_meter",
            Self::UsGallon => "us_gallon",
            Self::ImperialGallon => "imperial_gallon",
            Self::Kilogram => "kilogram",
            Self::Gram => "gram",
            Self::Pound => "pound",
            Self::Ounce => "ounce",
            Self::Meter => "meter",
            Self::Millimeter => "millimeter",
            Self::Kilometer => "kilometer",
            Self::Inch => "inch",
            Self::Foot => "foot",
            Self::Mile => "mile",
            Self::Kilowatt => "kilowatt",
            Self::Watt => "watt",
            Self::Horsepower => "horsepower",
            Self::KilowattHour => "kilowatt_hour",
            Self::Joule => "joule",
            Self::MetersPerSecond => "meters_per_second",
            Self::KilometersPerHour => "kilometers_per_hour",
            Self::MilesPerHour => "miles_per_hour",
            Self::Knot => "knot",
            Self::Ratio => "ratio",
            Self::Percent => "percent",
            Self::Millisecond => "millisecond",
            Self::Second => "second",
            Self::Minute => "minute",
            Self::Hour => "hour",
        }
    }

    /// Compact display symbol — the "°C" / "psi" / "L/s" form a
    /// unit-picker uses when space is tight. Not locale-aware;
    /// clients that want localised symbols go through ICU4X. This
    /// is the pragmatic fallback for CLI, logs, and any surface
    /// without an i18n framework.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Kelvin => "K",
            Self::Kilopascal => "kPa",
            Self::Bar => "bar",
            Self::Psi => "psi",
            Self::Hectopascal => "hPa",
            Self::LitersPerSecond => "L/s",
            Self::LitersPerMinute => "L/min",
            Self::CubicMetersPerHour => "m³/h",
            Self::GallonsPerMinute => "gpm",
            Self::Liter => "L",
            Self::CubicMeter => "m³",
            Self::UsGallon => "gal",
            Self::ImperialGallon => "imp gal",
            Self::Kilogram => "kg",
            Self::Gram => "g",
            Self::Pound => "lb",
            Self::Ounce => "oz",
            Self::Meter => "m",
            Self::Millimeter => "mm",
            Self::Kilometer => "km",
            Self::Inch => "in",
            Self::Foot => "ft",
            Self::Mile => "mi",
            Self::Kilowatt => "kW",
            Self::Watt => "W",
            Self::Horsepower => "hp",
            Self::KilowattHour => "kWh",
            Self::Joule => "J",
            Self::MetersPerSecond => "m/s",
            Self::KilometersPerHour => "km/h",
            Self::MilesPerHour => "mph",
            Self::Knot => "kn",
            Self::Ratio => "",
            Self::Percent => "%",
            Self::Millisecond => "ms",
            Self::Second => "s",
            Self::Minute => "min",
            Self::Hour => "h",
        }
    }

    /// Human-friendly English name for UI labels (compare with
    /// [`Self::symbol`] for the compact form). Used by a unit-picker
    /// that wants "Degrees Celsius" alongside "°C".
    pub fn label(self) -> &'static str {
        match self {
            Self::Celsius => "Degrees Celsius",
            Self::Fahrenheit => "Degrees Fahrenheit",
            Self::Kelvin => "Kelvin",
            Self::Kilopascal => "Kilopascals",
            Self::Bar => "Bar",
            Self::Psi => "Pounds per square inch",
            Self::Hectopascal => "Hectopascals",
            Self::LitersPerSecond => "Liters per second",
            Self::LitersPerMinute => "Liters per minute",
            Self::CubicMetersPerHour => "Cubic meters per hour",
            Self::GallonsPerMinute => "Gallons per minute",
            Self::Liter => "Liters",
            Self::CubicMeter => "Cubic meters",
            Self::UsGallon => "US gallons",
            Self::ImperialGallon => "Imperial gallons",
            Self::Kilogram => "Kilograms",
            Self::Gram => "Grams",
            Self::Pound => "Pounds",
            Self::Ounce => "Ounces",
            Self::Meter => "Meters",
            Self::Millimeter => "Millimeters",
            Self::Kilometer => "Kilometers",
            Self::Inch => "Inches",
            Self::Foot => "Feet",
            Self::Mile => "Miles",
            Self::Kilowatt => "Kilowatts",
            Self::Watt => "Watts",
            Self::Horsepower => "Horsepower",
            Self::KilowattHour => "Kilowatt-hours",
            Self::Joule => "Joules",
            Self::MetersPerSecond => "Meters per second",
            Self::KilometersPerHour => "Kilometers per hour",
            Self::MilesPerHour => "Miles per hour",
            Self::Knot => "Knots",
            Self::Ratio => "Ratio",
            Self::Percent => "Percent",
            Self::Millisecond => "Milliseconds",
            Self::Second => "Seconds",
            Self::Minute => "Minutes",
            Self::Hour => "Hours",
        }
    }
}

impl Unit {
    /// Return the [`Quantity`] this unit belongs to, if any. Reverse
    /// lookup over the registry's `allowed` sets. Useful for input
    /// validation ("is `temperature_unit: psi` actually allowed?")
    /// and for CLI / config code that has a unit string but wants to
    /// confirm what quantity it applies to.
    ///
    /// `None` for units that don't appear in any quantity's allowed
    /// set — which shouldn't happen for production variants, but
    /// stays as `Option` so adding a future-experimental unit doesn't
    /// force a panic.
    pub fn quantity(self) -> Option<Quantity> {
        let registry = default_registry();
        for q in ALL_QUANTITIES {
            if registry.allows(*q, self) {
                return Some(*q);
            }
        }
        None
    }
}

impl std::str::FromStr for Unit {
    type Err = UnknownUnit;

    /// Parse from the snake-case wire form (inverse of [`Self::as_str`]).
    /// Same pattern as [`Quantity::from_str`]: lets CLI / config code
    /// accept a unit without round-tripping through serde.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "celsius" => Self::Celsius,
            "fahrenheit" => Self::Fahrenheit,
            "kelvin" => Self::Kelvin,
            "kilopascal" => Self::Kilopascal,
            "bar" => Self::Bar,
            "psi" => Self::Psi,
            "hectopascal" => Self::Hectopascal,
            "liters_per_second" => Self::LitersPerSecond,
            "liters_per_minute" => Self::LitersPerMinute,
            "cubic_meters_per_hour" => Self::CubicMetersPerHour,
            "gallons_per_minute" => Self::GallonsPerMinute,
            "liter" => Self::Liter,
            "cubic_meter" => Self::CubicMeter,
            "us_gallon" => Self::UsGallon,
            "imperial_gallon" => Self::ImperialGallon,
            "kilogram" => Self::Kilogram,
            "gram" => Self::Gram,
            "pound" => Self::Pound,
            "ounce" => Self::Ounce,
            "meter" => Self::Meter,
            "millimeter" => Self::Millimeter,
            "kilometer" => Self::Kilometer,
            "inch" => Self::Inch,
            "foot" => Self::Foot,
            "mile" => Self::Mile,
            "kilowatt" => Self::Kilowatt,
            "watt" => Self::Watt,
            "horsepower" => Self::Horsepower,
            "kilowatt_hour" => Self::KilowattHour,
            "joule" => Self::Joule,
            "meters_per_second" => Self::MetersPerSecond,
            "kilometers_per_hour" => Self::KilometersPerHour,
            "miles_per_hour" => Self::MilesPerHour,
            "knot" => Self::Knot,
            "ratio" => Self::Ratio,
            "percent" => Self::Percent,
            "millisecond" => Self::Millisecond,
            "second" => Self::Second,
            "minute" => Self::Minute,
            "hour" => Self::Hour,
            other => return Err(UnknownUnit(other.to_string())),
        })
    }
}

/// Returned by [`Unit::from_str`] when the input doesn't match a
/// known variant. See [`UnknownQuantity`] for the paired quantity
/// error.
#[derive(Debug, thiserror::Error)]
#[error("unknown unit `{0}`")]
pub struct UnknownUnit(pub String);

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
/// with human-friendly fields added so unit-picker UIs don't have to
/// hard-code labels.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QuantityEntry {
    /// Serialised quantity id (e.g. `"temperature"`).
    pub id: Quantity,
    /// Human-friendly English name — "Temperature", "Flow rate".
    /// Locale-aware rendering is a client concern; this is the
    /// compact fallback.
    pub label: String,
    /// Canonical (storage) unit for this quantity.
    pub canonical: Unit,
    /// Every unit a user preference or slot schema can select for
    /// this quantity. Always includes the canonical.
    pub allowed: Vec<Unit>,
    /// Compact symbol for rendering the quantity itself (typically
    /// the canonical unit's symbol, e.g. `"°C"` for temperature).
    pub symbol: String,
}

/// One row in [`RegistryDto::units`]. Flat table so clients look up
/// any unit's label/symbol/conversion by id in O(1) without hitting
/// the quantity map first. Every unit appears exactly once.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UnitEntry {
    pub id: Unit,
    /// Compact symbol for space-constrained render — "°C", "psi",
    /// "L/s". Locale-neutral.
    pub symbol: String,
    /// Human-friendly English name — "Degrees Celsius", "Pounds per
    /// square inch". Localised rendering is a client concern.
    pub label: String,
    /// Affine conversion coefficients **to this unit's quantity
    /// canonical**:
    ///
    /// ```text
    /// canonical_value = scale * value + offset
    /// ```
    ///
    /// Inverse: `value = (canonical_value - offset) / scale`.
    ///
    /// Affine covers both linear conversions (`bar → kPa` = `×100 +
    /// 0`) and the one non-linear unit in the registry —
    /// temperature: `°F → °C` = `×5/9 + −17.78…`. A future truly
    /// non-affine unit (logarithmic, for example) would need a
    /// richer representation and a major bump.
    ///
    /// Shipped on the wire so clients apply conversion with the
    /// server's own factors — no hard-coded TS/Go tables drifting
    /// from the `uom`-backed `StaticRegistry`. See
    /// `USER-PREFERENCES.md § "Slot units"`.
    ///
    /// `None` for units that don't participate in conversion (none
    /// today; the field is optional so dimensionless additions in
    /// future versions don't force a major bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_canonical: Option<AffineCoeffs>,
}

/// Coefficients of an affine conversion to the canonical unit of a
/// quantity. See [`UnitEntry::to_canonical`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AffineCoeffs {
    pub scale: f64,
    pub offset: f64,
}

/// Full wire shape of `GET /api/v1/units`. Lets clients drive
/// unit-picker UIs without hard-coding any string. See
/// `agent/docs/design/USER-PREFERENCES.md` § "API surface" /
/// "Enum versioning and canonical-unit migration".
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RegistryDto {
    /// Quantities in enum-declaration order (stable across
    /// platform releases; adding a variant is additive, renaming
    /// requires a major bump).
    pub quantities: Vec<QuantityEntry>,
    /// Flat table of every unit — symbol + label keyed by id.
    /// Clients that render a unit-picker read from here; the
    /// per-quantity `allowed` list tells them which subset is valid
    /// for a given slot / preference field.
    pub units: Vec<UnitEntry>,
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
///
/// The `units` table is populated by walking every quantity's
/// `allowed` set, deduplicated by unit id. This keeps the flat table
/// in sync with whatever the registry declares as reachable — a unit
/// the registry can't use won't appear, so clients never see stale
/// entries.
pub fn registry_dto(registry: &dyn UnitRegistry) -> RegistryDto {
    let quantities: Vec<QuantityEntry> = ALL_QUANTITIES
        .iter()
        .map(|q| {
            let def = registry.quantity(*q);
            QuantityEntry {
                id: *q,
                label: q.label().to_string(),
                canonical: def.canonical,
                allowed: def.allowed.to_vec(),
                symbol: def.symbol.to_string(),
            }
        })
        .collect();

    // Flat unit table — deduplicate while preserving first-seen
    // order so the output is deterministic across runs and platform
    // versions. `Unit` doesn't implement `Ord` (closed enum with
    // semantic rather than sortable identity), so dedupe via
    // `Vec::contains` — O(n²) over a fixed-size set of ~40 units,
    // which is cheaper than the alternatives.
    //
    // Each entry carries affine `to_canonical` coefficients derived
    // **from the registry's own converter**. Two probe points
    // (`value=0` and `value=1`) are enough to recover `scale` and
    // `offset` for any affine transform — no duplicate factor
    // tables, and a server-side change to `uom` mappings
    // automatically changes what the wire ships.
    let mut units: Vec<UnitEntry> = Vec::new();
    for entry in &quantities {
        let canonical = entry.canonical;
        for u in &entry.allowed {
            if !units.iter().any(|e| e.id == *u) {
                let coeffs = derive_affine(registry, entry.id, *u, canonical);
                units.push(UnitEntry {
                    id: *u,
                    symbol: u.symbol().to_string(),
                    label: u.label().to_string(),
                    to_canonical: coeffs,
                });
            }
        }
    }

    RegistryDto { quantities, units }
}

/// Recover the affine `scale` + `offset` of a unit-to-canonical
/// conversion by probing the registry at `value=0` and `value=1`.
/// Works for every affine transform (linear + offset), which covers
/// every unit in the current registry including temperature.
///
/// Returns `Some` for the canonical unit (trivially `{1.0, 0.0}`) and
/// for every other allowed unit. Exotic units added later that are
/// genuinely non-affine (logarithmic dB, tone scales) should return
/// `None` and accept the client rendering them as canonical.
fn derive_affine(
    registry: &dyn UnitRegistry,
    q: Quantity,
    unit: Unit,
    canonical: Unit,
) -> Option<AffineCoeffs> {
    let at_zero = registry.convert(q, 0.0, unit, canonical);
    let at_one = registry.convert(q, 1.0, unit, canonical);
    let scale = at_one - at_zero;
    let offset = at_zero;
    // Sanity: NaN / infinities indicate a non-affine unit the
    // registry handles internally but can't be summarised as
    // scale + offset.
    if !scale.is_finite() || !offset.is_finite() {
        return None;
    }
    Some(AffineCoeffs { scale, offset })
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
            assert!(!e.label.is_empty(), "quantity `{:?}` missing label", e.id);
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
        assert_eq!(back.units.len(), dto.units.len());
    }

    #[test]
    fn registry_dto_units_table_has_one_entry_per_unique_unit() {
        let dto = registry_dto(default_registry());
        // Gather the union of every quantity's allowed set via
        // Vec::contains (Unit doesn't impl Ord; see registry_dto
        // comment for why).
        let mut expected: Vec<Unit> = Vec::new();
        for q in &dto.quantities {
            for u in &q.allowed {
                if !expected.contains(u) {
                    expected.push(*u);
                }
            }
        }
        assert_eq!(
            dto.units.len(),
            expected.len(),
            "flat unit table should dedupe across quantities",
        );
        for entry in &dto.units {
            assert!(
                !entry.symbol.is_empty() || entry.id == Unit::Ratio,
                "unit `{:?}` missing symbol",
                entry.id,
            );
            assert!(!entry.label.is_empty(), "unit `{:?}` missing label", entry.id);
        }
    }

    #[test]
    fn quantity_as_str_and_from_str_round_trip() {
        use std::str::FromStr;
        for q in ALL_QUANTITIES {
            let s = q.as_str();
            let parsed = Quantity::from_str(s).expect("known wire form parses");
            assert_eq!(*q, parsed, "quantity `{}` round-trips", s);
        }
    }

    #[test]
    fn quantity_from_str_rejects_unknown() {
        use std::str::FromStr;
        let err = Quantity::from_str("not_a_quantity").unwrap_err();
        assert!(err.to_string().contains("not_a_quantity"));
    }

    #[test]
    fn unit_as_str_and_from_str_round_trip() {
        use std::str::FromStr;
        for u in [
            Unit::Celsius, Unit::Fahrenheit, Unit::Kelvin,
            Unit::Kilopascal, Unit::Bar, Unit::Psi, Unit::Hectopascal,
            Unit::LitersPerSecond, Unit::LitersPerMinute,
            Unit::CubicMetersPerHour, Unit::GallonsPerMinute,
            Unit::Liter, Unit::CubicMeter, Unit::UsGallon, Unit::ImperialGallon,
            Unit::Kilogram, Unit::Gram, Unit::Pound, Unit::Ounce,
            Unit::Meter, Unit::Millimeter, Unit::Kilometer,
            Unit::Inch, Unit::Foot, Unit::Mile,
            Unit::Kilowatt, Unit::Watt, Unit::Horsepower,
            Unit::KilowattHour, Unit::Joule,
            Unit::MetersPerSecond, Unit::KilometersPerHour,
            Unit::MilesPerHour, Unit::Knot,
            Unit::Ratio, Unit::Percent,
            Unit::Millisecond, Unit::Second, Unit::Minute, Unit::Hour,
        ] {
            let s = u.as_str();
            let parsed = Unit::from_str(s).expect("known unit wire form parses");
            assert_eq!(u, parsed, "unit `{}` round-trips", s);
        }
    }

    #[test]
    fn unit_from_str_rejects_unknown() {
        use std::str::FromStr;
        let err = Unit::from_str("unobtainium").unwrap_err();
        assert!(err.to_string().contains("unobtainium"));
    }

    #[test]
    fn affine_coefficients_round_trip_against_registry_convert() {
        // The registry exposes conversion via `convert(q, v, from,
        // to)`. The flat unit table now ships `{scale, offset}`
        // coefficients so clients convert without a round-trip.
        // This test enforces the invariant: for every allowed unit,
        // applying the published `to_canonical` coefficients to a
        // probe set matches the registry's own conversion exactly
        // (within float tolerance).
        let r = default_registry();
        let dto = registry_dto(r);
        for q_entry in &dto.quantities {
            let canonical = q_entry.canonical;
            for unit_id in &q_entry.allowed {
                let unit_entry = dto
                    .units
                    .iter()
                    .find(|u| u.id == *unit_id)
                    .expect("every allowed unit appears in flat table");
                let coeffs = unit_entry
                    .to_canonical
                    .expect("allowed units ship coefficients");
                // Probe several values — a single 0/1 pair would
                // only prove the derivation method, not that it's
                // actually affine for this unit.
                for probe in [-100.0, -1.0, 0.0, 1.0, 42.0, 1_000_000.0] {
                    let via_coeffs = coeffs.scale * probe + coeffs.offset;
                    let via_registry = r.convert(q_entry.id, probe, *unit_id, canonical);
                    // Relative tolerance: 1e-9 of the magnitude, or
                    // 1e-9 absolute for near-zero values. Tight
                    // enough to catch a factor being off, loose
                    // enough to absorb f64 rounding.
                    let scale = via_registry.abs().max(1.0);
                    let err = (via_coeffs - via_registry).abs();
                    assert!(
                        err / scale < 1e-9,
                        "coefficient drift for {:?}/{:?}: coeffs gave {}, registry gave {} (err {})",
                        q_entry.id,
                        *unit_id,
                        via_coeffs,
                        via_registry,
                        err,
                    );
                }
            }
        }
    }

    #[test]
    fn temperature_fahrenheit_coefficients_match_known_affine() {
        // Sanity check: °F → °C is the one non-linear conversion
        // in the registry. Verify the derived coefficients are the
        // well-known 5/9 and −160/9 values (the latter = −32 × 5/9).
        let dto = registry_dto(default_registry());
        let f = dto.units.iter().find(|u| u.id == Unit::Fahrenheit).unwrap();
        let c = f.to_canonical.unwrap();
        assert!((c.scale - 5.0 / 9.0).abs() < 1e-12, "scale={}", c.scale);
        assert!((c.offset - (-160.0 / 9.0)).abs() < 1e-12, "offset={}", c.offset);
    }

    #[test]
    fn every_unit_maps_back_to_its_quantity() {
        // Closed-enum invariant: every `Unit` variant is in the
        // `allowed` set of at least one `Quantity`. If a variant
        // ever gets added without being wired into a quantity,
        // `Unit::quantity()` returns None for it and this test
        // fails — cheap integrity guard on future edits.
        for u in [
            Unit::Celsius, Unit::Fahrenheit, Unit::Kelvin,
            Unit::Kilopascal, Unit::Bar, Unit::Psi, Unit::Hectopascal,
            Unit::LitersPerSecond, Unit::LitersPerMinute,
            Unit::CubicMetersPerHour, Unit::GallonsPerMinute,
            Unit::Liter, Unit::CubicMeter, Unit::UsGallon, Unit::ImperialGallon,
            Unit::Kilogram, Unit::Gram, Unit::Pound, Unit::Ounce,
            Unit::Meter, Unit::Millimeter, Unit::Kilometer,
            Unit::Inch, Unit::Foot, Unit::Mile,
            Unit::Kilowatt, Unit::Watt, Unit::Horsepower,
            Unit::KilowattHour, Unit::Joule,
            Unit::MetersPerSecond, Unit::KilometersPerHour,
            Unit::MilesPerHour, Unit::Knot,
            Unit::Ratio, Unit::Percent,
            Unit::Millisecond, Unit::Second, Unit::Minute, Unit::Hour,
        ] {
            assert!(
                u.quantity().is_some(),
                "unit `{u:?}` is not in any quantity's allowed set",
            );
        }
    }

    #[test]
    fn unit_quantity_lookup_returns_expected_quantities() {
        assert_eq!(Unit::Celsius.quantity(), Some(Quantity::Temperature));
        assert_eq!(Unit::Psi.quantity(), Some(Quantity::Pressure));
        assert_eq!(Unit::KilowattHour.quantity(), Some(Quantity::Energy));
        assert_eq!(Unit::Knot.quantity(), Some(Quantity::Speed));
    }

    #[test]
    fn canonical_units_have_identity_coefficients() {
        let dto = registry_dto(default_registry());
        for q in &dto.quantities {
            let entry = dto.units.iter().find(|u| u.id == q.canonical).unwrap();
            let c = entry.to_canonical.unwrap();
            assert!((c.scale - 1.0).abs() < 1e-12, "{:?} scale={}", q.id, c.scale);
            assert!(c.offset.abs() < 1e-12, "{:?} offset={}", q.id, c.offset);
        }
    }

    #[test]
    fn enum_serde_matches_from_str_exactly() {
        use std::str::FromStr;
        // If serde's `rename_all = "snake_case"` ever diverges from
        // our hand-written as_str/from_str, this test catches it on
        // the next build — keeping the two source-of-truth strings
        // aligned.
        for q in ALL_QUANTITIES {
            let via_serde = serde_json::to_string(q).unwrap();
            // to_string wraps in quotes — `"temperature"` — strip them.
            let stripped = via_serde.trim_matches('"');
            assert_eq!(stripped, q.as_str(), "serde form for {q:?} diverged");
            assert_eq!(*q, Quantity::from_str(stripped).unwrap());
        }
    }

    #[test]
    fn unit_symbol_and_label_are_defined_for_every_variant() {
        // Compile-time: the match in `Unit::symbol`/`label` is
        // exhaustive. Runtime: ensure no variant resolves to an
        // empty label (symbol can be empty for Ratio by design).
        for u in [
            Unit::Celsius, Unit::Fahrenheit, Unit::Kelvin,
            Unit::Kilopascal, Unit::Bar, Unit::Psi, Unit::Hectopascal,
            Unit::LitersPerSecond, Unit::LitersPerMinute,
            Unit::CubicMetersPerHour, Unit::GallonsPerMinute,
            Unit::Liter, Unit::CubicMeter, Unit::UsGallon, Unit::ImperialGallon,
            Unit::Kilogram, Unit::Gram, Unit::Pound, Unit::Ounce,
            Unit::Meter, Unit::Millimeter, Unit::Kilometer,
            Unit::Inch, Unit::Foot, Unit::Mile,
            Unit::Kilowatt, Unit::Watt, Unit::Horsepower,
            Unit::KilowattHour, Unit::Joule,
            Unit::MetersPerSecond, Unit::KilometersPerHour,
            Unit::MilesPerHour, Unit::Knot,
            Unit::Ratio, Unit::Percent,
            Unit::Millisecond, Unit::Second, Unit::Minute, Unit::Hour,
        ] {
            assert!(!u.label().is_empty(), "unit `{u:?}` has empty label");
            assert!(!u.as_str().is_empty(), "unit `{u:?}` has empty id");
        }
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
