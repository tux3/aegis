use aegislib::command::admin::StoredCameraPicture;
use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use sqlx::PgConnection;

#[derive(sqlx::FromRow)]
pub struct DeviceCameraPicture {
    pub id: i32,
    pub dev_id: i32,
    pub created_at: NaiveDateTime,
    pub jpeg_data: Vec<u8>,
}

impl From<DeviceCameraPicture> for StoredCameraPicture {
    fn from(p: DeviceCameraPicture) -> Self {
        StoredCameraPicture {
            created_at_timestamp: p.created_at.timestamp() as u64,
            jpeg_data: p.jpeg_data,
        }
    }
}

impl DeviceCameraPicture {
    pub async fn insert(self, db: &mut PgConnection) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO device_cam_pics (dev_id, created_at, jpeg_data)
             VALUES ($1, $2, $3)",
            self.dev_id,
            self.created_at,
            self.jpeg_data,
        )
        .execute(db)
        .await?;
        Ok(())
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
