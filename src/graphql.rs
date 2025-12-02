use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject, Union};
use std::sync::Arc;
use crate::search::SearchIndex;
use crate::storage::Storage;
use crate::schemas::{Issue, Task, Comment, Planning, Document, IssueStatus, PlanningStatus};

pub struct QueryRoot;

#[derive(Union)]
pub enum Resource {
    Issue(GqlIssue),
    Task(GqlTask),
    Comment(GqlComment),
    Planning(GqlPlanning),
    Document(GqlDocument),
}

#[derive(SimpleObject)]
pub struct GqlIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub assignee: Option<String>,
    pub resolution: Option<String>,
    pub involved: Option<Vec<String>>,
}

#[derive(SimpleObject)]
pub struct GqlTask {
    pub id: String,
    pub cta: String,
    pub description: String,
    pub url: String,
    pub completed: bool,
    pub deadline: Option<String>,
}

#[derive(SimpleObject)]
pub struct GqlComment {
    pub id: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub mentions: Option<Vec<String>>,
}

#[derive(SimpleObject)]
pub struct GqlPlanning {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub moments: Vec<crate::schemas::PlanningMoment>,
}

#[derive(SimpleObject)]
pub struct GqlDocument {
    pub id: String,
    pub title: String,
    pub url: String,
    pub size: u64,
}

#[Object]
impl QueryRoot {
    async fn search(&self, ctx: &Context<'_>, query: String) -> async_graphql::Result<Vec<Resource>> {
        let search_index = ctx.data::<Arc<SearchIndex>>()?;
        let storage = ctx.data::<Arc<Storage>>()?;

        let results = search_index.search(storage, &query, 10).await?;

        let graphql_results = results.into_iter().filter_map(|r| {
            if let Some(json) = r.resource {
                let id = r.id;
                match r.doc_type.as_str() {
                    "issue" => serde_json::from_value::<Issue>(json).ok().map(|x| Resource::Issue(GqlIssue {
                        id,
                        title: x.title,
                        description: x.description,
                        status: x.status,
                        assignee: x.assignee,
                        resolution: x.resolution,
                        involved: x.involved,
                    })),
                    "task" => serde_json::from_value::<Task>(json).ok().map(|x| Resource::Task(GqlTask {
                        id,
                        cta: x.cta,
                        description: x.description,
                        url: x.url,
                        completed: x.completed,
                        deadline: x.deadline,
                    })),
                    "comment" => serde_json::from_value::<Comment>(json).ok().map(|x| Resource::Comment(GqlComment {
                        id,
                        content: x.content,
                        parent_id: x.parent_id,
                        mentions: x.mentions,
                    })),
                    "planning" => serde_json::from_value::<Planning>(json).ok().map(|x| Resource::Planning(GqlPlanning {
                        id,
                        title: x.title,
                        description: x.description,
                        moments: x.moments,
                    })),
                    "document" => serde_json::from_value::<Document>(json).ok().map(|x| Resource::Document(GqlDocument {
                        id,
                        title: x.title,
                        url: x.url,
                        size: x.size,
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }).collect();

        Ok(graphql_results)
    }
}

pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn create_schema(search_index: Arc<SearchIndex>, storage: Arc<Storage>) -> AppSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(search_index)
        .data(storage)
        .finish()
}
