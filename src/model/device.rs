use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use futures::future::BoxFuture;
use ormx::{Insert, Table};
use sodiumoxide::crypto::sign;
use sodiumoxide::crypto::sign::PublicKey;
use sqlx::database::HasArguments;
use sqlx::decode::Decode;
use sqlx::encode::{Encode, IsNull};
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgTypeInfo, PgValueRef};
use sqlx::{PgConnection, Postgres, Type};
use std::convert::TryFrom;

#[derive(Copy, Clone)]
pub struct DeviceKey(pub sign::PublicKey);

impl From<sign::PublicKey> for DeviceKey {
    fn from(k: PublicKey) -> Self {
        Self(k)
    }
}
impl TryFrom<Vec<u8>> for DeviceKey {
    type Error = anyhow::Error;
    fn try_from(k: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(sign::PublicKey::from_slice(&k).ok_or_else(|| {
            anyhow::anyhow!("Invalid device pubkey size")
        })?))
    }
}
impl Type<Postgres> for DeviceKey {
    fn type_info() -> PgTypeInfo {
        const BYTEA_OID: u32 = 17;
        PgTypeInfo::with_oid(BYTEA_OID)
    }
}
impl Encode<'_, Postgres> for DeviceKey {
    fn encode_by_ref(&self, buf: &mut <Postgres as HasArguments>::ArgumentBuffer) -> IsNull {
        <&[u8] as Encode<Postgres>>::encode(self.0.as_ref(), buf)
    }
}
impl Decode<'_, Postgres> for DeviceKey {
    fn decode(value: PgValueRef) -> Result<Self, BoxDynError> {
        let data = <&[u8] as Decode<Postgres>>::decode(value)?;
        let key = sign::PublicKey::from_slice(data)
            .ok_or_else(|| anyhow::anyhow!("Invalid DeviceKey value"))?;
        Ok(DeviceKey(key))
    }
}

#[derive(ormx::Table)]
#[ormx(table = "device", id = id, insertable)]
pub struct Device {
    #[ormx(default)]
    id: i32,
    created_at: NaiveDateTime,
    name: String,
    #[ormx(custom_type)]
    pubkey: DeviceKey,
    pending: bool,
}

pub struct PendingDevice {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub pubkey: DeviceKey,
}

impl Insert for PendingDevice {
    type Table = Device;

    fn insert(self, db: &mut PgConnection) -> BoxFuture<'_, sqlx::Result<Self::Table>> {
        Device::insert(
            db,
            InsertDevice {
                created_at: self.created_at,
                name: self.name,
                pubkey: self.pubkey,
                pending: true,
            },
        )
    }
}

impl From<PendingDevice> for aegislib::command::admin::PendingDevice {
    fn from(dev: PendingDevice) -> Self {
        Self {
            created_at: dev.created_at,
            name: dev.name,
            pubkey: dev.pubkey.0,
        }
    }
}

impl From<Device> for aegislib::command::admin::RegisteredDevice {
    fn from(dev: Device) -> Self {
        Self {
            created_at: dev.created_at,
            name: dev.name,
            pubkey: dev.pubkey.0,
        }
    }
}

pub async fn list_pending(conn: &mut PgConnection) -> Result<Vec<PendingDevice>> {
    let record = sqlx::query!("SELECT * FROM device WHERE pending = TRUE")
        .fetch_all(conn)
        .await?;
    Ok(record
        .into_iter()
        .map(|r| PendingDevice {
            created_at: r.created_at,
            name: r.name,
            pubkey: r.pubkey.try_into().unwrap(),
        })
        .collect())
}

pub async fn count_pending(conn: &mut PgConnection) -> Result<i64> {
    let record = sqlx::query!("SELECT COUNT(*) FROM device WHERE pending = TRUE")
        .fetch_one(conn)
        .await?;
    Ok(record.count.unwrap_or(0))
}

pub async fn delete_pending(conn: &mut PgConnection, name: &str) -> Result<()> {
    let result = sqlx::query!(
        "DELETE FROM device WHERE pending = TRUE AND name = $1",
        name
    )
    .execute(conn)
    .await?;
    if result.rows_affected() != 1 {
        debug_assert_eq!(result.rows_affected(), 0); // name is UNIQUE
        bail!("Pending device '{}' not found", name);
    }
    Ok(())
}

pub async fn confirm_pending(conn: &mut PgConnection, name: &str) -> Result<()> {
    let result = sqlx::query!(
        "UPDATE device SET pending = FALSE WHERE name = $1 AND pending = TRUE",
        name
    )
    .execute(conn)
    .await?;
    if result.rows_affected() != 1 {
        debug_assert_eq!(result.rows_affected(), 0); // name is UNIQUE
        bail!("Pending device '{}' not found", name);
    }
    Ok(())
}

pub async fn list_registered(conn: &mut PgConnection) -> Result<Vec<Device>> {
    let record = sqlx::query!("SELECT * FROM device WHERE pending = FALSE")
        .fetch_all(conn)
        .await?;
    Ok(record
        .into_iter()
        .map(|r| Device {
            id: r.id,
            created_at: r.created_at,
            name: r.name,
            pubkey: r.pubkey.try_into().unwrap(),
            pending: r.pending,
        })
        .collect())
}

pub async fn delete_registered(conn: &mut PgConnection, name: &str) -> Result<()> {
    let result = sqlx::query!(
        "DELETE FROM device WHERE pending = FALSE AND name = $1",
        name
    )
    .execute(conn)
    .await?;
    if result.rows_affected() != 1 {
        debug_assert_eq!(result.rows_affected(), 0); // name is UNIQUE
        bail!("Device '{}' not found", name);
    }
    Ok(())
}

pub async fn is_key_registered(conn: &mut PgConnection, key: &sign::PublicKey) -> Result<bool> {
    let record = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM device WHERE pending = FALSE AND pubkey = $1",
        key.as_ref()
    )
    .fetch_one(conn)
    .await?;
    Ok(matches!(record, Some(1)))
}
