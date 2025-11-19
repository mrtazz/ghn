use crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::palette::tailwind::{BLUE, GREEN, SLATE};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, Paragraph, StatefulWidget,
};
use ratatui::{text::Line, text::Text, Frame};
use Constraint::{Fill, Length, Min};

const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

fn main() {
    let mut terminal = ratatui::init();
    loop {
        terminal.draw(draw).expect("failed to draw frame");
        if matches!(event::read().expect("failed to read event"), Event::Key(_)) {
            break;
        }
    }
    ratatui::restore();
}

fn draw(frame: &mut Frame) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(frame.area());

    let index_content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_layout[1]);

    frame.render_widget(
        Paragraph::new("sidebar").block(Block::new().borders(Borders::ALL)),
        main_layout[0],
    );
    frame.render_widget(
        Paragraph::new("index").block(Block::new().borders(Borders::ALL)),
        index_content_layout[0],
    );
    frame.render_widget(
        Paragraph::new("content").block(Block::new().borders(Borders::ALL)),
        index_content_layout[1],
    );
    render_list(index_content_layout[1]);
}

fn render_list(area: Rect) {
    let block = Block::new()
        .title(Line::raw("Notifications").centered())
        .borders(Borders::TOP)
        .border_set(symbols::border::EMPTY)
        .border_style(TODO_HEADER_STYLE)
        .bg(NORMAL_ROW_BG);

    let raw_items = vec!["new stuff", "this needs your :eyes:", "stuff"];

    // Iterate through all elements in the `items` and stylize them.
    let items: Vec<ListItem> = raw_items
        .iter()
        .enumerate()
        .map(|(i, todo_item)| {
            let color = alternate_colors(i);
            ListItem::new(*todo_item).bg(color)
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
    StatefulWidget::render(list, area, buf, &mut self.todo_list.state);
}

const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}
