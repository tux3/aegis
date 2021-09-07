use chrono::{NaiveDateTime, Utc};
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

#[derive(Copy, Clone)]
pub struct DeviceKey(pub sign::PublicKey);

impl From<sign::PublicKey> for DeviceKey {
    fn from(k: PublicKey) -> Self {
        Self(k)
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
    pub name: String,
    pub pubkey: DeviceKey,
}

impl Insert for PendingDevice {
    type Table = Device;

    fn insert(self, db: &mut PgConnection) -> BoxFuture<'_, sqlx::Result<Self::Table>> {
        Device::insert(
            db,
            InsertDevice {
                created_at: Utc::now().naive_utc(),
                name: self.name,
                pubkey: self.pubkey,
                pending: true,
            },
        )
    }
}

pub async fn count_pending(conn: &mut PgConnection) -> sqlx::Result<i64> {
    let record = sqlx::query!("SELECT COUNT(*) FROM device WHERE pending = TRUE")
        .fetch_one(conn)
        .await?;
    Ok(record.count.unwrap_or(0))
}
