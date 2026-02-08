use crate::app::{App, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Padding},
    Frame,
};
use unicode_width::UnicodeWidthStr;

const BG: Color = Color::Rgb(20, 20, 24);
const FG: Color = Color::Rgb(220, 220, 220);
const DIM: Color = Color::Rgb(90, 90, 100);
const BORDER: Color = Color::Rgb(50, 50, 60);
const CYAN: Color = Color::Rgb(80, 200, 220);
const GREEN: Color = Color::Rgb(100, 200, 100);
const RED: Color = Color::Rgb(200, 80, 80);
const YELLOW: Color = Color::Rgb(200, 180, 80);
const SELECT_BG: Color = Color::Rgb(40, 45, 55);

pub fn render(frame: &mut Frame, app: &App) {
    frame.render_widget(Block::default().style(Style::default().bg(BG)), frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_input_box(frame, chunks[0], app);
    render_search_box(frame, chunks[1], app);
    render_results(frame, chunks[2], app);
    render_status(frame, chunks[3], app);

    if app.show_help {
        render_help(frame);
    }
}

fn render_input_box(frame: &mut Frame, area: Rect, app: &App) {
    let active = matches!(app.mode, Mode::Input);
    let border = if active { CYAN } else { BORDER };

    let text = if app.input.is_empty() && !active {
        Span::styled("Write a thought and press Enter to save...", Style::default().fg(DIM))
    } else {
        Span::styled(&app.input, Style::default().fg(FG))
    };

    let block = Block::default()
        .title(" + New ")
        .title_style(Style::default().fg(if active { CYAN } else { DIM }))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(text), inner);

    if active {
        let x = inner.x + app.input.width() as u16;
        frame.set_cursor_position((x.min(inner.right() - 1), inner.y));
    }
}

fn render_search_box(frame: &mut Frame, area: Rect, app: &App) {
    let active = matches!(app.mode, Mode::Search);
    let border = if active { CYAN } else { BORDER };

    let text = if app.search.is_empty() && !active {
        Span::styled("Search thoughts...", Style::default().fg(DIM))
    } else {
        Span::styled(&app.search, Style::default().fg(FG))
    };

    let block = Block::default()
        .title(" / Search ")
        .title_style(Style::default().fg(if active { CYAN } else { DIM }))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(text), inner);

    if active {
        let x = inner.x + app.search.width() as u16;
        frame.set_cursor_position((x.min(inner.right() - 1), inner.y));
    }
}

fn render_results(frame: &mut Frame, area: Rect, app: &App) {
    let results = &app.results;

    if results.is_empty() {
        let msg = if app.search.is_empty() {
            "Your thoughts will appear here"
        } else {
            "No matches"
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(DIM))
            .block(Block::default().padding(Padding::new(2, 0, 1, 0)));
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = results
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let selected = matches!(app.mode, Mode::Browse) && i == app.selected;
            let style = if selected {
                Style::default().fg(CYAN).bg(SELECT_BG).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            let prefix = if selected { " ▸ " } else { "   " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(CYAN)),
                Span::styled(&node.content, style),
            ]))
        })
        .collect();

    let title = format!(" {} thoughts ", results.len());
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .title_style(Style::default().fg(DIM))
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDER))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(list, area);
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(ref msg) = app.message {
        let color = if msg.starts_with("Error") || msg.starts_with("Failed") { RED } else { GREEN };
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {}", msg), Style::default().fg(color))),
            area,
        );
        return;
    }

    let hints: Vec<(&str, &str)> = match app.mode {
        Mode::Input => vec![("Enter", "save"), ("Tab", "search"), ("Esc", "browse")],
        Mode::Search => vec![("Enter", "browse"), ("Tab", "input"), ("Esc", "clear")],
        Mode::Browse => vec![("↑↓", "select"), ("y", "copy"), ("d", "delete"), ("Tab", "input"), ("?", "help"), ("q", "quit")],
    };

    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(k, v)| vec![
            Span::styled(format!(" {} ", k), Style::default().fg(YELLOW)),
            Span::styled(*v, Style::default().fg(DIM)),
        ])
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_help(frame: &mut Frame) {
    let area = centered(35, 50, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(" Tab      ", Style::default().fg(YELLOW)), Span::styled("Switch boxes", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" Enter    ", Style::default().fg(YELLOW)), Span::styled("Save / Confirm", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" ↑ ↓      ", Style::default().fg(YELLOW)), Span::styled("Navigate results", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" y        ", Style::default().fg(YELLOW)), Span::styled("Copy selected", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" d        ", Style::default().fg(YELLOW)), Span::styled("Delete selected", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" Esc      ", Style::default().fg(YELLOW)), Span::styled("Back / Clear", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" ?        ", Style::default().fg(YELLOW)), Span::styled("Toggle help", Style::default().fg(FG))]),
        Line::from(vec![Span::styled(" q        ", Style::default().fg(YELLOW)), Span::styled("Quit", Style::default().fg(FG))]),
        Line::from(""),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .title(" Help ")
            .title_style(Style::default().fg(CYAN))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CYAN))
            .style(Style::default().bg(BG))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(p, area);
}

fn centered(w: u16, h: u16, r: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - h) / 2),
            Constraint::Percentage(h),
            Constraint::Percentage((100 - h) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - w) / 2),
            Constraint::Percentage(w),
            Constraint::Percentage((100 - w) / 2),
        ])
        .split(v[1])[1]
}
