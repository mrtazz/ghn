use chrono::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    Unread,
    Read,
    Done,
}

#[derive(Debug)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub repo: String,
    pub url: String,
    pub github_type: String,
    pub reason: String,
    pub status: Status,
    pub updated_at: chrono::DateTime<Utc>,
}
