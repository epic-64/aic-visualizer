//! Rendering for every screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, LegendPosition, List, ListItem,
        Paragraph, Wrap,
    },
    Frame,
};

use crate::app::{App, Screen, FORM_FIELDS};

const ACCENT: Color = Color::Cyan;

pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::ModelSelect => model_select(f, app),
        Screen::CustomModel => custom_model(f, app),
        Screen::Start => start(f, app),
        Screen::Chat => chat(f, app),
        Screen::SaveName => {
            chat(f, app);
            save_name(f, app);
        }
    }
}

fn title_bar(f: &mut Frame, area: Rect, subtitle: &str) {
    let line = Line::from(vec![
        Span::styled(" Token Visualizer ", Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(subtitle, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn model_select(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    title_bar(f, chunks[0], "Choose a model");

    let items: Vec<ListItem> = app
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == app.selected;
            let marker = if selected { "▶ " } else { "  " };
            let name = Span::styled(
                format!("{marker}{:<16}", m.name),
                if selected {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            );
            let prices = Span::styled(
                format!(
                    "in ${:.2}  out ${:.2}  cached ${:.2}  / 1M",
                    m.input_per_m, m.output_per_m, m.cached_per_m
                ),
                Style::default().fg(Color::Gray),
            );
            ListItem::new(Line::from(vec![name, prices]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Models ")
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(list, chunks[1]);

    let help = Line::from(vec![
        key("↑/↓"), Span::raw(" move  "),
        key("Enter"), Span::raw(" select  "),
        key("c"), Span::raw(" custom model  "),
        key("q"), Span::raw(" quit"),
    ]);
    f.render_widget(Paragraph::new(help), chunks[2]);
}

fn start(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let subtitle = app
        .model()
        .map(|m| format!("Start a conversation with {}", m.name))
        .unwrap_or_else(|| "Start a conversation".into());
    title_bar(f, chunks[0], &subtitle);

    let items: Vec<ListItem> = app
        .start_items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let selected = i == app.start_selected;
            let marker = if selected { "▶ " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", it.label()),
                if selected {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Blank · examples · saved ")
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(list, chunks[1]);

    let help = Line::from(vec![
        key("↑/↓"), Span::raw(" move  "),
        key("Enter"), Span::raw(" start  "),
        key("Esc"), Span::raw(" back to models"),
    ]);
    f.render_widget(Paragraph::new(help), chunks[2]);
}

fn save_name(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 60, 3);
    f.render_widget(ratatui::widgets::Clear, area);
    let input = Paragraph::new(Line::from(vec![
        Span::raw(&app.save_name),
        Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Save as (Enter to confirm, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Green)),
    );
    f.render_widget(input, area);
}

fn custom_model(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    title_bar(f, chunks[0], "Create a custom model");

    let rows: Vec<Line> = FORM_FIELDS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let active = i == app.form_field;
            let cursor = if active { "_" } else { "" };
            Line::from(vec![
                Span::styled(
                    format!("{:>12}: ", label),
                    if active {
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
                Span::raw(format!("{}{}", app.form[i], cursor)),
            ])
        })
        .collect();

    let body = Paragraph::new(rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" New model (prices per 1M tokens, USD) ")
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(body, centered(chunks[1], 60, 9));

    let help = Line::from(vec![
        key("Tab"), Span::raw(" next field  "),
        key("Enter"), Span::raw(" save  "),
        key("Esc"), Span::raw(" cancel"),
    ]);
    f.render_widget(Paragraph::new(help), chunks[2]);

    if !app.status.is_empty() {
        let s = Paragraph::new(Span::styled(
            app.status.clone(),
            Style::default().fg(Color::Red),
        ))
        .alignment(Alignment::Center);
        let area = Rect { y: chunks[1].y + chunks[1].height.saturating_sub(2), height: 1, ..chunks[1] };
        f.render_widget(s, area);
    }
}

fn chat(f: &mut Frame, app: &App) {
    // Split the screen: chat on the left (capped), graphs fill
    // whatever width is left.
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Max(100), Constraint::Min(0)])
        .split(f.area());
    cost_graph(f, app, columns[1]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(3), // summary
            Constraint::Length(3), // context gauge
            Constraint::Min(0),    // history
            Constraint::Length(3), // input
            Constraint::Length(1), // help
        ])
        .split(columns[0]);

    let model = app.model();
    let subtitle = model.map(|m| m.name.as_str()).unwrap_or("");
    title_bar(f, chunks[0], subtitle);

    // Summary line: totals.
    let (tc, ti, to) = app.total_tokens();
    let summary = Line::from(vec![
        Span::styled("Total: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("${:.4}", app.total_cost()), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
        Span::styled(format!("cached {tc}  input {ti}  output {to}"), Style::default().fg(Color::Gray)),
        Span::raw("   "),
        Span::styled(format!("next turn cache: {} tok", app.carried_cached), Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(
        Paragraph::new(summary).block(Block::default().borders(Borders::ALL).title(" Conversation ")),
        chunks[1],
    );

    // Context-window usage gauge.
    let used = app.context_used();
    let max = app.context_max().max(1);
    let ratio = (used as f64 / max as f64).min(1.0);
    let pct = used as f64 / max as f64 * 100.0;
    let gauge_color = if pct >= 90.0 {
        Color::Red
    } else if pct >= 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Context window "))
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(ratio)
        .label(format!("{} / {} tokens  ({:.1}%)", fmt_int(used), fmt_int(max), pct));
    f.render_widget(gauge, chunks[2]);

    // History — most recent turns, newest at the bottom.
    let mut lines: Vec<Line> = Vec::new();
    for (i, t) in app.turns.iter().enumerate() {
        lines.push(Line::from(vec![
            Span::styled(format!("#{} ", i + 1), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw(t.raw.clone()),
        ]));
        // Per-component cost (dimmed, in parens) using the active model's rates.
        let (cached_cost, input_cost, output_cost) = match model {
            Some(m) => (
                t.cached as f64 / 1e6 * m.cached_per_m,
                t.input as f64 / 1e6 * m.input_per_m,
                t.output as f64 / 1e6 * m.output_per_m,
            ),
            None => (0.0, 0.0, 0.0),
        };
        let dim = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);
        lines.push(Line::from(vec![
            Span::styled("    cached ", Style::default().fg(Color::DarkGray)),
            Span::raw(t.cached.to_string()),
            Span::styled(format!(" (${cached_cost:.4})"), dim),
            Span::styled("  input ", Style::default().fg(Color::DarkGray)),
            Span::raw(t.input.to_string()),
            Span::styled(format!(" (${input_cost:.4})"), dim),
            Span::styled("  output ", Style::default().fg(Color::DarkGray)),
            Span::raw(t.output.to_string()),
            Span::styled(format!(" (${output_cost:.4})"), dim),
            Span::styled("   → ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("${:.4}", t.cost), Style::default().fg(Color::Green)),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No turns yet. Try: 300 tokens prompt, 12000 tokens tool inputs, 4000 tokens response",
            Style::default().fg(Color::DarkGray),
        )));
    }
    // Keep the latest content visible by scrolling to the bottom.
    let inner_height = chunks[3].height.saturating_sub(2) as usize;
    let scroll = lines.len().saturating_sub(inner_height) as u16;
    let history = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(Block::default().borders(Borders::ALL).title(" Turns "));
    f.render_widget(history, chunks[3]);

    // Input box.
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(ACCENT)),
        Span::raw(&app.input),
        Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Describe this turn ")
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(input, chunks[4]);

    let help = Line::from(vec![
        key("Enter"), Span::raw(" send  "),
        key("↑/↓"), Span::raw(" recall  "),
        key("Ctrl+S"), Span::raw(" save  "),
        key("Esc"), Span::raw(" models  "),
        key("Ctrl+C"), Span::raw(" quit  "),
        Span::styled(format!("  {}", app.status), Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(help), chunks[5]);
}

/// Right-hand panel: two stacked charts — total (cumulative) cost on top,
/// per-turn cost on the bottom. They get separate y-axes because the scales
/// diverge sharply (the total climbs far above any single turn).
fn cost_graph(f: &mut Frame, app: &App, area: Rect) {
    if app.turns.is_empty() {
        let placeholder = Paragraph::new(Span::styled(
            "No turns yet — the graphs fill in as you send turns.",
            Style::default().fg(Color::DarkGray),
        ))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cost ")
                .border_style(Style::default().fg(ACCENT)),
        );
        f.render_widget(placeholder, area);
        return;
    }

    // Build the series, indexed by turn number on the x-axis. For the
    // per-turn chart we also track how much of each bar was cache cost.
    let model = app.model();
    let mut per_turn: Vec<(f64, f64)> = Vec::new();
    let mut per_turn_cache: Vec<(f64, f64)> = Vec::new();
    let mut cumulative: Vec<(f64, f64)> = Vec::new();
    let mut running = 0.0;
    for (i, t) in app.turns.iter().enumerate() {
        let x = (i + 1) as f64;
        let cache_cost = model
            .map(|m| t.cached as f64 / 1e6 * m.cached_per_m)
            .unwrap_or(0.0);
        per_turn.push((x, t.cost));
        per_turn_cache.push((x, cache_cost));
        running += t.cost;
        cumulative.push((x, running));
    }
    let n = app.turns.len() as f64;
    let per_turn_max = per_turn.iter().map(|p| p.1).fold(0.0_f64, f64::max);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Top: cumulative total cost.
    cost_chart(
        f,
        rows[0],
        " Total cost ",
        &[("total", Color::Green, GraphType::Line, &cumulative)],
        n,
        running,
    );

    // Bottom: per-turn cost. The full bar is the turn's total; the magenta
    // overlay (drawn on top, same baseline) shows the cache-cost portion.
    cost_chart(
        f,
        rows[1],
        " Cost per turn (magenta = cache) ",
        &[
            ("turn", Color::Cyan, GraphType::Bar, &per_turn),
            ("cache", Color::Magenta, GraphType::Bar, &per_turn_cache),
        ],
        n,
        per_turn_max,
    );
}

/// Render a cost chart filling `area` with one or more overlaid series,
/// sharing a single y-scale derived from `data_max`.
fn cost_chart(
    f: &mut Frame,
    area: Rect,
    title: &str,
    series: &[(&str, Color, GraphType, &[(f64, f64)])],
    n: f64,
    data_max: f64,
) {
    // Headroom so the top isn't flush against the border.
    let y_max = (data_max * 1.1).max(0.0001);

    let datasets: Vec<Dataset> = series
        .iter()
        .map(|(name, color, graph_type, data)| {
            Dataset::default()
                .name(*name)
                .marker(Marker::Braille)
                .graph_type(*graph_type)
                .style(Style::default().fg(*color))
                .data(data)
        })
        .collect();

    let x_axis = Axis::default()
        .style(Style::default().fg(Color::DarkGray))
        .bounds([0.5, n + 0.5])
        .labels(vec![Span::raw("1"), Span::raw(format!("{}", n as usize))]);

    let y_axis = Axis::default()
        .style(Style::default().fg(Color::DarkGray))
        .bounds([0.0, y_max])
        .labels(vec![
            Span::raw("$0"),
            Span::styled(format!("${:.3}", y_max / 2.0), Style::default().fg(Color::Gray)),
            Span::styled(format!("${:.3}", y_max), Style::default().fg(Color::Gray)),
        ]);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(ACCENT)),
        )
        .x_axis(x_axis)
        .y_axis(y_axis)
        .legend_position(Some(LegendPosition::TopLeft));
    f.render_widget(chart, area);
}

/// Format an integer with thousands separators, e.g. 1234567 -> "1,234,567".
fn fmt_int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

fn key(k: &str) -> Span<'static> {
    Span::styled(
        k.to_string(),
        Style::default().fg(Color::Black).bg(Color::Gray).add_modifier(Modifier::BOLD),
    )
}

/// A centered sub-rectangle of `area` with the given width/height (clamped).
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width - w) / 2;
    let y = area.y + (area.height - h) / 2;
    Rect { x, y, width: w, height: h }
}
