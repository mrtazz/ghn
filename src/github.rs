use octocrab::models::IssueState;
use octocrab::Octocrab;
use tokio;

use crate::notifications::{Notification, NotificationDetail, Repo, Status};

#[tokio::main]
pub async fn get_notifications() -> octocrab::Result<Vec<Notification>> {
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

    for n in gh_notifications {
        notifications.push(Notification {
            id: *n.id,
            title: n.subject.title,
            github_type: n.subject.r#type,
            reason: n.reason,
            repo: Repo {
                nwo: n.repository.full_name.unwrap_or(String::from("no-name")),
                owner: n.repository.owner.unwrap().login,
                name: n.repository.name,
            },
            latest_comment_url: Some(String::from("")),
            updated_at: n.updated_at,
            status: Status::Unread,
            url: convert_to_html_url(String::from(n.subject.url.unwrap())).unwrap(),
        });
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

#[tokio::main]
pub async fn hydrate_notification(n: &Notification) -> Result<NotificationDetail, String> {
    let token =
        std::env::var("GHN_GITHUB_TOKEN").expect("GHN_GITHUB_TOKEN env variable is required");
    let octocrab = Octocrab::builder()
        .personal_token(token)
        .build()
        .expect("octocrab client");
    let mut detail = NotificationDetail::default();
    detail.url = String::from(&n.url);
    if n.github_type == String::from("Issue") {
        let issue_number = &n.url.split("/").last().unwrap().parse::<u64>().unwrap();
        match octocrab
            .issues(&n.repo.owner, &n.repo.name)
            .get(*issue_number)
            .await
        {
            Err(e) => {
                return Err(format!("unable to retrieve issue: {}", e));
            }
            Ok(v) => {
                detail.url = String::from(v.html_url);
                detail.author = v.user.login;
                match v.state {
                    IssueState::Open => detail.state = String::from("open"),
                    IssueState::Closed => detail.state = String::from("closed"),
                    _ => detail.state = String::from("n/a"),
                }
            }
        }
    }

    Ok(detail)
}

pub fn update_state(notifications: &Vec<Notification>) -> Result<(), String> {
    return notifications.iter().map(update_thread_state).collect();
}

#[tokio::main]
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
