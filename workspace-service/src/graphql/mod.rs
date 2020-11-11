mod file_download_urls;
mod file_upload_urls;
mod files;
mod folders;
mod schema;
mod tracing_ext;
mod users;
mod validation;
mod workspaces;

use super::{azure, db};
use crate::services::{
    team::TeamRepo,
    user::UserRepo,
    workspace::{WorkspaceRepo, WorkspaceServiceImpl},
};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, MergedObject, Schema,
};
use sqlx::PgPool;
use tide::{http::mime, Request, Response, StatusCode};
use uuid::Uuid;

#[derive(Clone)]
pub struct State<T, U, W>
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    schema: Schema<Query, Mutation, EmptySubscription>,
    workspace_service: WorkspaceServiceImpl<T, U, W>,
}

impl<T, U, W> State<T, U, W>
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    pub fn new(
        pool: PgPool,
        azure_config: azure::Config,
        workspace_service: WorkspaceServiceImpl<T, U, W>,
    ) -> Self {
        State {
            schema: Schema::build(Query::default(), Mutation::default(), EmptySubscription)
                .extension(tracing_ext::Tracing)
                .data(pool)
                .data(azure_config)
                .finish(),
            workspace_service,
        }
    }
}

#[derive(MergedObject, Default)]
struct Query(
    files::FilesQuery,
    folders::FoldersQuery,
    workspaces::WorkspacesQuery,
);

#[derive(MergedObject, Default)]
struct Mutation(
    file_download_urls::FileDownloadUrlsMutation,
    file_upload_urls::FileUploadUrlsMutation,
    files::FilesMutation,
    folders::FoldersMutation,
    workspaces::WorkspacesMutation,
    users::UsersMutation,
);

#[derive(Debug)]
pub struct RequestingUser {
    auth_id: Uuid,
}

pub async fn handle_healthz<T, U, W>(req: Request<State<T, U, W>>) -> tide::Result
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    // let response = if !req.state().event_client.is_configured() {
    //     Response::builder(500).body("invalid event client").build()
    // } else {
    //     Response::new(204)
    // };

    let response = Response::new(204);
    Ok(response)
}

pub async fn handle_graphql<T, U, W>(req: Request<State<T, U, W>>) -> tide::Result
where
    T: TeamRepo + Clone + Send + Sync + 'static,
    U: UserRepo + Clone + Send + Sync + 'static,
    W: WorkspaceRepo + Clone + Send + Sync + 'static,
{
    let schema = req.state().schema.clone();
    let auth_id = req
        .header("x-user-auth-id")
        .and_then(|values| values.get(0))
        .and_then(|value| Uuid::parse_str(value.as_str()).ok());

    let mut req = async_graphql_tide::receive_request(req).await?;
    if let Some(auth_id) = auth_id {
        req = req.data(RequestingUser { auth_id });
    }

    async_graphql_tide::respond(schema.execute(req).await)
}

pub async fn handle_graphiql<T, U, W>(_: Request<State<T, U, W>>) -> tide::Result
where
    T: TeamRepo,
    U: UserRepo,
    W: WorkspaceRepo,
{
    let response = Response::builder(StatusCode::Ok)
        .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
        .content_type(mime::HTML)
        .build();

    Ok(response)
}

pub async fn generate_graphql_schema() -> anyhow::Result<String> {
    let schema = Schema::new(Query::default(), Mutation::default(), EmptySubscription);
    schema::generate_introspection_schema(&schema).await
}
