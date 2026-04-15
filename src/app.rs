use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::palette::tailwind::{BLUE, GREEN, RED, SLATE};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, Padding, Paragraph, Row, StatefulWidget, Table, TableState,
    Widget, Wrap,
};
use ratatui::{symbols, DefaultTerminal};

use crate::github;
use crate::notifications::{Notification, Status};

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;
const DONE_TEXT_BG_COLOR: Color = RED.c800;

pub struct App {
    should_exit: bool,
    should_show_info: bool,
    should_show_message: bool,
    message: String,
    notifications_list: NotificationList,
}

struct NotificationList {
    items: Vec<Notification>,
    state: TableState,
}

impl Default for App {
    fn default() -> Self {
        let mut items: Vec<Notification> = vec![];
        let mut should_show_error = false;
        let mut error_message = String::from("");

        match github::get_notifications() {
            Err(e) => {
                should_show_error = true;
                error_message = format!("Failed to get initial notifications: {}", e);
            }
            Ok(notifications) => {
                items = notifications;
            }
        }
        Self {
            should_exit: false,
            should_show_info: false,
            should_show_message: should_show_error,
            message: error_message,
            notifications_list: NotificationList {
                items: items,
                state: TableState::default(),
            },
        }
    }
}

impl App {
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    fn show_message(&mut self, msg: String) {
        self.should_show_message = true;
        self.message = msg;
    }

    fn sync_state_to_github(&mut self) {
        match github::update_state(&self.notifications_list.items) {
            Err(e) => {
                self.show_message(format!("Failed to sync notificatons state: {}", e));
            }
            Ok(_) => {}
        }
        self.show_message(String::from(
            "Done updating notificatons state, re-fetching...",
        ));
        match github::get_notifications() {
            Err(e) => {
                self.show_message(format!("Failed to fetch updated notificatons: {}", e));
            }
            Ok(notifications) => {
                self.notifications_list.items = notifications;
            }
        }
        self.should_show_message = false;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('$') => self.sync_state_to_github(),
            KeyCode::Char('q') => self.close_content_or_app(),
            KeyCode::Char('h') | KeyCode::Left => self.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.select_last(),
            KeyCode::Char('N') => self.change_status(Status::Unread),
            KeyCode::Char('d') => {
                self.change_status(Status::Done);
                self.select_next();
            }
            KeyCode::Char('r') => self.change_status(Status::Read),
            KeyCode::Enter => self.show_info(),
            _ => {}
        }
    }

    fn close_content_or_app(&mut self) {
        if self.should_show_message {
            self.should_show_message = false;
            return;
        }
        if self.should_show_info {
            self.should_show_info = false
        } else {
            self.should_exit = true
        }
    }

    fn show_info(&mut self) {
        self.should_show_info = true
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

    fn change_status(&mut self, status: Status) {
        if let Some(i) = self.notifications_list.state.selected() {
            self.notifications_list.items[i].status = status;
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
        App::render_header(header_area, buf);
        self.render_footer(footer_area, buf);

        if !self.should_show_info {
            let content_layout = Layout::vertical([Constraint::Fill(1)]);
            let [list_area] = content_area.layout(&content_layout);
            self.render_list(list_area, buf);
            return;
        }

        let content_layout = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]);
        let [list_area, item_area] = content_area.layout(&content_layout);
        self.render_list(list_area, buf);
        self.render_selected_item(item_area, buf);
    }
}

/// Rendering logic for the app
impl App {
    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("ghn - GitHub notifications")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = "Use j/k to move, g/G to go top/bottom, d to mark done, N to mark unread, r to mark as read, $ to sync state";
        if self.should_show_message {
            Paragraph::new(self.message.to_string())
                .centered()
                .render(area, buf);
        } else {
            Paragraph::new(text).centered().render(area, buf);
        }
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let count = self.notifications_list.items.len();
        let block = Block::new()
            .title(Line::raw(format!("Notifications ({})", count)).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<Row> = self
            .notifications_list
            .items
            .iter()
            .enumerate()
            .map(|(i, notification)| {
                let color = alternate_colors(i, notification.status);
                Row::from(notification).bg(color)
            })
            .collect();

        let widths = [
            Constraint::Length(2),
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(100),
        ];

        let table = Table::new(items, widths)
            .block(block)
            .row_highlight_style(SELECTED_STYLE)
            .highlight_symbol(">>")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        StatefulWidget::render(table, area, buf, &mut self.notifications_list.state);
    }

    fn render_selected_item(&self, area: Rect, buf: &mut Buffer) {
        if !self.should_show_info {
            return;
        }
        // We get the info depending on the item's state.
        let info = if let Some(i) = self.notifications_list.state.selected() {
            format!("URL: {}", self.notifications_list.items[i].url)
        } else {
            "Nothing selected...".to_string()
        };

        // We show the list item's info under the list in this paragraph
        let block = Block::new()
            .title(Line::raw("Notification Info").centered())
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

impl<'a> From<&Notification> for Row<'a> {
    fn from(value: &Notification) -> Self {
        let status_marker = match value.status {
            Status::Unread => "N",
            Status::Read => "R",
            Status::Done => "D",
        };

        Row::new(vec![
            format!("{}", status_marker),
            format!("{}", value.updated_at.format("%Y-%m-%d %H:%M:%S")),
            format!("{}", value.github_type),
            format!("{}", value.reason),
            format!("{}", value.repo),
            format!("{}", value.title),
        ])
        .style(TEXT_FG_COLOR)
    }
}

const fn alternate_colors(i: usize, s: Status) -> Color {
    match s {
        Status::Done => DONE_TEXT_BG_COLOR,
        _ => {
            if i.is_multiple_of(2) {
                NORMAL_ROW_BG
            } else {
                ALT_ROW_BG_COLOR
            }
        }
    }
}
