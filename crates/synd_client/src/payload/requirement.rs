use serde::{Deserialize, Deserializer, Serializer, de};
use synd_feed::types::Requirement;

const VARIANTS: &[&str] = &["MUST", "SHOULD", "MAY"];

#[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
pub(super) fn serialize<S>(value: &Option<Requirement>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(Requirement::Must) => serializer.serialize_some("MUST"),
        Some(Requirement::Should) => serializer.serialize_some("SHOULD"),
        Some(Requirement::May) => serializer.serialize_some("MAY"),
        None => serializer.serialize_none(),
    }
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Requirement>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };

    match value.as_str() {
        "MUST" => Ok(Some(Requirement::Must)),
        "SHOULD" => Ok(Some(Requirement::Should)),
        "MAY" => Ok(Some(Requirement::May)),
        value => Err(de::Error::unknown_variant(value, VARIANTS)),
    }
}
