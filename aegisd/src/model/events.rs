use aegislib::command::device::{DeviceEvent, EventLogLevel};
use anyhow::{bail, Result};
use chrono::{DateTime, NaiveDateTime};
use sqlx::PgConnection;

#[derive(Copy, Clone, Debug, Eq, PartialEq, sqlx::Type)]
#[sqlx(type_name = "event_log_level", rename_all = "snake_case")]
enum DbEventLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<DbEventLogLevel> for EventLogLevel {
    fn from(e: DbEventLogLevel) -> Self {
        match e {
            DbEventLogLevel::Trace => Self::Trace,
            DbEventLogLevel::Debug => Self::Debug,
            DbEventLogLevel::Info => Self::Info,
            DbEventLogLevel::Warn => Self::Warn,
            DbEventLogLevel::Error => Self::Error,
        }
    }
}

impl From<EventLogLevel> for DbEventLogLevel {
    fn from(e: EventLogLevel) -> Self {
        match e {
            EventLogLevel::Trace => Self::Trace,
            EventLogLevel::Debug => Self::Debug,
            EventLogLevel::Info => Self::Info,
            EventLogLevel::Warn => Self::Warn,
            EventLogLevel::Error => Self::Error,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DbDeviceEvent {
    #[sqlx(default)]
    #[allow(unused)]
    id: i32,
    #[allow(unused)]
    dev_id: i32,
    created_at: NaiveDateTime,
    level: DbEventLogLevel,
    message: String,
}

impl From<DbDeviceEvent> for DeviceEvent {
    fn from(e: DbDeviceEvent) -> Self {
        Self {
            timestamp: e.created_at.and_utc().timestamp() as u64,
            level: e.level.into(),
            message: e.message,
        }
    }
}

pub async fn insert(conn: &mut PgConnection, dev_id: i32, event: DeviceEvent) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO device_event (dev_id, created_at, level, message) VALUES ($1, $2, $3, $4)"#,
        dev_id,
        DateTime::from_timestamp(event.timestamp as i64, 0)
            .unwrap()
            .naive_utc()
            .into(),
        DbEventLogLevel::from(event.level) as _,
        &event.message
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get_for_device(conn: &mut PgConnection, dev_id: i32) -> Result<Vec<DeviceEvent>> {
    let record = sqlx::query_as!(
        DbDeviceEvent,
        r#"SELECT id, dev_id, created_at, level as "level: _", message FROM device_event WHERE dev_id = $1"#,
        dev_id
    )
        .fetch_all(conn)
        .await?;
    Ok(record.into_iter().map(Into::into).collect())
}

pub async fn delete_for_device(conn: &mut PgConnection, dev_id: i32) -> Result<()> {
    let result = sqlx::query!("DELETE FROM device_event WHERE dev_id = $1", dev_id)
        .execute(conn)
        .await?;
    if result.rows_affected() == 0 {
        bail!("Device {} has no stored events", dev_id);
    }
    Ok(())
}
