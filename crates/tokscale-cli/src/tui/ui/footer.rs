use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::spinner::{get_phase_message, get_scanner_spans};
use super::widgets::{format_cost, format_tokens};
use crate::tui::app::{App, ClickAction, SortField, Tab};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border))
        .style(Style::default().bg(app.theme.background));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into 3 rows: sources+sort, help text, status
    let row_constraints = if inner.height >= 3 {
        vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    } else if inner.height >= 2 {
        vec![Constraint::Length(1), Constraint::Length(1)]
    } else {
        vec![Constraint::Length(1)]
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    render_main_row(frame, app, rows[0]);

    if rows.len() >= 2 {
        render_help_row(frame, app, rows[1]);
    }

    if rows.len() >= 3 {
        render_status_row(frame, app, rows[2]);
    }
}

fn render_main_row(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_very_narrow = app.is_very_narrow();

    // Split into left (sort buttons) and right (totals)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left side: sort buttons
    if !is_very_narrow {
        let mut spans: Vec<Span> = Vec::new();
        let mut x_offset = chunks[0].x;

        spans.push(Span::styled("Sort: ", Style::default().fg(app.theme.muted)));
        x_offset += 6;

        let sort_buttons = [
            (SortField::Date, "Date"),
            (SortField::Cost, "Cost"),
            (SortField::Tokens, "Tokens"),
        ];

        for (field, label) in sort_buttons {
            let is_active = app.sort_field == field;
            let style = if is_active {
                Style::default()
                    .fg(app.theme.foreground)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.muted)
            };

            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));

            let btn_width = label.len() as u16;
            app.add_click_area(
                Rect::new(x_offset, chunks[0].y, btn_width, 1),
                ClickAction::Sort(field),
            );
            x_offset += btn_width + 1;
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, chunks[0]);
    }

    // Right side: scroll info | tokens | cost
    let mut right_spans: Vec<Span> = Vec::new();

    // Scroll position indicator for Overview tab
    if app.current_tab == Tab::Overview {
        let total_models = app.data.models.len();
        if total_models > app.max_visible_items && app.max_visible_items > 0 {
            let start = app.scroll_offset + 1;
            let end = (app.scroll_offset + app.max_visible_items).min(total_models);
            if !is_very_narrow {
                right_spans.push(Span::styled(
                    format!("↓ {}-{} of {} ", start, end, total_models),
                    Style::default().fg(app.theme.muted),
                ));
                right_spans.push(Span::styled("| ", Style::default().fg(app.theme.muted)));
            }
        }
    }

    let totals = current_totals(app);

    // Total tokens
    right_spans.push(Span::styled(
        format_tokens(totals.tokens),
        Style::default().fg(Color::Cyan),
    ));
    if !is_very_narrow {
        right_spans.push(Span::styled(
            " tokens",
            Style::default().fg(app.theme.muted),
        ));
    }

    right_spans.push(Span::styled(" | ", Style::default().fg(app.theme.muted)));

    // Total cost
    right_spans.push(Span::styled(
        format_cost(totals.cost),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));

    // Current list count
    if !is_very_narrow {
        let count_label = current_count_label(app);
        right_spans.push(Span::styled(
            count_label,
            Style::default().fg(app.theme.muted),
        ));
    }

    let right_line = Line::from(right_spans);
    let right_para = Paragraph::new(right_line).alignment(Alignment::Right);
    frame.render_widget(right_para, chunks[1]);
}

#[derive(Debug, Clone, Copy, Default)]
struct ViewTotals {
    tokens: u64,
    cost: f64,
}

impl ViewTotals {
    fn add(&mut self, tokens: u64, cost: f64) {
        self.tokens = self.tokens.saturating_add(tokens);
        if cost.is_finite() && cost >= 0.0 {
            self.cost += cost;
        }
    }
}

fn current_totals(app: &App) -> ViewTotals {
    match app.current_tab {
        Tab::Models if app.is_model_session_detail_active() => {
            let mut totals = ViewTotals::default();
            for row in app.get_sorted_model_message_rows() {
                totals.add(row.tokens.total(), row.cost);
            }
            totals
        }
        Tab::Models if app.is_model_detail_active() => {
            let mut totals = ViewTotals::default();
            for row in app.get_sorted_model_session_rows() {
                totals.add(row.tokens.total(), row.cost);
            }
            totals
        }
        Tab::Daily if app.is_daily_session_detail_active() => {
            let mut totals = ViewTotals::default();
            for row in app.get_sorted_daily_message_rows() {
                totals.add(row.tokens.total(), row.cost);
            }
            totals
        }
        Tab::Daily if app.is_daily_model_detail_active() => {
            let mut totals = ViewTotals::default();
            for row in app.get_sorted_daily_session_rows() {
                totals.add(row.tokens.total(), row.cost);
            }
            totals
        }
        Tab::Daily if app.is_daily_detail_active() => {
            let mut totals = ViewTotals::default();
            for row in app.get_sorted_daily_detail_rows() {
                totals.add(row.tokens.total(), row.cost);
            }
            totals
        }
        _ => ViewTotals {
            tokens: app.data.total_tokens,
            cost: app.data.total_cost,
        },
    }
}

fn current_count_label(app: &App) -> String {
    match app.current_tab {
        Tab::Models if app.is_model_session_detail_active() => {
            format!(" ({} requests)", app.get_sorted_model_message_rows().len())
        }
        Tab::Models if app.is_model_detail_active() => {
            format!(" ({} sessions)", app.get_sorted_model_session_rows().len())
        }
        Tab::Overview | Tab::Models => format!(" ({} models)", app.data.models.len()),
        Tab::Agents => format!(" ({} agents)", app.data.agents.len()),
        Tab::Daily if app.is_daily_session_detail_active() => {
            format!(" ({} requests)", app.get_sorted_daily_message_rows().len())
        }
        Tab::Daily if app.is_daily_model_detail_active() => {
            format!(" ({} sessions)", app.get_sorted_daily_session_rows().len())
        }
        Tab::Daily if app.is_daily_detail_active() => {
            format!(" ({} models)", app.get_sorted_daily_detail_rows().len())
        }
        Tab::Daily => format!(" ({} days)", app.data.daily.len()),
        Tab::Hourly => format!(" ({} hours)", app.data.hourly.len()),
        Tab::Minutely => format!(" ({} minutes)", app.data.minutely.len()),
        Tab::Stats | Tab::Usage => String::new(),
    }
}

fn render_help_row(frame: &mut Frame, app: &App, area: Rect) {
    let is_very_narrow = app.is_very_narrow();

    let spans = if is_very_narrow {
        let mut spans = vec![
            Span::styled("↑↓", Style::default().fg(app.theme.muted)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("←→", Style::default().fg(app.theme.muted)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("d/t/c", Style::default().fg(Color::Blue)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("[s]", Style::default().fg(Color::Cyan)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("[g]", Style::default().fg(Color::Cyan)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("[p]", Style::default().fg(Color::Magenta)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("[r]", Style::default().fg(Color::Yellow)),
            Span::styled("·", Style::default().fg(app.theme.muted)),
            Span::styled("q", Style::default().fg(app.theme.muted)),
        ];
        if app.current_tab == Tab::Daily {
            spans.push(Span::styled("·", Style::default().fg(app.theme.muted)));
            if app.is_daily_session_detail_active() {
                spans.push(Span::styled("esc", Style::default().fg(Color::Yellow)));
            } else if app.is_daily_model_detail_active() {
                spans.push(Span::styled("↵", Style::default().fg(Color::Yellow)));
                spans.push(Span::styled("·", Style::default().fg(app.theme.muted)));
                spans.push(Span::styled("esc", Style::default().fg(Color::Yellow)));
            } else if app.is_daily_detail_active() {
                spans.push(Span::styled("↵", Style::default().fg(Color::Yellow)));
                spans.push(Span::styled("·", Style::default().fg(app.theme.muted)));
                spans.push(Span::styled("esc", Style::default().fg(Color::Yellow)));
            } else {
                spans.push(Span::styled("↵", Style::default().fg(Color::Yellow)));
                spans.push(Span::styled("·", Style::default().fg(app.theme.muted)));
                spans.push(Span::styled("j", Style::default().fg(Color::Yellow)));
            }
        }
        if app.current_tab == Tab::Hourly {
            spans.push(Span::styled("·", Style::default().fg(app.theme.muted)));
            spans.push(Span::styled("v", Style::default().fg(Color::Yellow)));
        }
        spans
    } else {
        let mut spans = vec![
            Span::styled(
                "↑↓ scroll • ←→/tab view • ",
                Style::default().fg(app.theme.muted),
            ),
            Span::styled("[d/t/c:sort]", Style::default().fg(Color::Blue)),
            Span::styled(" • ", Style::default().fg(app.theme.muted)),
        ];
        if app.current_tab == Tab::Daily {
            if app.is_daily_session_detail_active() {
                spans.push(Span::styled(
                    "[esc:back]",
                    Style::default().fg(Color::Yellow),
                ));
            } else if app.is_daily_model_detail_active() {
                spans.push(Span::styled(
                    "[enter:requests]",
                    Style::default().fg(Color::Yellow),
                ));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    "[esc:back]",
                    Style::default().fg(Color::Yellow),
                ));
            } else if app.is_daily_detail_active() {
                spans.push(Span::styled(
                    "[enter:sessions]",
                    Style::default().fg(Color::Yellow),
                ));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    "[esc:back]",
                    Style::default().fg(Color::Yellow),
                ));
            } else {
                spans.push(Span::styled(
                    "[enter:details]",
                    Style::default().fg(Color::Yellow),
                ));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    "[j:today]",
                    Style::default().fg(Color::Yellow),
                ));
            }
            spans.push(Span::styled(" • ", Style::default().fg(app.theme.muted)));
        }
        if app.current_tab == Tab::Hourly {
            spans.push(Span::styled(
                "[v:profile]",
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(" • ", Style::default().fg(app.theme.muted)));
        }
        spans.push(Span::styled(
            "[s:sources]",
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::styled(" ", Style::default()));
        spans.push(Span::styled(
            format!("[g:{}]", app.group_by.borrow()),
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::styled(" • ", Style::default().fg(app.theme.muted)));
        spans.push(Span::styled(
            format!("[p:{}]", app.theme.name.as_str()),
            Style::default().fg(Color::Magenta),
        ));
        spans.push(Span::styled(" ", Style::default()));
        spans.push(Span::styled(
            if app.auto_refresh {
                format!("[R:auto {}s]", app.auto_refresh_interval.as_secs())
            } else {
                "[R:auto off]".to_string()
            },
            Style::default().fg(if app.auto_refresh {
                Color::Green
            } else {
                app.theme.muted
            }),
        ));
        spans.push(Span::styled(" • ", Style::default().fg(app.theme.muted)));
        spans.push(Span::styled(
            "[r:refresh]",
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::styled(
            " • e • q",
            Style::default().fg(app.theme.muted),
        ));
        spans
    };

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Data-source indicator label (#699): "local" when only this machine's
/// data is on screen, or "local+remote (N devices)" when server-side
/// aggregated stats are available for cross-checking.
fn data_source_label(app: &App) -> String {
    match app.remote_stats {
        Some(ref remote) => {
            let devices = if remote.device_count == 1 {
                "1 device".to_string()
            } else {
                format!("{} devices", remote.device_count)
            };
            format!("local+remote ({})", devices)
        }
        None => "local".to_string(),
    }
}

fn render_status_row(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();

    // Always-visible data-source indicator, so it is clear whether the
    // numbers on screen are local-only or backed by server-side aggregates.
    spans.push(Span::styled(
        data_source_label(app),
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    ));
    if let Some(ref remote) = app.remote_stats {
        spans.push(Span::styled(
            format!(
                " all devices: {} · {}",
                format_tokens(remote.total_tokens),
                format_cost(remote.total_cost)
            ),
            Style::default().fg(app.theme.muted),
        ));
    }
    spans.push(Span::styled(" • ", Style::default().fg(app.theme.muted)));

    if app.data.loading {
        let scanner_spans = get_scanner_spans(app.spinner_frame, &app.theme);
        spans.extend(scanner_spans);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            get_phase_message("parsing-sources"),
            Style::default().fg(app.theme.muted),
        ));
    } else if app.background_loading {
        if app.has_visible_data() {
            spans.push(Span::styled(
                "Refreshing cached data in background...",
                Style::default().fg(app.theme.muted),
            ));
        } else {
            let scanner_spans = get_scanner_spans(app.spinner_frame, &app.theme);
            spans.extend(scanner_spans);
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                get_phase_message("parsing-sources"),
                Style::default().fg(app.theme.muted),
            ));
        }
    } else if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            msg.clone(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        let elapsed = app.last_refresh.elapsed();
        let ago = if elapsed.as_secs() < 60 {
            format!("{}s ago", elapsed.as_secs())
        } else if elapsed.as_secs() < 3600 {
            format!("{}m ago", elapsed.as_secs() / 60)
        } else {
            format!("{}h ago", elapsed.as_secs() / 3600)
        };
        spans.push(Span::styled(
            format!("Last updated: {}", ago),
            Style::default().fg(app.theme.muted),
        ));

        if app.auto_refresh {
            spans.push(Span::styled(
                format!(" • Auto: {}s", app.auto_refresh_interval.as_secs()),
                Style::default().fg(app.theme.muted),
            ));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::TuiConfig;
    use crate::tui::data::{
        DailyModelInfo, DailySourceInfo, DailyUsage, MessageUsage, ModelUsage, TokenBreakdown,
        UsageData,
    };
    use chrono::NaiveDate;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::BTreeMap;

    fn make_app_on(tab: Tab) -> App {
        let config = TuiConfig {
            theme: "blue".to_string(),
            refresh: 0,
            sessions_path: None,
            clients: None,
            since: None,
            until: None,
            year: None,
            initial_tab: Some(tab),
        };
        App::new_with_cached_data(config, Some(UsageData::default())).unwrap()
    }

    fn make_app_on_with_data(tab: Tab, data: UsageData) -> App {
        let config = TuiConfig {
            theme: "blue".to_string(),
            refresh: 0,
            sessions_path: None,
            clients: None,
            since: None,
            until: None,
            year: None,
            initial_tab: Some(tab),
        };
        App::new_with_cached_data(config, Some(data)).unwrap()
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn tokens(input: u64) -> TokenBreakdown {
        TokenBreakdown {
            input,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            reasoning: 0,
        }
    }

    fn model_usage(model_key: &str, model: &str, cost: f64, input_tokens: u64) -> ModelUsage {
        ModelUsage {
            model_key: model_key.to_string(),
            model: model.to_string(),
            provider: "anthropic".to_string(),
            client: "claude".to_string(),
            workspace_key: None,
            workspace_label: None,
            tokens: tokens(input_tokens),
            cost,
            performance: Default::default(),
            session_count: 1,
        }
    }

    fn message_usage(
        date: &str,
        model_key: &str,
        session_id: &str,
        timestamp: i64,
        input_tokens: u64,
        cost: f64,
    ) -> MessageUsage {
        MessageUsage {
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            timestamp,
            source: "claude".to_string(),
            provider: "anthropic".to_string(),
            model_group_key: model_key.to_string(),
            model_key: model_key.to_string(),
            model: model_key.to_string(),
            color_key: model_key.to_string(),
            session_id: session_id.to_string(),
            workspace_key: None,
            workspace_label: None,
            agent: None,
            content_preview: Some("request".to_string()),
            tokens: tokens(input_tokens),
            cost,
            message_count: 1,
            duration_ms: None,
            request_start_timestamp: None,
            request_end_timestamp: timestamp,
            is_turn_start: true,
        }
    }

    fn daily_usage(
        date: &str,
        models: Vec<(&str, &str, &str, f64, u64)>,
        total_cost: f64,
        total_tokens: u64,
    ) -> DailyUsage {
        let mut model_breakdown = BTreeMap::new();
        for (model_key, provider, display_name, cost, input_tokens) in models {
            model_breakdown.insert(
                model_key.to_string(),
                DailyModelInfo {
                    provider: provider.to_string(),
                    display_name: display_name.to_string(),
                    color_key: display_name.to_string(),
                    tokens: tokens(input_tokens),
                    cost,
                    messages: 1,
                },
            );
        }

        let mut source_breakdown = BTreeMap::new();
        source_breakdown.insert(
            "claude".to_string(),
            DailySourceInfo {
                tokens: tokens(total_tokens),
                cost: total_cost,
                models: model_breakdown,
            },
        );

        DailyUsage {
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            tokens: tokens(total_tokens),
            cost: total_cost,
            source_breakdown,
            message_count: 1,
            turn_count: 1,
        }
    }

    fn assert_totals(app: &App, tokens: u64, cost: f64) {
        let totals = current_totals(app);
        assert_eq!(totals.tokens, tokens);
        assert!((totals.cost - cost).abs() < f64::EPSILON);
    }

    #[test]
    fn test_current_count_label_matches_active_tab() {
        assert_eq!(
            current_count_label(&make_app_on(Tab::Models)),
            " (0 models)"
        );
        assert_eq!(
            current_count_label(&make_app_on(Tab::Agents)),
            " (0 agents)"
        );
        assert_eq!(current_count_label(&make_app_on(Tab::Daily)), " (0 days)");
        assert_eq!(current_count_label(&make_app_on(Tab::Hourly)), " (0 hours)");
        assert_eq!(current_count_label(&make_app_on(Tab::Stats)), "");
    }

    #[test]
    fn test_current_count_label_minutely_when_flag_enabled() {
        let mut app = make_app_on(Tab::Models);
        app.settings.minutely_tab_enabled = true;
        app.current_tab = Tab::Minutely;
        assert_eq!(current_count_label(&app), " (0 minutes)");
    }

    #[test]
    fn test_data_source_label_local_without_remote_stats() {
        let app = make_app_on(Tab::Models);
        assert_eq!(data_source_label(&app), "local");
    }

    #[test]
    fn test_data_source_label_with_remote_stats() {
        let mut app = make_app_on(Tab::Models);
        app.remote_stats = Some(crate::tui::remote::RemoteStats {
            schema_version: 1,
            total_tokens: 1250,
            total_cost: 1.75,
            device_count: 2,
            last_submitted_at: None,
            days: Vec::new(),
            devices: Vec::new(),
            fetched_at_secs: 0,
            cached_for_user: "alice".to_string(),
            cached_for_api_url: "https://tokscale.ai".to_string(),
        });
        assert_eq!(data_source_label(&app), "local+remote (2 devices)");

        app.remote_stats.as_mut().unwrap().device_count = 1;
        assert_eq!(data_source_label(&app), "local+remote (1 device)");
    }

    #[test]
    fn test_current_totals_follow_model_drilldown() {
        let data = UsageData {
            models: vec![
                model_usage("target-model", "target-model", 2.0, 200),
                model_usage("other-model", "other-model", 1.0, 100),
            ],
            messages: vec![
                message_usage("2026-05-18", "target-model", "session-a", 1_779_000_001_000, 50, 0.5),
                message_usage("2026-05-18", "target-model", "session-a", 1_779_000_002_000, 50, 0.5),
                message_usage("2026-05-18", "target-model", "session-b", 1_779_000_003_000, 100, 1.0),
                message_usage("2026-05-18", "other-model", "session-c", 1_779_000_004_000, 100, 1.0),
            ],
            total_tokens: 300,
            total_cost: 3.0,
            ..Default::default()
        };
        let mut app = make_app_on_with_data(Tab::Models, data);

        assert_totals(&app, 300, 3.0);

        app.handle_key_event(key(KeyCode::Enter));
        assert!(app.is_model_detail_active());
        assert_totals(&app, 200, 2.0);
        assert_eq!(current_count_label(&app), " (2 sessions)");

        app.handle_key_event(key(KeyCode::Enter));
        assert!(app.is_model_session_detail_active());
        assert_totals(&app, 100, 1.0);
        assert_eq!(current_count_label(&app), " (1 requests)");
    }

    #[test]
    fn test_current_totals_follow_daily_drilldown() {
        let data = UsageData {
            daily: vec![
                daily_usage(
                    "2026-05-17",
                    vec![("old-model", "anthropic", "old-model", 1.0, 100)],
                    1.0,
                    100,
                ),
                daily_usage(
                    "2026-05-18",
                    vec![
                        ("a-target", "anthropic", "a-target", 2.0, 200),
                        ("z-other", "anthropic", "z-other", 1.0, 100),
                    ],
                    3.0,
                    300,
                ),
            ],
            messages: vec![
                message_usage("2026-05-18", "a-target", "session-a", 1_779_000_001_000, 50, 0.5),
                message_usage("2026-05-18", "a-target", "session-b", 1_779_000_002_000, 150, 1.5),
                message_usage("2026-05-18", "z-other", "session-c", 1_779_000_003_000, 100, 1.0),
                message_usage("2026-05-17", "old-model", "session-old", 1_778_000_001_000, 100, 1.0),
            ],
            total_tokens: 400,
            total_cost: 4.0,
            ..Default::default()
        };
        let mut app = make_app_on_with_data(Tab::Daily, data);

        assert_totals(&app, 400, 4.0);

        app.handle_key_event(key(KeyCode::Enter));
        assert!(app.is_daily_detail_active());
        assert_totals(&app, 300, 3.0);

        app.handle_key_event(key(KeyCode::Enter));
        assert!(app.is_daily_model_detail_active());
        assert_totals(&app, 200, 2.0);

        app.handle_key_event(key(KeyCode::Enter));
        assert!(app.is_daily_session_detail_active());
        assert_totals(&app, 150, 1.5);
    }
}
