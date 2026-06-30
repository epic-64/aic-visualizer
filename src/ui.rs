//! Rendering for every screen.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
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
        Screen::ConfirmDelete => {
            start(f, app);
            confirm_delete(f, app);
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
                    "out ${:>6.2}   in ${:>6.2}   cached ${:>6.2}   / 1M",
                    m.output_per_m, m.input_per_m, m.cached_per_m
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
    // Center the table: content row is 66 cols wide (2 marker + 16 name + 48
    // prices) plus the box border, and one row per model plus the border.
    let area = centered(chunks[1], 70, app.models.len() as u16 + 2);
    f.render_widget(list, area);

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

    // Show feedback (e.g. a deletion result) when present, otherwise the keys.
    let footer = if app.status.is_empty() {
        Line::from(vec![
            key("↑/↓"), Span::raw(" move  "),
            key("Enter"), Span::raw(" start  "),
            key("d"), Span::raw(" delete saved  "),
            key("Esc"), Span::raw(" back to models"),
        ])
    } else {
        Line::from(Span::styled(app.status.clone(), Style::default().fg(ACCENT)))
    };
    f.render_widget(Paragraph::new(footer), chunks[2]);
}

fn confirm_delete(f: &mut Frame, app: &App) {
    let name = app.selected_saved_name().unwrap_or("");
    let area = centered(f.area(), 60, 5);
    f.render_widget(ratatui::widgets::Clear, area);
    let body = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Delete saved conversation "),
            Span::styled(format!("'{name}'"), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(Span::styled("This removes the file from disk.", Style::default().fg(Color::DarkGray))),
        Line::from(vec![
            key("y"), Span::raw(" delete   "),
            key("n"), Span::raw(" cancel"),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm delete ")
            .border_style(Style::default().fg(Color::Red)),
    );
    f.render_widget(body, area);
}

fn save_name(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 60, 4);
    f.render_widget(ratatui::widgets::Clear, area);
    let input = Paragraph::new(vec![
        Line::from(vec![
            Span::raw(&app.save_name),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        Line::from(Span::styled(
            "An existing name overrides that saved conversation.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
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
            Constraint::Length(1), // tab bar
            Constraint::Length(1), // title
            Constraint::Length(3), // summary
            Constraint::Length(3), // context gauge
            Constraint::Min(0),    // history
            Constraint::Length(3), // input
            Constraint::Length(1), // help
        ])
        .split(columns[0]);

    tab_bar(f, app, chunks[0]);

    let model = app.model();
    let subtitle = model.map(|m| m.name.as_str()).unwrap_or("");
    title_bar(f, chunks[1], subtitle);

    // Summary line: totals.
    let (tc, ti, to, tt) = app.total_tokens();
    let summary = Line::from(vec![
        Span::styled("Total: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("${:.4}", app.total_cost()), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
        Span::styled(format!("cached {}  input {}  output {}  thinking {}", fmt_tok(tc), fmt_tok(ti), fmt_tok(to), fmt_tok(tt)), Style::default().fg(Color::Gray)),
        Span::raw("   "),
        Span::styled(format!("next turn cache: {} tok", fmt_tok(app.carried_cached)), Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(
        Paragraph::new(summary).block(Block::default().borders(Borders::ALL).title(" Conversation ")),
        chunks[2],
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
    f.render_widget(gauge, chunks[3]);

    // History: most recent turns, newest at the bottom. Markers are woven
    // in at their recorded positions as un-numbered separator lines.
    let mut lines: Vec<Line> = Vec::new();
    let push_markers = |lines: &mut Vec<Line>, pos: usize| {
        for m in &app.markers {
            if m.after == pos {
                let text = if m.label.is_empty() {
                    "──────── marker ────────".to_string()
                } else {
                    format!("──────── marker · {} ────────", m.label)
                };
                lines.push(Line::from(Span::styled(
                    text,
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                )));
            }
        }
    };
    for (i, t) in app.turns.iter().enumerate() {
        push_markers(&mut lines, i);
        lines.push(Line::from(vec![
            Span::styled(format!("#{} ", i + 1), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw(t.raw.clone()),
            Span::raw("  "),
            Span::styled(format!("${:.4}", t.cost), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        // Per-component cost (dimmed, in parens) using the active model's rates.
        // Thinking tokens bill at the output rate.
        let (cached_cost, input_cost, output_cost, thinking_cost) = match model {
            Some(m) => (
                t.cached as f64 / 1e6 * m.cached_per_m,
                t.input as f64 / 1e6 * m.input_per_m,
                t.output as f64 / 1e6 * m.output_per_m,
                t.thinking as f64 / 1e6 * m.output_per_m,
            ),
            None => (0.0, 0.0, 0.0, 0.0),
        };
        let dim = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);
        let mut detail = vec![
            Span::styled("    cached ", Style::default().fg(Color::DarkGray)),
            Span::raw(fmt_tok(t.cached)),
            Span::styled(format!(" (${cached_cost:.4})"), dim),
            Span::styled("  input ", Style::default().fg(Color::DarkGray)),
            Span::raw(fmt_tok(t.input)),
            Span::styled(format!(" (${input_cost:.4})"), dim),
            Span::styled("  output ", Style::default().fg(Color::DarkGray)),
            Span::raw(fmt_tok(t.output)),
            Span::styled(format!(" (${output_cost:.4})"), dim),
        ];
        // Only show the thinking component when there is any.
        if t.thinking > 0 {
            detail.push(Span::styled("  thinking ", Style::default().fg(Color::DarkGray)));
            detail.push(Span::raw(fmt_tok(t.thinking)));
            detail.push(Span::styled(format!(" (${thinking_cost:.4})"), dim));
        }
        lines.push(Line::from(detail));
    }
    // Markers placed after the most recent turn.
    push_markers(&mut lines, app.turns.len());
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No turns yet. Try: 300 tokens prompt, 12000 tokens tool inputs, 4000 tokens response",
            Style::default().fg(Color::DarkGray),
        )));
    }
    // Default to keeping the latest content visible (scrolled to the bottom);
    // the user's mouse-wheel offset pulls the view back up toward older turns.
    let inner_height = chunks[4].height.saturating_sub(2) as usize;
    let bottom = lines.len().saturating_sub(inner_height) as u16;
    // Record the real scrollable distance so the scroll handlers can clamp
    // against it (markers and wrapping make turns*2 the wrong cap).
    app.history_max_scroll.set(bottom);
    let scroll = bottom.saturating_sub(app.scroll_up.min(bottom));
    let scrolled = scroll < bottom;
    let title = if scrolled { " Turns (scrolled; wheel down for newer) " } else { " Turns " };
    let history = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(history, chunks[4]);

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
    f.render_widget(input, chunks[5]);

    let help = Line::from(vec![
        key("Enter"), Span::raw(" send  "),
        key("↑/↓"), Span::raw(" recall  "),
        key("Tab"), Span::raw(" switch tab  "),
        key("Ctrl+T"), Span::raw(" new  "),
        key("Ctrl+W"), Span::raw(" close  "),
        key("Ctrl+S"), Span::raw(" save  "),
        key("marker [text]"), Span::raw(" mark  "),
        key("Esc"), Span::raw(" models  "),
        Span::styled(format!("  {}", app.status), Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(help), chunks[6]);
}

/// The tab strip across the top of the chat view: one chip per open
/// conversation, the active one highlighted.
fn tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = vec![Span::styled(
        " Tabs: ",
        Style::default().fg(Color::DarkGray),
    )];
    for t in app.tab_summaries() {
        let style = if t.active {
            Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(format!(" {} ", t.label), style));
        spans.push(Span::raw(" "));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Right-hand panel: two stacked charts. Total (cumulative) cost on top,
/// per-turn cost on the bottom. They get separate y-axes because the scales
/// diverge sharply (the total climbs far above any single turn).
fn cost_graph(f: &mut Frame, app: &App, area: Rect) {
    if app.turns.is_empty() {
        let placeholder = Paragraph::new(Span::styled(
            "No turns yet. The graphs fill in as you send turns.",
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

    // Build a series set per open tab, indexed by turn number on the x-axis.
    // Every tab is plotted at once so conversations can be compared directly;
    // the active tab is drawn bright and on top, the rest dimmed underneath.
    // For the per-turn chart we also track each bar's cache-cost portion.
    let mut charts: Vec<TabChart> = Vec::with_capacity(app.tabs.len());
    for i in 0..app.tabs.len() {
        let model = app.tab_model(i);
        let mut per_turn: Vec<(f64, f64)> = Vec::new();
        let mut per_turn_cache: Vec<(f64, f64)> = Vec::new();
        let mut cumulative: Vec<(f64, f64)> = Vec::new();
        let mut running = 0.0;
        for (j, t) in app.tab_turns(i).iter().enumerate() {
            let x = (j + 1) as f64;
            let cache_cost = model
                .map(|m| t.cached as f64 / 1e6 * m.cached_per_m)
                .unwrap_or(0.0);
            per_turn.push((x, t.cost));
            per_turn_cache.push((x, cache_cost));
            running += t.cost;
            cumulative.push((x, running));
        }
        charts.push(TabChart {
            active: i == app.active_tab,
            per_turn,
            per_turn_cache,
            cumulative,
        });
    }

    // X-axis spans the longest conversation; y-axes use the cross-tab maxima
    // so every overlaid series shares a comparable scale.
    let n = charts.iter().map(|c| c.cumulative.len()).max().unwrap_or(0) as f64;
    let total_max = app.max_total_cost_across_tabs();
    let per_turn_y_max = app.max_turn_cost_across_tabs();

    // Style for the dimmed underlay series (the non-active tabs).
    let dim = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);

    // Total chart overlays every tab (dim-first so the active one draws on
    // top). The per-turn chart shows only the active tab; overlaying every
    // tab's bars there made it too noisy to read.
    let mut total_series: Vec<Series> = Vec::new();
    let mut turn_series: Vec<Series> = Vec::new();
    for c in charts.iter().filter(|c| !c.active) {
        total_series.push(("", dim, GraphType::Line, &c.cumulative));
    }
    for c in charts.iter().filter(|c| c.active) {
        total_series.push((
            "total",
            Style::default().fg(Color::Green),
            GraphType::Line,
            &c.cumulative,
        ));
        turn_series.push((
            "turn",
            Style::default().fg(Color::Cyan),
            GraphType::Bar,
            &c.per_turn,
        ));
        turn_series.push((
            "cache",
            Style::default().fg(Color::Magenta),
            GraphType::Bar,
            &c.per_turn_cache,
        ));
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // The active conversation's markers, placed between turns: a marker set
    // after turn k sits at x = k + 0.5.
    let marker_xs: Vec<f64> = app.markers.iter().map(|m| m.after as f64 + 0.5).collect();

    // Markers split the active conversation into segments; label each segment
    // (start→marker, marker→marker, marker→end) with its summed cost, centred
    // over the segment. Skipped when there are no markers (one whole segment).
    let mut seg_labels: Vec<(f64, String)> = Vec::new();
    if !app.markers.is_empty() {
        let turn_n = app.turns.len();
        let mut bounds: Vec<usize> = vec![0, turn_n];
        bounds.extend(app.markers.iter().map(|m| m.after.min(turn_n)));
        bounds.sort_unstable();
        bounds.dedup();
        for w in bounds.windows(2) {
            let (p, q) = (w[0], w[1]);
            let cost: f64 = app.turns[p..q].iter().map(|t| t.cost).sum();
            // Boundary p sits at data-x p + 0.5, so the segment's midpoint is
            // ((p + 0.5) + (q + 0.5)) / 2.
            let mid = (p as f64 + q as f64 + 1.0) / 2.0;
            seg_labels.push((mid, format!("${cost:.4}")));
        }
    }

    // Top: cumulative total cost. Braille gives a smooth line. The per-segment
    // cost labels live here; the legend is dropped so they own the top row.
    cost_chart(f, rows[0], " Total cost ", Marker::Braille, &total_series, n, total_max, &marker_xs, false, &seg_labels);

    // Bottom: per-turn cost. The full bar is the turn's total; the magenta
    // overlay (drawn on top, same baseline) shows the cache-cost portion.
    // HalfBlock keeps the two bars cell-aligned so they meet without a gap.
    cost_chart(
        f,
        rows[1],
        " Cost per turn (magenta = cache) ",
        Marker::HalfBlock,
        &turn_series,
        n,
        per_turn_y_max,
        &marker_xs,
        true,
        &[],
    );
}

/// A single plotted series: legend name, style, shape, and its data points.
type Series<'a> = (&'a str, Style, GraphType, &'a [(f64, f64)]);

/// One tab's computed cost series, ready to plot. `active` marks the tab the
/// user is currently viewing (drawn bright; the others are dimmed underlays).
struct TabChart {
    active: bool,
    per_turn: Vec<(f64, f64)>,
    per_turn_cache: Vec<(f64, f64)>,
    cumulative: Vec<(f64, f64)>,
}

/// Render a cost chart filling `area` with one or more overlaid series,
/// sharing a single y-scale derived from `data_max`.
fn cost_chart(
    f: &mut Frame,
    area: Rect,
    title: &str,
    marker: Marker,
    series: &[Series],
    n: f64,
    data_max: f64,
    marker_xs: &[f64],
    show_legend: bool,
    segment_labels: &[(f64, String)],
) {
    // Headroom so the top isn't flush against the border.
    let y_max = (data_max * 1.1).max(0.0001);

    // Each marker is a dim vertical line spanning the full height. Built up
    // front so the points outlive the datasets that borrow them.
    let marker_lines: Vec<[(f64, f64); 2]> =
        marker_xs.iter().map(|&x| [(x, 0.0), (x, y_max)]).collect();
    let marker_style = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);

    // Markers first so the cost series draw on top of them. Always drawn with
    // the Braille marker so they read as thin dotted lines, even on the
    // per-turn chart whose bars use the chunkier HalfBlock marker.
    let mut datasets: Vec<Dataset> = marker_lines
        .iter()
        .map(|line| {
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(marker_style)
                .data(line)
        })
        .collect();
    datasets.extend(series.iter().map(|(name, style, graph_type, data)| {
        // Leave dimmed underlay series (empty name) out of the legend.
        let mut ds = Dataset::default()
            .marker(marker)
            .graph_type(*graph_type)
            .style(*style)
            .data(data);
        if !name.is_empty() {
            ds = ds.name(*name);
        }
        ds
    }));

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
        .legend_position(show_legend.then_some(LegendPosition::TopLeft));
    f.render_widget(chart, area);

    draw_segment_labels(f, area, n, &y_label_widths(y_max), segment_labels);
}

/// The y-axis label strings cost_chart renders, so the segment-label placement
/// can account for the left margin ratatui reserves for them.
fn y_label_widths(y_max: f64) -> [u16; 3] {
    [
        "$0".len() as u16,
        format!("${:.3}", y_max / 2.0).len() as u16,
        format!("${:.3}", y_max).len() as u16,
    ]
}

/// Draw segment cost labels along the top of a chart's plot area. We replicate
/// ratatui's internal graph-area geometry (block border + y-axis label column)
/// to map each label's data-x to a screen column.
fn draw_segment_labels(
    f: &mut Frame,
    area: Rect,
    n: f64,
    y_label_widths: &[u16; 3],
    segment_labels: &[(f64, String)],
) {
    if segment_labels.is_empty() || n <= 0.0 {
        return;
    }
    let inner = area.inner(Margin::new(1, 1)); // inside the block border
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    // ratatui reserves the widest y label (capped at 1/3 width) plus one column
    // for the y-axis line itself, to the left of the plot.
    let y_label_w = (*y_label_widths.iter().max().unwrap()).min(inner.width / 3);
    let graph_x = inner.left() + y_label_w + 1;
    let graph_w = inner.right().saturating_sub(graph_x);
    if graph_w == 0 {
        return;
    }
    let style = Style::default().fg(Color::Gray).add_modifier(Modifier::DIM);
    for (vx, text) in segment_labels {
        // x-axis bounds are [0.5, n + 0.5]; map the midpoint into the plot.
        let frac = ((vx - 0.5) / n).clamp(0.0, 1.0);
        let center = graph_x as f64 + frac * graph_w.saturating_sub(1) as f64;
        let w = text.chars().count() as u16;
        let start = (center - w as f64 / 2.0).round().max(graph_x as f64) as u16;
        let start = start.min(graph_x + graph_w.saturating_sub(w));
        f.buffer_mut().set_stringn(start, inner.top(), text, graph_w as usize, style);
    }
}

/// Format a token count compactly with a magnitude suffix, e.g. 950 -> "950",
/// 12000 -> "12k", 12300 -> "12.3k", 1_500_000 -> "1.5m". A trailing ".0" is
/// dropped so round values stay clean.
fn fmt_tok(n: u64) -> String {
    fn scaled(v: f64, suffix: &str) -> String {
        if (v.fract()).abs() < 0.05 {
            format!("{:.0}{suffix}", v)
        } else {
            format!("{:.1}{suffix}", v)
        }
    }
    if n >= 1_000_000 {
        scaled(n as f64 / 1e6, "m")
    } else if n >= 1_000 {
        scaled(n as f64 / 1e3, "k")
    } else {
        n.to_string()
    }
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
