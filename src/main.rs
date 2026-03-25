/// A Ratatui example that demonstrates how to create a todo list with selectable items.
///
/// This example runs with the Ratatui library code in the branch that you are currently
/// reading. See the [`latest`] branch for the code which works with the most recent Ratatui
/// release.
///
/// [`latest`]: https://github.com/ratatui/ratatui/tree/latest
use chrono::prelude::*;
use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEvent};
use octocrab::Octocrab;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::palette::tailwind::{BLUE, GREEN, SLATE};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
    StatefulWidget, Widget, Wrap,
};
use ratatui::{symbols, DefaultTerminal};
use tokio;

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;
const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::default().run(terminal))
}

/// This struct holds the current state of the app. In particular, it has the `todo_list` field
/// which is a wrapper around `ListState`. Keeping track of the state lets us render the
/// associated widget with its state and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events. Check
/// the drawing logic for items on how to specify the highlighting style for selected items.
struct App {
    should_exit: bool,
    notifications_list: NotificationList,
}

struct NotificationList {
    items: Vec<Notification>,
    state: ListState,
}

#[derive(Debug)]
struct Notification {
    title: String,
    body: String,
    repo: String,
    url: String,
    github_type: String,
    reason: String,
    status: Status,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Status {
    Unread,
    Read,
    Done,
}

#[tokio::main]
async fn get_github_notifications() -> octocrab::Result<Vec<Notification>> {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");

    let octocrab = Octocrab::builder().personal_token(token).build().unwrap();

    let mut notifications: Vec<Notification> = Vec::new();
    let gh_notifications = octocrab
        .activity()
        .notifications()
        .list()
        .all(true)
        .send()
        .await?;
    for n in gh_notifications {
        notifications.push(Notification {
            title: n.subject.title,
            body: String::from(""),
            github_type: n.subject.r#type,
            reason: n.reason,
            repo: n.repository.full_name.unwrap_or(String::from("no-name")),
            updated_at: n.updated_at,
            status: Status::Unread,
            url: String::from(n.url),
        });
    }
    Ok(notifications)
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_exit: false,
            notifications_list: NotificationList {
                items: get_github_notifications().unwrap(),
                state: ListState::default(),
            },
        }
    }
}

//impl Notification {
//    fn new(status: Status, title: &str, body: &str) -> Self {
//        Self {
//            status,
//            title: title.to_string(),
//            body: body.to_string(),
//        }
//    }
//}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('h') | KeyCode::Left => self.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.select_last(),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                self.toggle_status();
            }
            _ => {}
        }
    }

    const fn select_none(&mut self) {
        self.notifications_list.state.select(None);
    }

    fn select_next(&mut self) {
        self.notifications_list.state.select_next();
    }
    fn select_previous(&mut self) {
        self.notifications_list.state.select_previous();
    }

    const fn select_first(&mut self) {
        self.notifications_list.state.select_first();
    }

    const fn select_last(&mut self) {
        self.notifications_list.state.select_last();
    }

    /// Changes the status of the selected list item
    fn toggle_status(&mut self) {
        if let Some(i) = self.notifications_list.state.selected() {
            self.notifications_list.items[i].status = match self.notifications_list.items[i].status
            {
                Status::Done => Status::Unread,
                Status::Read => Status::Unread,
                Status::Unread => Status::Done,
            }
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_layout = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ]);
        let [header_area, content_area, footer_area] = area.layout(&main_layout);

        let content_layout = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]);
        let [list_area, item_area] = content_area.layout(&content_layout);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);
        self.render_list(list_area, buf);
        self.render_selected_item(item_area, buf);
    }
}

/// Rendering logic for the app
impl App {
    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Ratatui Todo List Example")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, ← to unselect, → to change status, g/G to go top/bottom.")
            .centered()
            .render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw("TODO List").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .notifications_list
            .items
            .iter()
            .enumerate()
            .map(|(i, todo_item)| {
                let color = alternate_colors(i);
                ListItem::from(todo_item).bg(color)
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        StatefulWidget::render(list, area, buf, &mut self.notifications_list.state);
    }

    fn render_selected_item(&self, area: Rect, buf: &mut Buffer) {
        // We get the info depending on the item's state.
        let info = if let Some(i) = self.notifications_list.state.selected() {
            match self.notifications_list.items[i].status {
                Status::Done => format!("✓ DONE: {}", self.notifications_list.items[i].body),
                Status::Unread => format!("☐ UNREAD: {}", self.notifications_list.items[i].body),
                Status::Read => format!("☐ READ: {}", self.notifications_list.items[i].body),
            }
        } else {
            "Nothing selected...".to_string()
        };

        // We show the list item's info under the list in this paragraph
        let block = Block::new()
            .title(Line::raw("TODO Info").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG)
            .padding(Padding::horizontal(1));

        // We can now render the item info
        Paragraph::new(info)
            .block(block)
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

const fn alternate_colors(i: usize) -> Color {
    if i.is_multiple_of(2) {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}

impl From<&Notification> for ListItem<'_> {
    fn from(value: &Notification) -> Self {
        let line = match value.status {
            Status::Unread => Line::styled(format!(" ☐ {}", value.title), TEXT_FG_COLOR),
            Status::Read => Line::styled(format!(" ☐ {}", value.title), TEXT_FG_COLOR),
            Status::Done => Line::styled(format!(" ✓ {}", value.title), COMPLETED_TEXT_FG_COLOR),
        };
        ListItem::new(line)
    }
}
