pub mod bool_from_int {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(u8::deserialize(deserializer)? != 0)
    }

    pub fn serialize<S>(b: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(if *b { 1 } else { 0 })
    }
}

pub mod lines_from_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    pub fn serialize<S>(b: &[String], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&b.join("\n"))
    }
}
