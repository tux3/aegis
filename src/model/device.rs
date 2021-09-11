use aegislib::command::admin::SetStatusArg;
use aegislib::command::device::StatusReply;
use anyhow::{bail, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::future::BoxFuture;
use ormx::{Insert, Table};
use sqlx::{Connection, PgConnection};

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
            created_at: DateTime::<Utc>::from_utc(dev.created_at, Utc).into(),
            name: dev.name,
            pubkey: dev.pubkey,
        }
    }
}

impl From<Device> for aegislib::command::admin::RegisteredDevice {
    fn from(dev: Device) -> Self {
        Self {
            id: dev.id,
            created_at: DateTime::<Utc>::from_utc(dev.created_at, Utc).into(),
            name: dev.name,
            pubkey: dev.pubkey,
        }
    }
}

#[derive(ormx::Table, sqlx::FromRow)]
#[ormx(table = "device_status", id = dev_id, insertable)]
pub struct Status {
    dev_id: i32,
    updated_at: NaiveDateTime,
    #[ormx(default)]
    vt_locked: bool,
    #[ormx(default)]
    ssh_locked: bool,
}

impl From<Status> for StatusReply {
    fn from(s: Status) -> Self {
        Self {
            vt_locked: s.vt_locked,
            ssh_locked: s.ssh_locked,
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
            pubkey: r.pubkey,
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
    let mut tx = conn.begin().await?;
    let result = sqlx::query!(
        "UPDATE device SET pending = FALSE WHERE name = $1 AND pending = TRUE
         RETURNING id",
        name
    )
    .fetch_one(&mut tx)
    .await?;
    Status::insert(
        &mut tx,
        InsertStatus {
            dev_id: result.id,
            updated_at: Utc::now().naive_utc(),
        },
    )
    .await?;
    tx.commit().await?;
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
            pubkey: r.pubkey,
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

pub async fn get_dev_id(conn: &mut PgConnection, pubkey: &ed25519_dalek::PublicKey) -> Result<i32> {
    let pubkey = base64::encode_config(pubkey.as_ref(), base64::URL_SAFE_NO_PAD);
    let id = sqlx::query_scalar!(
        "SELECT id FROM device WHERE pending = FALSE AND pubkey = $1",
        pubkey
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

pub async fn update_status(conn: &mut PgConnection, status: &SetStatusArg) -> Result<Status> {
    let mut fields = Vec::new();
    if let Some(val) = status.vt_locked {
        fields.push(format!("vt_locked = {}", val));
    }
    if let Some(val) = status.ssh_locked {
        fields.push(format!("ssh_locked = {}", val));
    }
    let fields = fields.join(",");
    let query = &format!("UPDATE device SET {} WHERE dev_id = $1 RETURNING *", fields);
    let result = sqlx::query_as::<_, Status>(query)
        .bind(status.dev_id)
        .fetch_one(conn)
        .await?;
    Ok(result)
}

pub async fn get_status(conn: &mut PgConnection, dev_id: i32) -> Result<Status> {
    let result = sqlx::query_as!(
        Status,
        "SELECT * FROM device_status WHERE dev_id = $1",
        dev_id
    )
    .fetch_one(conn)
    .await?;
    Ok(result)
}
