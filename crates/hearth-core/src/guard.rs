//! Data guard. Runs before EVERY model call.
//! Private fields pass only for modules the owner
//! switched on for AI. Default: AI off everywhere.

use serde::Serialize;
use serde_json::Value;

/// Data class carried by each field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataClass {
    /// Safe for the model (titles, counts, labels).
    General,
    /// Health, finance, journal, business records.
    Private,
}

/// One named field aimed at the model.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    /// Field name, e.g. "mood".
    pub name: String,
    /// Its data class.
    pub class: DataClass,
    /// The value.
    pub value: Value,
}

impl Field {
    pub fn general(name: impl Into<String>, value: Value) -> Self {
        Self {
            name: name.into(),
            class: DataClass::General,
            value,
        }
    }

    pub fn private(name: impl Into<String>, value: Value) -> Self {
        Self {
            name: name.into(),
            class: DataClass::Private,
            value,
        }
    }
}

/// The guard. One method: filter fields for a model call.
pub struct DataGuard;

impl DataGuard {
    /// Keep general fields always. Keep private fields
    /// only when `ai_on` is true for this module.
    pub fn filter(fields: Vec<Field>, ai_on: bool) -> Vec<Field> {
        fields
            .into_iter()
            .filter(|f| ai_on || f.class == DataClass::General)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Vec<Field> {
        vec![
            Field::general("count", json!(3)),
            Field::private("mood", json!("low")),
        ]
    }

    #[test]
    fn ai_off_drops_private_fields() {
        let out = DataGuard::filter(sample(), false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "count");
    }

    #[test]
    fn ai_on_keeps_all_fields() {
        let out = DataGuard::filter(sample(), true);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn empty_in_empty_out() {
        let out = DataGuard::filter(vec![], true);
        assert!(out.is_empty());
    }
}
