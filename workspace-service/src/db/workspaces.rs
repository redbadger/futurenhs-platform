// sqlx::query_file_as!() causes spurious errors with this lint enabled
#![allow(clippy::suspicious_else_formatting)]

use crate::services::{
    team::TeamId,
    workspace::{Workspace, WorkspaceId, WorkspaceRepo},
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{types::Uuid, Executor, PgConnection, Postgres};
use std::sync::Arc;

#[derive(Clone)]
pub struct DbWorkspace {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub admins: Uuid,
    pub members: Uuid,
}

impl From<DbWorkspace> for Workspace {
    fn from(_: DbWorkspace) -> Self {
        todo!()
    }
}

#[derive(Clone)]
pub struct WorkspaceRepoImpl {
    connection: Arc<std::sync::Mutex<PgConnection>>,
}

#[async_trait]
impl WorkspaceRepo for WorkspaceRepoImpl {
    async fn create(
        &self,
        title: &str,
        description: &str,
        admins_team_id: TeamId,
        members_team_id: TeamId,
    ) -> Result<Workspace> {
        let mut tx = &mut *self.connection;
        let admins_team_id: Uuid = admins_team_id.into();
        let members_team_id: Uuid = members_team_id.into();
        let workspace = sqlx::query_file_as!(
            DbWorkspace,
            "sql/workspaces/create.sql",
            title,
            description,
            admins_team_id,
            members_team_id
        )
        .fetch_one(tx)
        .await
        .context("create workspace")?
        .into();

        Ok(workspace)
    }

    async fn find_all<'c, E>(executor: E) -> Result<Vec<Workspace>>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let workspaces: Vec<DbWorkspace> =
            sqlx::query_file_as!(DbWorkspace, "sql/workspaces/find_all.sql")
                .fetch_all(executor)
                .await
                .context("find all workspaces")?;

        Ok(workspaces.iter().cloned().map(Into::into).collect())
    }

    async fn find_by_id<'c, E>(id: WorkspaceId, executor: E) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let id: Uuid = id.into();
        let workspace = sqlx::query_file_as!(DbWorkspace, "sql/workspaces/find_by_id.sql", id)
            .fetch_one(executor)
            .await
            .context("find a workspace by id")?
            .into();

        Ok(workspace)
    }

    async fn update<'c, E>(
        id: WorkspaceId,
        title: &str,
        description: &str,
        executor: E,
    ) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let id: Uuid = id.into();
        let workspace = sqlx::query_file_as!(
            DbWorkspace,
            "sql/workspaces/update.sql",
            id,
            title,
            description
        )
        .fetch_one(executor)
        .await
        .context("update workspace")?
        .into();

        Ok(workspace)
    }

    async fn delete<'c, E>(id: WorkspaceId, executor: E) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>,
    {
        let id: Uuid = id.into();
        let workspace = sqlx::query_file_as!(DbWorkspace, "sql/workspaces/delete.sql", id)
            .fetch_one(executor)
            .await
            .context("delete workspace")?
            .into();

        Ok(workspace)
    }
}
