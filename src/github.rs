use std::collections::HashMap;

use octocrab::models::activity;
use octocrab::models::CommentId;
use octocrab::models::IssueState;
use octocrab::Octocrab;
use tokio;

use crate::notifications::{Comment, Notification, NotificationDetail, Repo, Status};

#[tokio::main]
pub async fn get_notifications(
    current: Option<&Vec<Notification>>,
) -> octocrab::Result<Vec<Notification>> {
    let token =
        std::env::var("GHN_GITHUB_TOKEN").expect("GHN_GITHUB_TOKEN env variable is required");
    let octocrab = Octocrab::builder()
        .personal_token(token)
        .build()
        .expect("octocrab client");
    let mut notifications: Vec<Notification> = Vec::new();
    let mut current_page = octocrab
        .activity()
        .notifications()
        .list()
        .per_page(100)
        .all(false)
        .send()
        .await?;

    let mut gh_notifications = current_page.take_items();

    while let Ok(Some(mut new_page)) = octocrab.get_page(&current_page.next).await {
        gh_notifications.extend(new_page.take_items());
        current_page = new_page;
    }

    let mut existing_notifications: HashMap<u64, Notification> = HashMap::new();
    for n in current.unwrap_or(&vec![]) {
        existing_notifications.insert(n.id, n.clone());
    }

    let mut handles = Vec::new();

    for n in gh_notifications {
        let n = n.clone();
        let o = octocrab.clone();
        let mut new_n = Notification::from(&n);
        let details: Option<NotificationDetail> = match existing_notifications.get(&n.id) {
            None => None,
            Some(n) => {
                if new_n.updated_at > n.updated_at {
                    None
                } else {
                    Some(n.details.clone().unwrap())
                }
            }
        };

        match details {
            Some(d) => {
                new_n.details = Ok(d);
                notifications.push(new_n);
            }
            None => {
                handles.push(tokio::spawn(async move {
                    new_n.details = hydrate_notification(&new_n, &o).await;
                    new_n
                }));
            }
        }
    }
    for handle in handles {
        let new_n = handle.await.unwrap();
        notifications.push(new_n);
    }

    notifications.sort_by_key(|x| x.updated_at);
    Ok(notifications)
}

// converts something like https://api.github.com/repos/foo/bla/pulls/1234
// into https://github.com/foo/bla/pull/1234
fn convert_to_html_url(url: String) -> Result<String, String> {
    let ret = url
        .replace("/repos/", "/")
        .replace("/pulls/", "/pull/")
        .replace("api.github.com", "github.com");

    return Ok(ret);
}
impl From<&activity::Notification> for Notification {
    fn from(n: &activity::Notification) -> Self {
        let subject_url = match n.subject.url.as_ref() {
            Some(v) => format!("{}", v),
            None => String::from(""),
        };
        return Notification {
            id: *n.id,
            title: format!("{}", n.subject.title),
            github_type: format!("{}", n.subject.r#type),
            reason: format!("{}", n.reason),
            repo: Repo {
                nwo: format!(
                    "{}",
                    n.repository
                        .full_name
                        .as_ref()
                        .unwrap_or(&String::from("no-name"))
                ),
                owner: format!("{}", n.repository.owner.as_ref().unwrap().login),
                name: format!("{}", n.repository.name),
            },
            latest_comment_url: match n.subject.latest_comment_url.as_ref() {
                None => None,
                Some(v) => {
                    if format!("{v}") == format!("{subject_url}") {
                        None
                    } else {
                        Some(format!("{v}"))
                    }
                }
            },
            updated_at: n.updated_at,
            status: Status::Unread,
            url: convert_to_html_url(format!("{subject_url}")).unwrap(),
            details: Err(String::from("Not yet retrieved")),
        };
    }
}

async fn hydrate_notification(
    n: &Notification,
    o: &Octocrab,
) -> Result<NotificationDetail, String> {
    let mut detail = NotificationDetail::default();
    detail.url = String::from(&n.url);
    if n.github_type == String::from("Issue") {
        let issue_number = &n.url.split("/").last().unwrap().parse::<u64>().unwrap();
        match o
            .issues(&n.repo.owner, &n.repo.name)
            .get(*issue_number)
            .await
        {
            Err(e) => {
                return Err(format!(
                    "unable to retrieve issue {}/{}#{}: {e:#?}",
                    &n.repo.owner, &n.repo.name, *issue_number
                ));
            }
            Ok(v) => {
                detail.url = String::from(v.html_url);
                detail.author = v.user.login;
                detail.latest_comment = get_comment_details(&n, &o).await;
                match v.state {
                    IssueState::Open => detail.state = String::from("open"),
                    IssueState::Closed => detail.state = String::from("closed"),
                    _ => detail.state = String::from("n/a"),
                }
            }
        }
    }
    if n.github_type == String::from("PullRequest") {
        let pr_number = &n.url.split("/").last().unwrap().parse::<u64>().unwrap();
        match o.pulls(&n.repo.owner, &n.repo.name).get(*pr_number).await {
            Err(e) => {
                return Err(format!(
                    "unable to retrieve PR {}/{}#{}: {e:#?}",
                    &n.repo.owner, &n.repo.name, *pr_number
                ));
            }
            Ok(v) => {
                let author = match v.user {
                    Some(u) => u.login,
                    None => format!("nologin"),
                };
                let html_url = match v.html_url {
                    Some(u) => format!("{u}"),
                    None => format!("no html_url found"),
                };
                detail.url = format!("{html_url}");
                detail.author = format!("{author}");
                match &n.latest_comment_url {
                    Some(_) => detail.latest_comment = get_comment_details(&n, &o).await,
                    None => {
                        detail.latest_comment = Some(Comment {
                            author: format!("{author}"),
                            body: v.body.unwrap_or(format!("No body found for {html_url}")),
                            url: format!("{html_url}"),
                        });
                    }
                }
                match v.state {
                    Some(IssueState::Open) => detail.state = String::from("open"),
                    Some(IssueState::Closed) => detail.state = String::from("closed"),
                    _ => detail.state = String::from("n/a"),
                }
            }
        }
    }

    Ok(detail)
}

async fn get_comment_details(n: &Notification, o: &Octocrab) -> Option<Comment> {
    if let Some(url) = &n.latest_comment_url {
        let comment_id = url.split("/").last().unwrap().parse::<u64>().unwrap();
        let mut ret = Comment::default();
        match o
            .issues(&n.repo.owner, &n.repo.name)
            .get_comment(CommentId(comment_id))
            .await
        {
            Err(e) => {
                ret.body = format!("failed to retrieve comment from {}\n{e:#?}", url);
            }
            Ok(v) => {
                ret.body = v.body.unwrap_or(format!("no body information"));
                ret.author = v.user.login;
                ret.url = String::from(v.html_url);
            }
        }
        return Some(ret);
    }
    None
}

#[tokio::main]
pub async fn update_state(notifications: &Vec<Notification>) -> Result<(), String> {
    let mut handles = Vec::new();
    let mut ret: Result<(), String> = Ok(());

    for n in notifications {
        let n = n.clone();
        handles.push(tokio::spawn(async move { update_thread_state(&n).await }));
    }

    for handle in handles {
        if let Err(e) = handle.await.unwrap() {
            ret = Err(format!("unable to update thread: {e}"));
        }
    }

    return ret;
}

async fn update_thread_state(n: &Notification) -> Result<(), String> {
    match n.status {
        Status::Read => {
            let token = std::env::var("GHN_GITHUB_TOKEN")
                .expect("GHN_GITHUB_TOKEN env variable is required");
            let octocrab = Octocrab::builder()
                .personal_token(token)
                .build()
                .expect("octocrab client");
            match octocrab
                .activity()
                .notifications()
                .mark_as_read(octocrab::models::NotificationId(n.id))
                .await
            {
                Err(e) => Err(format!("Failed to mark thread '{}, as read: {}", &n.url, e)),
                Ok(_) => Ok(()),
            }
        }
        Status::Done => {
            let token = std::env::var("GHN_GITHUB_TOKEN")
                .expect("GHN_GITHUB_TOKEN env variable is required");
            let octocrab = Octocrab::builder()
                .personal_token(token)
                .build()
                .expect("octocrab client");

            let url = format!("https://api.github.com/notifications/threads/{}", n.id);
            match octocrab._delete(url, None::<&()>).await {
                Err(e) => Err(format!(
                    "Failed to mark thread '{}, as done: {}",
                    String::from(&n.url),
                    e
                )),
                Ok(_) => Ok(()),
            }
        }
        Status::Unread => Ok(()),
    }
}
