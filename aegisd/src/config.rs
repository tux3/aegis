use ed25519_dalek::PublicKey;
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};
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
    pub root_public_signature_key: PublicKey,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Config {
        let contents = std::fs::read_to_string(path.as_ref()).expect("Failed to read config file");
        toml::from_str(&contents).expect("Invalid config file format")
    }

    #[cfg(test)]
    pub(crate) fn test_config(test_root_public_key: PublicKey) -> Self {
        Self {
            port: 8080,
            db_host: ":memory:".to_string(),
            db_name: "aegisd".to_string(),
            db_user: "aegisd".to_string(),
            db_pass: "test_password".to_string(),
            db_max_conn: db_max_conn_default(),
            root_public_signature_key: test_root_public_key,
        }
    }
}

fn db_max_conn_default() -> u32 {
    16
}

fn deserialize_pub_sig_key<'de, D>(deser: D) -> Result<PublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor {}
    impl<'de> Visitor<'de> for StrVisitor {
        type Value = PublicKey;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "a base64 urlsafe nopad public signature key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            let bytes = base64::decode_config(v, base64::URL_SAFE_NO_PAD)
                .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))?;
            PublicKey::from_bytes(&bytes).map_err(|_| Error::invalid_length(bytes.len(), &self))
        }
    }

    deser.deserialize_str(StrVisitor {})
}
