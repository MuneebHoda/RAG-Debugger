use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize)]
#[serde(untagged)]
enum WireDateTime {
    Rfc3339(String),
    Legacy(OffsetDateTime),
}

pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let value = value.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
    value.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    match WireDateTime::deserialize(deserializer)? {
        WireDateTime::Rfc3339(value) => {
            OffsetDateTime::parse(&value, &Rfc3339).map_err(serde::de::Error::custom)
        }
        WireDateTime::Legacy(value) => Ok(value),
    }
}

pub mod option {
    use super::{Rfc3339, WireDateTime};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S>(value: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = value
            .map(|date_time| date_time.format(&Rfc3339))
            .transpose()
            .map_err(serde::ser::Error::custom)?;
        value.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<WireDateTime>::deserialize(deserializer)?;
        value
            .map(|value| match value {
                WireDateTime::Rfc3339(value) => {
                    OffsetDateTime::parse(&value, &Rfc3339).map_err(serde::de::Error::custom)
                }
                WireDateTime::Legacy(value) => Ok(value),
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};

    #[derive(Debug, Deserialize, Serialize)]
    struct RequiredDate {
        #[serde(with = "crate::wire_time")]
        value: OffsetDateTime,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct OptionalDate {
        #[serde(with = "crate::wire_time::option")]
        value: Option<OffsetDateTime>,
    }

    #[test]
    fn serializes_rfc3339_strings() {
        let expected = OffsetDateTime::parse("2026-06-27T10:46:19.985489Z", &Rfc3339)
            .expect("valid timestamp");
        let json = serde_json::to_value(RequiredDate { value: expected }).expect("date serializes");

        assert_eq!(json["value"], "2026-06-27T10:46:19.985489Z");
    }

    #[test]
    fn reads_legacy_time_arrays() {
        let expected = OffsetDateTime::parse("2026-06-27T10:46:19.985489Z", &Rfc3339)
            .expect("valid timestamp");
        let date: RequiredDate = serde_json::from_value(serde_json::json!({
            "value": [2026, 178, 10, 46, 19, 985489000, 0, 0, 0]
        }))
        .expect("legacy date deserializes");

        assert_eq!(date.value, expected);
    }

    #[test]
    fn reads_null_optional_dates() {
        let date: OptionalDate = serde_json::from_value(serde_json::json!({ "value": null }))
            .expect("optional date deserializes");

        assert_eq!(date.value, None);
    }
}
