pub mod bool_from_int {
    use serde::{Deserialize as _, Deserializer, Serializer};

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
        serializer.serialize_u8(u8::from(*b))
    }
}

pub mod lines_from_string {
    use serde::{Deserialize as _, Deserializer, Serializer};

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

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    // Tests for bool_from_int
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestBoolStruct {
        #[serde(with = "bool_from_int")]
        value: bool,
    }

    #[test]
    fn test_bool_from_int_deserialize_zero() {
        let json = r#"{"value": 0}"#;
        let result: TestBoolStruct = serde_json::from_str(json).unwrap();
        assert!(!result.value);
    }

    #[test]
    fn test_bool_from_int_deserialize_one() {
        let json = r#"{"value": 1}"#;
        let result: TestBoolStruct = serde_json::from_str(json).unwrap();
        assert!(result.value);
    }

    #[test]
    fn test_bool_from_int_deserialize_nonzero() {
        let json = r#"{"value": 42}"#;
        let result: TestBoolStruct = serde_json::from_str(json).unwrap();
        assert!(result.value);
    }

    #[test]
    fn test_bool_from_int_serialize_true() {
        let data = TestBoolStruct { value: true };
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(json, r#"{"value":1}"#);
    }

    #[test]
    fn test_bool_from_int_serialize_false() {
        let data = TestBoolStruct { value: false };
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(json, r#"{"value":0}"#);
    }

    #[test]
    fn test_bool_from_int_roundtrip_true() {
        let original = TestBoolStruct { value: true };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TestBoolStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_bool_from_int_roundtrip_false() {
        let original = TestBoolStruct { value: false };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TestBoolStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    // Tests for lines_from_string
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestLinesStruct {
        #[serde(with = "lines_from_string")]
        lines: Vec<String>,
    }

    #[test]
    fn test_lines_from_string_deserialize_empty() {
        let json = r#"{"lines": ""}"#;
        let result: TestLinesStruct = serde_json::from_str(json).unwrap();
        // Empty string produces an empty vec (Rust's lines() behavior)
        assert_eq!(result.lines, Vec::<String>::new());
    }

    #[test]
    fn test_lines_from_string_deserialize_single_line() {
        let json = r#"{"lines": "single line"}"#;
        let result: TestLinesStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.lines, vec!["single line"]);
    }

    #[test]
    fn test_lines_from_string_deserialize_multiple_lines() {
        let json = r#"{"lines": "line 1\nline 2\nline 3"}"#;
        let result: TestLinesStruct = serde_json::from_str(json).unwrap();
        assert_eq!(result.lines, vec!["line 1", "line 2", "line 3"]);
    }

    #[test]
    fn test_lines_from_string_deserialize_with_trailing_newline() {
        let json = r#"{"lines": "line 1\nline 2\n"}"#;
        let result: TestLinesStruct = serde_json::from_str(json).unwrap();
        // Trailing newline doesn't produce an extra empty line (Rust's lines() behavior)
        assert_eq!(result.lines, vec!["line 1", "line 2"]);
    }

    #[test]
    fn test_lines_from_string_serialize_empty_vec() {
        let data = TestLinesStruct { lines: vec![] };
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(json, r#"{"lines":""}"#);
    }

    #[test]
    fn test_lines_from_string_serialize_single_line() {
        let data = TestLinesStruct {
            lines: vec!["single line".to_owned()],
        };
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(json, r#"{"lines":"single line"}"#);
    }

    #[test]
    fn test_lines_from_string_serialize_multiple_lines() {
        let data = TestLinesStruct {
            lines: vec![
                "line 1".to_owned(),
                "line 2".to_owned(),
                "line 3".to_owned(),
            ],
        };
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(json, r#"{"lines":"line 1\nline 2\nline 3"}"#);
    }

    #[test]
    fn test_lines_from_string_roundtrip_single_line() {
        let original = TestLinesStruct {
            lines: vec!["single line".to_owned()],
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TestLinesStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lines_from_string_roundtrip_multiple_lines() {
        let original = TestLinesStruct {
            lines: vec![
                "line 1".to_owned(),
                "line 2".to_owned(),
                "line 3".to_owned(),
            ],
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TestLinesStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lines_from_string_roundtrip_empty() {
        let original = TestLinesStruct { lines: vec![] };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: TestLinesStruct = serde_json::from_str(&json).unwrap();
        assert!(deserialized.lines.is_empty());
    }

    #[test]
    fn test_lines_from_string_with_unicode() {
        let data = TestLinesStruct {
            lines: vec!["hello 世界".to_owned(), "こんにちは".to_owned()],
        };
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: TestLinesStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(data, deserialized);
    }
}
