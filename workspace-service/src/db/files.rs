// sqlx::query_file_as!() causes spurious errors with this lint enabled
#![allow(clippy::suspicious_else_formatting)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{types::Uuid, PgPool};

#[derive(Clone)]
pub struct File {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub latest_version: Uuid
}

impl File {
    pub async fn create(
        created_by: &Uuid,
        latest_version: &Uuid,
        pool: &PgPool,
    ) -> Result<File> {
        let file = sqlx::query_file_as!(
            File,
            "sql/files/create.sql",
            created_by,
            latest_version
        )
        .fetch_one(pool)
        .await?;

        Ok(file)
    }

    pub async fn find_by_folder(folder: Uuid, pool: &PgPool) -> Result<Vec<File>> {
        let files = sqlx::query_file_as!(File, "sql/files/find_by_folder.sql", folder)
            .fetch_all(pool)
            .await?;

        Ok(files)
    }

    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<File> {
        let file = sqlx::query_file_as!(File, "sql/files/find_by_id.sql", id)
            .fetch_one(pool)
            .await?;

        Ok(file)
    }

    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<File> {
        let file = sqlx::query_file_as!(File, "sql/files/delete.sql", id)
            .fetch_one(pool)
            .await?;

        Ok(file)
    }
}
