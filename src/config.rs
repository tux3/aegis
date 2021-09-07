use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};
use sodiumoxide::base64;
use sodiumoxide::crypto::sign;
use std::fmt::Formatter;
use std::path::Path;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub db_host: String,
    pub db_name: String,
    pub db_user: String,
    pub db_pass: String,
    #[serde(default = "db_max_conn_default")]
    pub db_max_conn: u32,
    #[serde(deserialize_with = "deserialize_pub_sig_key")]
    pub root_public_signature_key: sign::PublicKey,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Config {
        let contents = std::fs::read_to_string(path.as_ref()).expect("Failed to read config file");
        toml::from_str(&contents).expect("Invalid config file format")
    }
}

fn db_max_conn_default() -> u32 {
    16
}

fn deserialize_pub_sig_key<'de, D>(deser: D) -> Result<sign::PublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor {}
    impl<'de> Visitor<'de> for StrVisitor {
        type Value = sign::PublicKey;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "a base64 urlsafe nopad public signature key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            let bytes = base64::decode(v, base64::Variant::UrlSafeNoPadding)
                .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))?;
            sign::PublicKey::from_slice(&bytes)
                .ok_or_else(|| Error::invalid_length(bytes.len(), &self))
        }
    }

    deser.deserialize_str(StrVisitor {})
}
