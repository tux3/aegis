use anyhow::{bail, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::future::BoxFuture;
use ormx::{Insert, Table};
use sqlx::PgConnection;

#[derive(ormx::Table)]
#[ormx(table = "device", id = id, insertable)]
pub struct Device {
    #[ormx(default)]
    id: i32,
    created_at: NaiveDateTime,
    name: String,
    pubkey: String,
    pending: bool,
}

pub struct PendingDevice {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub pubkey: String,
}

impl Insert for PendingDevice {
    type Table = Device;

    fn insert(self, db: &mut PgConnection) -> BoxFuture<'_, sqlx::Result<Self::Table>> {
        Device::insert(
            db,
            InsertDevice {
                created_at: self.created_at.into(),
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
            created_at: DateTime::<Utc>::from_utc(dev.created_at, Utc).into(),
            name: dev.name,
            pubkey: dev.pubkey,
        }
    }
}

impl From<Device> for aegislib::command::admin::RegisteredDevice {
    fn from(dev: Device) -> Self {
        Self {
            created_at: DateTime::<Utc>::from_utc(dev.created_at, Utc).into(),
            name: dev.name,
            pubkey: dev.pubkey,
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

pub async fn is_key_registered(
    conn: &mut PgConnection,
    key: &ed25519_dalek::PublicKey,
) -> Result<bool> {
    let key = base64::encode_config(key.as_ref(), base64::URL_SAFE_NO_PAD);
    let record = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM device WHERE pending = FALSE AND pubkey = $1",
        key
    )
    .fetch_one(conn)
    .await?;
    Ok(matches!(record, Some(1)))
}
