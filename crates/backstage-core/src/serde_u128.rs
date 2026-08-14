use std::fmt;

use serde::{Deserialize, Deserializer, Serializer, de};

pub mod option_decimal_string {
    use super::*;

    pub fn serialize<S>(value: &Option<u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_str(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionalU128Visitor)
    }

    struct OptionalU128Visitor;

    impl<'de> de::Visitor<'de> for OptionalU128Visitor {
        type Value = Option<u128>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("null, a non-negative integer, or a decimal integer string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            U128Visitor::deserialize(deserializer).map(|value| Some(value.0))
        }
    }

    struct U128Visitor(u128);

    impl<'de> Deserialize<'de> for U128Visitor {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(Self(0))
        }
    }

    impl<'de> de::Visitor<'de> for U128Visitor {
        type Value = U128Visitor;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a non-negative integer or a decimal integer string")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(Self(u128::from(value)))
        }

        fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E> {
            Ok(Self(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u128::try_from(value)
                .map(Self)
                .map_err(|_| E::custom("timestamp must not be negative"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map(Self).map_err(E::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }
}
