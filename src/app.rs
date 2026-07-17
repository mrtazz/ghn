use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, Padding, Paragraph, Row, StatefulWidget, Table, TableState,
    Widget, Wrap,
};
use ratatui::{symbols, DefaultTerminal};

use crate::cache;
use crate::config;
use crate::github;
use crate::notifications::{Notification, Status};

struct Theme {
    pub accent: Color,
    pub secondary: Color,
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub selection: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
}

const SOLARIZED_LIGHT: Theme = Theme {
    accent: Color::Rgb(38, 139, 210),     // Blue
    secondary: Color::Rgb(108, 113, 196), // Violet
    bg: Color::Rgb(253, 246, 227),        // base3
    fg: Color::Rgb(101, 123, 131),        // base00
    muted: Color::Rgb(147, 161, 161),     // base1
    selection: Color::Rgb(238, 232, 213), // base2
    error: Color::Rgb(220, 50, 47),       // red
    warning: Color::Rgb(181, 137, 0),     // yellow
    success: Color::Rgb(133, 153, 0),     // green
    info: Color::Rgb(42, 161, 152),       // cyan
};

enum InputMode {
    Normal,
    Insert,
}

pub struct App {
    should_exit: bool,
    should_show_info: bool,
    should_show_message: bool,
    message: String,
    notifications_list: NotificationList,
    theme: Theme,
    input_mode: InputMode,
    input: String,
    character_index: usize,
}

struct NotificationList {
    items: Vec<Notification>,
    state: TableState,
}

struct IndexWidths {
    pub status: Constraint,
    pub datetime: Constraint,
    pub author: Constraint,
    pub repo: Constraint,
    pub title: Constraint,
    pub github_type: Constraint,
    pub state: Constraint,
    pub reason: Constraint,
}
impl Default for IndexWidths {
    fn default() -> Self {
        Self {
            status: Constraint::Length(2),
            datetime: Constraint::Length(18),
            author: Constraint::Length(20),
            repo: Constraint::Length(20),
            title: Constraint::Length(100),
            github_type: Constraint::Length(15),
            state: Constraint::Length(8),
            reason: Constraint::Length(17),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        let mut items: Vec<Notification> = vec![];
        let mut should_show_error = false;
        let mut error_message = String::from("");

        // try and read from cache
        let cfg = config::Config::default();
        let cache_data = cache::read(&cfg.cache_file.unwrap()).ok();

        match github::get_notifications(cache_data.as_ref()) {
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
            theme: SOLARIZED_LIGHT,
            input_mode: InputMode::Normal,
            input: String::new(),
            character_index: 0,
        }
    }
}

impl App {
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                match self.input_mode {
                    InputMode::Normal => self.handle_normal_mode_key(key),
                    InputMode::Insert => self.handle_insert_mode_key(key),
                }
            }
        }
        Ok(())
    }

    fn show_message(&mut self, msg: String) {
        self.should_show_message = true;
        self.message = msg;
    }

    fn write_data_to_cache(&mut self) {
        let cfg = config::Config::default();
        match cache::write(&self.notifications_list.items, &cfg.cache_file.unwrap()) {
            Err(e) => {
                self.show_message(format!("Failed to sync notificatons state: {}", e));
            }
            Ok(_) => {}
        }
        self.should_show_message = false;
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
        match github::get_notifications(Some(&self.notifications_list.items)) {
            Err(e) => {
                self.show_message(format!("Failed to fetch updated notificatons: {}", e));
            }
            Ok(notifications) => {
                self.notifications_list.items = notifications;
            }
        }
        self.should_show_message = false;
    }

    fn handle_normal_mode_key(&mut self, key: KeyEvent) {
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
            KeyCode::Char('D') => self.input_mode = InputMode::Insert,
            KeyCode::Enter => self.show_info(),
            _ => {}
        }
    }
    fn handle_insert_mode_key(&mut self, key: KeyEvent) {
        match key.kind {
            KeyEventKind::Press => match key.code {
                KeyCode::Enter => self.submit_message(),
                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                KeyCode::Backspace => self.delete_char(),
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                _ => {}
            },
            _ => {}
        }
    }
    fn submit_message(&mut self) {
        if let Err(e) = self.mark_messages_done(self.input.clone()) {
            self.show_message(format!("Unable to mark as done: {}", e));
        }
        self.input.clear();
        self.reset_cursor();
        self.input_mode = InputMode::Normal;
    }

    fn mark_messages_done(&mut self, match_string: String) -> Result<(), String> {
        if match_string.is_empty() {
            return Ok(());
        }
        for n in self.notifications_list.items.iter_mut() {
            if n.title.contains(&match_string) {
                n.status = Status::Done;
            } else if n.repo.nwo.contains(&match_string) {
                n.status = Status::Done;
            }
        }
        Ok(())
    }

    // functions for handling the input box, most of this is taken from
    // https://ratatui.rs/examples/apps/user_input/
    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }
    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }
    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }
    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
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
            self.sync_state_to_github();
            self.write_data_to_cache();
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

        match self.input_mode {
            InputMode::Insert => self.render_input(footer_area, buf),
            _ => self.render_footer(footer_area, buf),
        }

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
        let text = "Use j/k to move, g/G to go top/bottom, d to mark done, D to mark done by substring match, N to mark unread, $ to sync state";
        if self.should_show_message {
            Paragraph::new(self.message.to_string())
                .centered()
                .fg(self.theme.error)
                .render(area, buf);
        } else {
            Paragraph::new(text)
                .centered()
                .fg(self.theme.accent)
                .render(area, buf);
        }
    }

    fn render_input(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(format!("Match to mark as done: {}", self.input.as_str()))
            .style(Style::default().fg(Color::Yellow))
            .render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let count = self.notifications_list.items.len();
        let block = Block::new()
            .title(Line::raw(format!("Notifications ({})", count)).centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(Style::new().fg(self.theme.accent));

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<Row> = self
            .notifications_list
            .items
            .iter()
            .enumerate()
            .map(|(_, notification)| match notification.status {
                Status::Done => Row::from(notification).style(self.theme.error),
                _ => Row::from(notification).style(self.theme.info),
            })
            .collect();

        let default_widths = IndexWidths::default();

        let widths = [
            default_widths.status,
            default_widths.datetime,
            default_widths.repo,
            default_widths.github_type,
            default_widths.author,
            default_widths.title,
            default_widths.reason,
        ];

        let table = Table::new(items, widths)
            .block(block)
            .row_highlight_style(
                Style::new()
                    .bg(self.theme.warning)
                    .fg(self.theme.selection)
                    .add_modifier(Modifier::BOLD),
            )
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
            match &self.notifications_list.items[i].details {
                Err(e) => format!(
                    "URL: {}\nUnable to get notification details: {}",
                    &self.notifications_list.items[i].url, e
                ),
                Ok(v) => {
                    let comment = v.latest_comment.clone().unwrap_or_default();
                    format!(
                        "URL: {}\nauthor: {}\n\n{} ({}):\n\n{}",
                        v.url, v.author, comment.author, comment.url, comment.body,
                    )
                }
            }
        } else {
            "Nothing selected...".to_string()
        };

        // We show the list item's info under the list in this paragraph
        let block = Block::new()
            .title(Line::raw("Notification Info").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(Style::new().fg(self.theme.fg))
            .padding(Padding::horizontal(1));

        // We can now render the item info
        Paragraph::new(info)
            .block(block)
            .fg(self.theme.fg)
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
            format!("{}", value.updated_at.format("%Y-%m-%d %H:%M")),
            format!("{}", value.repo.nwo),
            format!(
                "{} ({})",
                value.github_type,
                match value.details.clone().unwrap_or_default().state.as_str() {
                    "closed" => format!("C"),
                    "open" => format!("O"),
                    _ => format!(""),
                }
            ),
            match &value.details {
                Err(_) => format!("n/a"),
                Ok(v) => match v.latest_comment.clone() {
                    Some(comment) => format!("{}", comment.author),
                    None => format!("{}", v.author),
                },
            },
            format!("{}", value.title),
            format!("{}", value.reason),
        ])
    }
}
