use octocrab::Octocrab;
use tokio;

use crate::notifications::{Notification, Status};

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
            body: String::from(""),
            github_type: n.subject.r#type,
            reason: n.reason,
            repo: n.repository.full_name.unwrap_or(String::from("no-name")),
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
