use crate::app::{App, EditableField, FocusedPanel};
use crate::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block,
        BorderType,
        Borders,
        List,
        ListItem,
        ListState,
        Paragraph,
    },
};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BASE).fg(theme::TEXT)),
        frame.area(),
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11),
            Constraint::Min(6),
            Constraint::Length(4),
        ])
        .split(frame.area());

    render_header(frame, layout[0]);
    render_body(frame, layout[1], app);
    render_footer(frame, layout[2], app);

    if app.show_help {
        render_help_overlay(frame, frame.area());
    }

    if app.is_load_picker_open() {
        render_load_picker_overlay(frame, frame.area(), app);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(48), Constraint::Min(20)])
        .split(area);

    let logo_lines: Vec<Line> = theme::LOGO
        .lines()
        .map(|line| Line::from(Span::styled(
            line,
            Style::default().fg(theme::TEAL),
        )))
        .collect();

    frame.render_widget(
        Paragraph::new(logo_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border())
                .title(" bptui ")
                .title_style(theme::title()),
        ),
        sections[0],
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("boilerplate measures TUI", theme::title())),
            Line::from(""),
            Line::from(Span::styled(
                theme::TAGLINE,
                Style::default().fg(theme::SUBTEXT0).add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "learn, inspect, edit, validate — all in one terminal.",
                Style::default().fg(theme::SAPPHIRE),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press ? for command legend, q to quit",
                Style::default().fg(theme::PEACH),
            )),
        ])
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border())
                .title(" mission ")
                .title_style(theme::panel_title()),
        ),
        sections[1],
    );
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let measures_focused = matches!(app.focused_panel(), FocusedPanel::Measures);
    let details_focused = app.is_edit_mode()
        || matches!(app.focused_panel(), FocusedPanel::Details);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(48), Constraint::Min(20)])
        .split(area);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(" measures ")
        .title_style(theme::panel_title())
        .border_style(if measures_focused {
            theme::help_border()
        } else {
            theme::border()
        })
        .style(Style::default().bg(theme::BASE));
    let list_area = list_block.inner(chunks[0]);
    frame.render_widget(list_block, chunks[0]);

    let items: Vec<ListItem> = app
        .measures
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let name = &item.name;
            if Some(index) == Some(app.selected_measure) {
                let selected = format!("▶ {}", name);
                ListItem::new(Span::styled(selected, theme::selection()))
            } else {
                ListItem::new(Span::styled(name, theme::text()))
            }
        })
        .collect();

    let mut list_state = ListState::default();
    if !app.measures.is_empty() {
        list_state.select(Some(app.selected_measure));
    }

    let list = List::new(items).highlight_style(theme::selection()).highlight_symbol(" ");
    frame.render_stateful_widget(list, list_area, &mut list_state);

    if list_area.height >= 2 && list_area.width >= 10 {
        let count_area = Rect {
            x: list_area.x,
            y: list_area.y + list_area.height - 1,
            width: list_area.width,
            height: 1,
        };
        let count_footer = Paragraph::new(Line::from(Span::styled(
            format!("{} measure(s).", app.measures.len()),
            Style::default().fg(theme::SUBTEXT0),
        )));
        frame.render_widget(count_footer, count_area);
    }

    if app.is_edit_mode() {
        if app.is_edit_value_mode() {
            render_edit_value_panel(frame, chunks[1], app, true);
        } else {
            render_edit_menu_panel(frame, chunks[1], app, true);
        }
    } else if app.measures.is_empty() {
        let no_data_border = if details_focused {
            theme::help_border()
        } else {
            theme::border()
        };

        frame.render_widget(
            Paragraph::new(vec![
                Line::from("No measures loaded."),
                Line::from(""),
                Line::from("Press /, then type /load and Enter to open the in-app picker."),
                Line::from("Type a few letters (for example: boilerplate_unified), Enter to load."),
                Line::from("Fallback: /load <path-to-measures-file>.json"),
                Line::from(""),
                Line::from("Tip: copy the path from your terminal or from yazi."),
            ])
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" details ")
                    .title_style(theme::panel_title())
                    .border_style(no_data_border)
                    .style(Style::default().bg(theme::BASE)),
            ),
            chunks[1],
        );
    } else {
        render_measure_details_panel(frame, chunks[1], app, details_focused);
    }
}

fn render_measure_details_panel(frame: &mut Frame<'_>, area: Rect, app: &App, focused: bool) {
    let selected_record = app.selected_measure_record();
    let selected = selected_record.map(|value| value.name.as_str()).unwrap_or("<none>");
    let validation = selected_record
        .map(|record| record.validation())
        .unwrap_or_else(|| app.validate_measure(selected));
    let status = if app.dirty { "⚠ unsaved" } else { "✓ clean" };
    let mut issues = Vec::new();
    if !validation.description {
        issues.push("description");
    }
    if !validation.reference {
        issues.push("reference");
    }
    if !validation.waves {
        issues.push("waves");
    }
    if !validation.keywords {
        issues.push("keywords");
    }
    if !validation.standardised {
        issues.push("standardised");
    }

    let mut detail_lines = vec![
        Line::from(Span::styled(
            format!("selected: {}", selected),
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "path:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(Span::styled(selected, Style::default().fg(theme::TEAL))),
        Line::from(Span::styled(
            "description:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(
            preview_text(
                selected_record
                    .and_then(|record| record.description.as_ref())
                    .map(|text| text.as_str())
                    .unwrap_or("—"),
                140,
            ),
        ),
        Line::from(Span::styled(
            "reference:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(
            preview_text(
                selected_record
                    .and_then(|record| record.reference.as_ref())
                    .map(|text| text.as_str())
                    .unwrap_or("—"),
                140,
            ),
        ),
        Line::from(Span::styled(
            "waves:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(
            preview_text(
                selected_record
                    .and_then(|record| record.waves.as_ref())
                    .map(|text| text.as_str())
                    .unwrap_or("—"),
                140,
            ),
        ),
        Line::from(Span::styled(
            "keywords:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(
            preview_text(
                selected_record
                    .and_then(|record| record.keywords.as_ref())
                    .map(|text| text.as_str())
                    .unwrap_or("—"),
                140,
            ),
        ),
        Line::from(Span::styled(
            "standardised:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(
            selected_record
                .and_then(|record| record.standardised)
                .map(|value| if value { "true" } else { "false" })
                .unwrap_or("false"),
        ),
        Line::from(Span::styled(
            "standardised_date:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(
            selected_record
                .and_then(|record| record.standardised_date.as_ref())
                .map(|text| text.as_str())
                .unwrap_or("—"),
        ),
        Line::from(Span::styled(
            "items:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(""),
    ];

    if let Some(record) = selected_record {
        if record.items.is_empty() {
            detail_lines.push(Line::from("  —"));
        } else {
            for item in record.items.iter().take(3) {
                detail_lines.push(Line::from(format!(
                    "  {}",
                    preview_text(item, 120),
                )));
            }
            if record.items.len() > 3 {
                detail_lines.push(Line::from(format!("  ... and {} more", record.items.len() - 3)));
            }
        }
    } else {
        detail_lines.push(Line::from("  —"));
    }

    detail_lines.push(Line::from(""));
    detail_lines.extend(vec![
        Line::from(Span::styled(
            "fields:",
            Style::default().fg(theme::SAPPHIRE).add_modifier(Modifier::ITALIC),
        )),
        field_line("description", validation.description),
        field_line("reference", validation.reference),
        field_line("waves", validation.waves),
        field_line("keywords", validation.keywords),
        field_line("standardised", validation.standardised),
    ]);

    detail_lines.push(Line::from(""));
    detail_lines.push(Line::from(Span::styled(
        format!("status: {}", status),
        if app.dirty {
            Style::default().fg(theme::YELLOW)
        } else {
            Style::default().fg(theme::GREEN)
        },
    )));
    detail_lines.push(Line::from(""));

    if !issues.is_empty() {
        detail_lines.push(Line::from(Span::styled(
            format!("validation: {}", issues.join(", ")),
            Style::default().fg(theme::YELLOW),
        )));
        detail_lines.push(Line::from(""));
    }

    detail_lines.push(Line::from(Span::styled(
        "Tip: press Enter or e to open edit menu for all fields, and /find <query> to jump.",
        Style::default().fg(theme::SUBTEXT0),
    )));

    frame.render_widget(
        Paragraph::new(detail_lines)
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" details ")
                    .title_style(theme::panel_title())
                    .border_style(if focused {
                        theme::help_border()
                    } else {
                        theme::border()
                    })
                    .style(Style::default().bg(theme::BASE)),
            ),
        area,
    );
}

fn render_edit_menu_panel(frame: &mut Frame<'_>, area: Rect, app: &App, focused: bool) {
    let selected = app
        .selected_edit_field()
        .and_then(|selected| app.edit_menu_fields().iter().position(|field| *field == selected))
        .unwrap_or(0);

    let selected_measure = app
        .selected_measure_name()
        .unwrap_or("<no measure selected>");
    let mut lines = vec![
        Line::from(Span::styled("measure edit menu", theme::title())),
        Line::from(Span::styled(
            format!("record: {selected_measure}"),
            Style::default().fg(theme::SUBTEXT0),
        )),
        Line::from(""),
        Line::from(
            "Use j/k (or ↑/↓) to choose a field. Enter edits values. Space toggles booleans.",
        ),
        Line::from(""),
    ];

    for (index, field) in app.edit_menu_fields().iter().enumerate() {
        let label = field_label(*field);
        let value = app.edit_menu_field_value(*field);
        let marker = if index == selected { "▶" } else { " " };
        let (state_glyph, state_text) = if field.is_toggleable() {
            if value == "true" {
                ("[x]", state_add_hint("on"))
            } else {
                ("[ ]", state_add_hint("off"))
            }
        } else {
            ("   ", preview_text(&value, 80))
        };
        let style = if index == selected {
            theme::selection()
        } else {
            theme::text()
        };

        lines.push(Line::from(Span::styled(
            format!("{marker} {state_glyph} {label:16} {state_text}"),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Enter: edit value · Space: toggle field"));
    lines.push(Line::from("Esc: exit edit menu"));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(Style::default().bg(theme::BASE).fg(theme::TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if focused {
                        theme::help_border()
                    } else {
                        theme::border()
                    })
                    .title(" Edit measure ")
                    .title_style(theme::title()),
            ),
        area,
    );
}

fn render_edit_value_panel(frame: &mut Frame<'_>, area: Rect, app: &App, focused: bool) {
    let field = app
        .selected_edit_field()
        .unwrap_or(EditableField::Description);
    let label = field_label(field);
    let selected_measure = app
        .selected_measure_name()
        .unwrap_or("<no measure selected>");
    let current_value = app.edit_menu_field_value(field);
    let mode = if app.is_edit_field_insert_mode() {
        "INSERT"
    } else {
        "NORMAL"
    };
    let value_cursor = app.edit_field_cursor.min(app.edit_field_input.len());
    let value_cursor_spans = render_text_with_cursor(
        &app.edit_field_input,
        value_cursor,
        Style::default().fg(theme::YELLOW),
        Style::default().bg(theme::YELLOW).fg(theme::BASE),
    );

    let mut lines = vec![
        Line::from(Span::styled("edit field", theme::title())),
        Line::from(""),
        Line::from(Span::styled(
            format!("record: {selected_measure}"),
            Style::default().fg(theme::SUBTEXT0),
        )),
        Line::from(format!("field: {label}")),
        Line::from(format!("current: {}", preview_text(&current_value, 120))),
        Line::from(""),
        Line::from(Span::styled(
            format!("mode: {mode}"),
            Style::default().fg(theme::SAPPHIRE),
        )),
    ];
    let mut new_value_line = vec![Span::styled("new value: ", Style::default().fg(theme::YELLOW))];
    new_value_line.extend(value_cursor_spans);
    lines.push(Line::from(new_value_line));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Enter: save changes · Esc: back to field menu · Ctrl+n: normal mode · i/a/x in normal for insert/cursor/delete",
    ));
    lines.push(Line::from("Tip: leave empty and press Enter to keep current value."));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(Style::default().bg(theme::BASE).fg(theme::TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if focused {
                        theme::help_border()
                    } else {
                        theme::border()
                    })
                    .title(" Edit value ")
                    .title_style(theme::title()),
            ),
        area,
    );
}
fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let panel_hint = if app.is_measures_panel_focused() {
        " Tab: focus details"
    } else {
        " Tab: focus measures"
    };

    let keybinds = if app.is_load_picker_open() {
        String::from(" j/k: move   Enter: load selected   Esc: cancel   Backspace: delete text   type to filter")
    } else if app.is_edit_mode() {
        if app.is_edit_value_mode() {
            String::from(" Enter: save   Esc: cancel   Backspace/Del: delete text   Ctrl+n: normal mode   type value")
        } else {
            String::from(" j/k: move   Enter: edit field   Space: toggle bool   Esc: close menu")
        }
    } else if app.is_command_mode() {
        String::from(" Enter: run command   Esc: cancel   Backspace: clear   Ctrl+n: normal mode   Tab: autocomplete   h/l in normal for cursor")
    } else if app.measures.is_empty() {
        format!("{}   Enter: edit selected   /: open command mode   ? : help   q: quit", panel_hint)
    } else {
        format!(
            "{}   j/k: move   Enter: edit   d: delete   e: edit   s: save   /: command   ? : help   q: quit",
            panel_hint
        )
    };

    let status = if app.is_load_picker_open() {
        Line::from(vec![
            Span::styled("load: ", Style::default().fg(theme::PEACH)),
            Span::styled(
                app.load_picker_query.clone(),
                Style::default().fg(theme::YELLOW),
            ),
            Span::styled("█", Style::default().fg(theme::TEXT)),
        ])
    } else if app.is_command_mode() {
        let mode = if app.is_command_insert_mode() {
            "INSERT"
        } else {
            "NORMAL"
        };
        let cursor = app.command_cursor.min(app.command_buffer.len());
        let command_with_cursor = render_text_with_cursor(
            &app.command_buffer,
            cursor,
            Style::default().fg(theme::YELLOW),
            Style::default().bg(theme::YELLOW).fg(theme::BASE),
        );
        let mut status_spans = vec![
            Span::styled("command ", Style::default().fg(theme::PEACH)),
            Span::styled(format!("[{mode}] "), Style::default().fg(theme::YELLOW)),
        ];
        status_spans.extend(command_with_cursor);
        Line::from(status_spans)
    } else {
        Line::from(vec![
            Span::styled("🧪 ", Style::default().fg(theme::PEACH)),
            Span::styled(format!("status: {}", app.status), theme::text()),
        ])
    };

    let suggestions = if app.is_command_mode() {
        app.command_suggestions()
            .into_iter()
            .take(3)
            .collect::<Vec<_>>()
            .join(" • ")
    } else {
        String::new()
    };

    frame.render_widget(
        Paragraph::new(vec![
            status,
            Line::from(""),
            Line::from(Span::styled(
                if app.is_command_mode() && !suggestions.is_empty() {
                    format!("{keybinds} | {suggestions}")
                } else {
                    keybinds.to_string()
                },
                Style::default().fg(theme::SAPPHIRE),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if app.dirty {
                    Style::default().fg(theme::YELLOW)
                } else {
                    theme::border()
                })
                .title(" quick keys ")
                .title_style(theme::panel_title()),
        )
        .alignment(Alignment::Left),
        area,
    );
}

fn field_line(label: &str, status: bool) -> Line<'static> {
    let badge = if status {
        Span::styled(" [ok] ".to_string(), theme::chip_ok())
    } else {
        Span::styled(" [warn] ".to_string(), theme::chip_warn())
    };

    Line::from(vec![
        Span::styled(format!("  {}", label), Style::default().fg(theme::TEXT)),
        badge,
    ])
}

fn state_add_hint(value: &str) -> String {
    if value == "true" {
        "on (space to disable)".to_string()
    } else {
        "off (space to enable)".to_string()
    }
}

fn field_label(field: EditableField) -> &'static str {
    match field {
        EditableField::Name => "Name",
        EditableField::Description => "Description",
        EditableField::Reference => "Reference",
        EditableField::Waves => "Waves",
        EditableField::Keywords => "Keywords",
        EditableField::Standardised => "Standardised",
        EditableField::StandardisedDate => "Standardised date",
        EditableField::Items => "Items",
    }
}

fn preview_text(value: &str, max_chars: usize) -> String {
    let clean = value.replace('\n', " ").replace('\r', " ");
    let mut chars = clean.chars();
    let max_chars = max_chars.max(3);

    let total = clean.chars().count();
    if total <= max_chars {
        return clean;
    }

    let visible = max_chars.saturating_sub(3);
    let snippet: String = chars.by_ref().take(visible).collect();
    format!("{snippet}...")
}

fn render_text_with_cursor(
    text: &str,
    cursor: usize,
    text_style: Style,
    cursor_style: Style,
) -> Vec<Span<'_>> {
    let cursor = cursor.min(text.len());
    let before = &text[..cursor];
    let rest = &text[cursor..];

    let mut spans = Vec::new();

    if !before.is_empty() {
        spans.push(Span::styled(before, text_style));
    }

    if rest.is_empty() {
        spans.push(Span::styled(" ", cursor_style));
        return spans;
    }

    let mut chars = rest.char_indices();
    let (offset, first_char) = match chars.next() {
        Some(value) => value,
        None => {
            spans.push(Span::styled(" ", cursor_style));
            return spans;
        }
    };

    let next = cursor + offset + first_char.len_utf8();
    let current = &text[cursor..next];
    spans.push(Span::styled(current, cursor_style));

    let after = &text[next..];
    if !after.is_empty() {
        spans.push(Span::styled(after, text_style));
    }

    spans
}

fn render_help_overlay(frame: &mut Frame<'_>, area: Rect) {
    let width = area.width.min(72).max(36);
    let height = area.height.min(20).max(16);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };

    let help_lines = vec![
        Line::from("Navigation"),
        Line::from("  Tab/←→/hjkl   switch panel focus (list/details)"),
        Line::from("  j/k or ↑/↓   move through measures (when list panel focused)"),
        Line::from("  Enter / e      open edit menu for selected measure"),
        Line::from(""),
        Line::from("Edit actions"),
        Line::from("  n or a        add a placeholder measure"),
        Line::from("  d             delete selected measure"),
        Line::from("  s             save checkpoint"),
        Line::from(""),
        Line::from("Command mode"),
        Line::from("  /             open command bar"),
        Line::from("  Ctrl+n        enter normal mode in command/edit value input"),
        Line::from("  /help         show this panel"),
        Line::from("  /save         same as save"),
        Line::from("  /history      list recent checkpoints"),
        Line::from("  /restore <n>  restore checkpoint n from the /history list"),
        Line::from("  /revert /undo go back one checkpoint and save a new checkpoint"),
        Line::from("  /add [path]   add placeholder or provided path"),
        Line::from("  /find <query> select next matching measure"),
        Line::from("  /edit <field> <value>  edit one field of selected measure"),
        Line::from("                fields: name, description, reference, waves, keywords, standardised, standardised_date, items"),
        Line::from("  /e            open edit menu for selected measure"),
        Line::from("  /load          open the in-app file picker"),
        Line::from("  /load <file>   direct path (absolute, quoted, or ~/style paths)"),
        Line::from("                checkpoints are append-only; restore/revert creates a new checkpoint"),
        Line::from("  /reload       placeholder reload status"),
        Line::from("  /quit or /q   quit"),
        Line::from("  /clear        clear status message"),
        Line::from("  Tab           autocomplete matching commands"),
        Line::from("  h/l in normal mode: move cursor"),
        Line::from("  i/a/x in edit-value normal mode: insert/cursor-right/delete"),
        Line::from(""),
        Line::from("Esc closes this panel, q quits from normal mode."),
    ];

    let mut popup_lines = help_lines;
    popup_lines.insert(
        0,
        Line::from(Span::styled(
            "bptui lab keyboard help".to_uppercase(),
            theme::title(),
        )),
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(theme::help_border())
        .title(" HELP ")
        .title_style(theme::title());
    frame.render_widget(
        Paragraph::new(popup_lines)
            .alignment(Alignment::Left)
            .style(Style::default().bg(theme::BASE).fg(theme::TEXT))
            .block(block),
        popup,
    );
}

fn render_load_picker_overlay(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let width = area.width.min(100).max(48);
    let height = area.height.min(24).max(12);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };

    let candidates = app.load_picker_candidates();
    let selected = if candidates.is_empty() {
        0
    } else {
        app.load_picker_selection.min(candidates.len() - 1)
    };
    let visible = usize::from((height.saturating_sub(8)).max(6));
    let start = selected.saturating_sub(visible / 2);
    let end = (start + visible).min(candidates.len());

    let mut lines = vec![
        Line::from(Span::styled("load file picker", theme::title())),
        Line::from(format!("matches: {} (type to filter)", candidates.len())),
        Line::from(""),
        Line::from(format!("filter: {}", app.load_picker_query)),
        Line::from(""),
    ];

    for (index, candidate) in candidates[start..end].iter().enumerate() {
        let is_selected = start + index == selected;
        let mut row = format!("  {candidate}");
        if is_selected {
            row = format!("▶ {row}");
        }
        let style = if is_selected {
            theme::selection()
        } else {
            theme::text()
        };
        lines.push(Line::from(Span::styled(row, style)));
    }

    if candidates.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(
            "No matching files yet. Keep typing, or /load <path> to force.",
        ));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(Style::default().bg(theme::BASE).fg(theme::TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::help_border())
                    .title(" Load file ")
                    .title_style(theme::title()),
            ),
        popup,
    );
}
