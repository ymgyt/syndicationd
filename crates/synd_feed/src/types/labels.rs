use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Category, Requirement};

/// User-defined labels with string keys and JSON values.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct Labels(BTreeMap<String, Value>);

/// Labels a subscriber assigns to a feed subscription.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct SubscriptionLabels(Labels);

impl SubscriptionLabels {
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        let Labels(values) = &mut self.0;
        values.insert(key.into(), value)
    }

    pub fn set_requirement(&mut self, requirement: Requirement) {
        self.insert(
            "requirement",
            requirement.to_string().to_ascii_lowercase().into(),
        );
    }

    pub fn set_category(&mut self, category: Category<'_>) {
        self.insert("category", category.into_inner().into_owned().into());
    }

    pub fn requirement(&self) -> Option<Requirement> {
        let Labels(values) = &self.0;
        values.get("requirement")?.as_str()?.parse().ok()
    }

    pub fn category(&self) -> Option<Category<'static>> {
        let Labels(values) = &self.0;
        Category::new(values.get("category")?.as_str()?.to_owned()).ok()
    }
}
