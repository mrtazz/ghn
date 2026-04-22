use chrono::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    Unread,
    Read,
    Done,
}

#[derive(Clone, Debug)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub nwo: String,
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u64,
    pub title: String,
    pub repo: Repo,
    pub url: String,
    pub latest_comment_url: Option<String>,
    pub github_type: String,
    pub reason: String,
    pub status: Status,
    pub updated_at: chrono::DateTime<Utc>,
    pub details: Result<NotificationDetail, String>,
}

#[derive(Clone, Debug, Default)]
pub struct Comment {
    pub body: String,
    pub author: String,
    pub url: String,
}

#[derive(Clone, Debug, Default)]
pub struct NotificationDetail {
    pub state: String,
    pub latest_comment: Option<Comment>,
    pub url: String,
    pub author: String,
}
