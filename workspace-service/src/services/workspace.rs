use super::{
    team::{TeamId, TeamRepo},
    user::{AuthId, User, UserId, UserRepo},
};
use anyhow::Result;
use async_trait::async_trait;
use derive_more::{Display, From, Into};
use fnhs_event_models::{Event, WorkspaceCreatedData, WorkspaceMembershipChangedData};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub struct Workspace {
    pub id: WorkspaceId,
    pub title: String,
    pub description: String,
    pub admins: TeamId,
    pub members: TeamId,
}
#[derive(Copy, Clone)]
pub enum Role {
    /// User is a workspace administrator
    Admin,
    /// User is a workspace member
    NonAdmin,
    /// User is not a workspace member
    NonMember,
}

#[derive(Copy, Clone)]
pub enum RoleFilter {
    /// Only return Admins
    Admin,
    /// Only return Non-Admins
    NonAdmin,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Role::Admin => "Admin",
                Role::NonAdmin => "NonAdmin",
                Role::NonMember => "NonMember",
            }
        )
    }
}

#[derive(From, Into, Display, Copy, Clone)]
pub struct WorkspaceId(Uuid);

#[async_trait::async_trait]
pub trait WorkspaceRepo {
    async fn create<'c, E>(
        &self,
        title: &str,
        description: &str,
        admins_team_id: TeamId,
        members_team_id: TeamId,
    ) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>;

    async fn find_all<'c, E>(executor: E) -> Result<Vec<Workspace>>
    where
        E: Executor<'c, Database = Postgres>;

    async fn find_by_id<'c, E>(id: WorkspaceId, executor: E) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>;

    async fn update<'c, E>(
        id: WorkspaceId,
        title: &str,
        description: &str,
        executor: E,
    ) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>;

    async fn delete<'c, E>(id: WorkspaceId, executor: E) -> Result<Workspace>
    where
        E: Executor<'c, Database = Postgres>;
}

#[async_trait::async_trait]
pub trait EventRepo {}

#[async_trait]
pub trait WorkspaceService<'c> {
    async fn members(&self, filter: Option<RoleFilter>) -> Result<Vec<User>>;

    async fn create(
        &self,
        title: &str,
        description: &str,
        requesting_user: AuthId,
    ) -> Result<Workspace>;

    async fn is_admin(&self, workspace_id: WorkspaceId, user_id: UserId) -> Result<bool>;

    async fn change_workspace_membership(
        &self,
        workspace_id: WorkspaceId,
        user_id: UserId,
        new_role: Role,
        requesting_user: AuthId,
    ) -> Result<Workspace>;
}

#[derive(Clone)]
pub struct WorkspaceServiceImpl<T, U, W>
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    team_repo: T,
    user_repo: U,
    workspace_repo: W,
}

impl<'c, T, U, W> WorkspaceServiceImpl<T, U, W>
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    pub fn new(team_repo: T, user_repo: U, workspace_repo: W) -> Self {
        Self {
            team_repo,
            user_repo,
            workspace_repo,
        }
    }
}

#[async_trait]
impl<'c, T, U, W> WorkspaceService<'c> for WorkspaceServiceImpl<T, U, W>
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    async fn members(&self, filter: Option<RoleFilter>) -> Result<Vec<User>> {
        let users = match filter {
            Some(RoleFilter::Admin) => self.team_repo.members(self.admins, self.executor).await?,
            Some(RoleFilter::NonAdmin) => {
                self.team_repo
                    .members_difference(self.members, self.admins, self.executor)
                    .await?
            }
            None => self.team_repo.members(self.members, self.executor).await?,
        };
        Ok(users.into_iter().map(Into::into).collect())
    }

    async fn create(
        &self,
        title: &str,
        description: &str,
        requesting_user: AuthId,
    ) -> Result<Workspace> {
        let user = self
            .user_repo
            .find_by_auth_id(requesting_user)
            .await?
            .ok_or_else(|| anyhow::anyhow!("user not found"))?;
        if !user.is_platform_admin {
            return Err(anyhow::anyhow!(
                "User with auth_id {} does not have permission to create a workspace.",
                requesting_user,
            )
            .into());
        }
        let mut tx = self.executor.begin().await?;

        let admins = self
            .team_repo
            .create(&format!("{} Admins", title), &tx)
            .await?;
        let members = self
            .team_repo
            .create(&format!("{} Members", title), &tx)
            .await?;

        let workspace: Workspace = self.workspace_repo.create(title, description).await?.into();

        tx.commit().await?;

        self.event_client
            .publish_events(&[Event::new(
                workspace.id.to_string(),
                WorkspaceCreatedData {
                    workspace_id: workspace.id.to_string(),
                    user_id: requesting_user.to_string(),
                    title: workspace.title,
                },
            )])
            .await?;

        Ok(workspace)
    }

    async fn is_admin(&self, workspace_id: WorkspaceId, user_id: UserId) -> Result<bool> {
        match self.user_repo.find_by_id(&user_id, self.executor).await? {
            Some(user) => {
                let workspace = self
                    .workspace_repo
                    .find_by_id(workspace_id, self.executor)
                    .await?;
                self.team_repo
                    .is_member(workspace.admins, user.id, self.executor)
                    .await
            }
            None => Ok(false),
        }
    }

    async fn change_workspace_membership(
        &self,
        workspace_id: WorkspaceId,
        user_id: UserId,
        new_role: Role,
        requesting_user: AuthId,
    ) -> Result<Workspace> {
        let user = self
            .user_repo
            .find_by_auth_id(&requesting_user, self.executor)
            .await?
            .ok_or_else(|| anyhow::anyhow!("user not found"))?;

        if !user.is_platform_admin && !self.is_admin(workspace_id, user.id).await? {
            return Err(anyhow::anyhow!(
                "user with auth_id {} does not have permission to update workspace membership",
                user.auth_id,
            )
            .into());
        }

        if user.id == user_id {
            return Err(anyhow::anyhow!(
                "user with auth_id {} cannot demote themselves to {}",
                user.auth_id,
                new_role
            )
            .into());
        }

        let mut tx = self.executor.begin().await?;

        let workspace = self.workspace_repo.find_by_id(workspace_id, tx).await?;

        match new_role {
            Role::Admin => {
                self.team_repo
                    .add_member(workspace.admins, user_id, &mut tx)
                    .await?;
                self.team_repo
                    .add_member(workspace.members, user_id, &mut tx)
                    .await?;
            }
            Role::NonAdmin => {
                self.team_repo
                    .remove_member(workspace.admins, user_id, &mut tx)
                    .await?;
                self.team_repo
                    .add_member(workspace.members, user_id, &mut tx)
                    .await?;
            }
            Role::NonMember => {
                self.team_repo
                    .remove_member(workspace.admins, user_id, &mut tx)
                    .await?;
                self.team_repo
                    .remove_member(workspace.members, user_id, &mut tx)
                    .await?;
            }
        }

        tx.commit().await?;

        self.event_client
            .publish_events(&[Event::new(
                workspace.id.clone(),
                WorkspaceMembershipChangedData {
                    requesting_user_id: requesting_user.auth_id.to_string(),
                    affected_workspace_id: workspace.id.clone().into(),
                    affected_user_id: user_id.to_string(),
                    affected_role: new_role.to_string(),
                },
            )])
            .await?;

        Ok(workspace)
    }
}
