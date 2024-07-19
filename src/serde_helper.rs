use chrono::{DateTime, Utc, NaiveDateTime, TimeZone};

struct DateTimeVisitor;

impl<'de> serde::de::Visitor<'de> for DateTimeVisitor {
    type Value = DateTime<Utc>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a datetime")
    }
    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error, {
        let time = NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap();
        Ok(Utc.from_utc_datetime(&time))
    }
}

pub fn serialize_datetime<S>(value: &DateTime<Utc>, serializer: S) -> std::result::Result<S::Ok, S::Error> 
    where S: serde::Serializer 
{
    let repr = value.format("%Y-%m-%dT%H:%M:%S%.3fZ");
    serializer.serialize_str(&repr.to_string())
}

pub fn deserialize_datetime<'de, D>(deserializer: D) -> std::result::Result<DateTime<Utc>, D::Error>
    where D: serde::Deserializer<'de>
{
    deserializer.deserialize_str(DateTimeVisitor)
}