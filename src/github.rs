use octocrab::Octocrab;
use tokio;

use crate::notifications::{Notification, Status};

#[tokio::main]
pub async fn get_github_notifications() -> octocrab::Result<Vec<Notification>> {
    let token =
        std::env::var("GHN_GITHUB_TOKEN").expect("GHN_GITHUB_TOKEN env variable is required");

    let octocrab = Octocrab::builder().personal_token(token).build().unwrap();

    let mut notifications: Vec<Notification> = Vec::new();
    let mut current_page = octocrab
        .activity()
        .notifications()
        .list()
        .per_page(100)
        .all(true)
        .send()
        .await?;

    let mut gh_notifications = current_page.take_items();

    while let Ok(Some(mut new_page)) = octocrab.get_page(&current_page.next).await {
        gh_notifications.extend(new_page.take_items());
        current_page = new_page;
    }

    for n in gh_notifications {
        notifications.push(Notification {
            title: n.subject.title,
            body: String::from(""),
            github_type: n.subject.r#type,
            reason: n.reason,
            repo: n.repository.full_name.unwrap_or(String::from("no-name")),
            updated_at: n.updated_at,
            status: Status::Unread,
            url: String::from(n.subject.url.unwrap()),
        });
    }
    notifications.sort_by_key(|x| x.updated_at);
    Ok(notifications)
}
