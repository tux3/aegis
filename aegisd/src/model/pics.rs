use aegislib::command::admin::StoredCameraPicture;
use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use sqlx::PgConnection;

#[derive(ormx::Table, sqlx::FromRow)]
#[ormx(table = "device_cam_pics", id = dev_id, insertable)]
pub struct DeviceCameraPicture {
    #[ormx(default)]
    id: i32,
    dev_id: i32,
    created_at: NaiveDateTime,
    jpeg_data: Vec<u8>,
}

impl From<DeviceCameraPicture> for StoredCameraPicture {
    fn from(p: DeviceCameraPicture) -> Self {
        StoredCameraPicture {
            created_at_timestamp: p.created_at.timestamp() as u64,
            jpeg_data: p.jpeg_data,
        }
    }
}

pub async fn get_for_device(
    conn: &mut PgConnection,
    dev_id: i32,
) -> Result<Vec<DeviceCameraPicture>> {
    let record = sqlx::query_as!(
        DeviceCameraPicture,
        "SELECT * FROM device_cam_pics WHERE dev_id = $1",
        dev_id
    )
    .fetch_all(conn)
    .await?;
    Ok(record)
}

pub async fn delete_for_device(conn: &mut PgConnection, dev_id: i32) -> Result<()> {
    let result = sqlx::query!("DELETE FROM device_cam_pics WHERE dev_id = $1", dev_id)
        .execute(conn)
        .await?;
    if result.rows_affected() == 0 {
        bail!("Device {} has no stored camera pictures", dev_id);
    }
    Ok(())
}
