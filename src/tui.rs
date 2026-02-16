use crate::app::App;
use crate::ui;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::io::Stdout;
use std::time::Duration;

pub fn run_tui(app: &mut App) -> Result<()> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    let app_result = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    app_result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                if app.show_help {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                        app.toggle_help();
                    }
                    if matches!(key.code, KeyCode::Char('q')) {
                        app.should_quit = true;
                    }
                    continue;
                }

                if app.is_load_picker_open() {
                    match key.code {
                        KeyCode::Esc => app.close_load_picker(),
                        KeyCode::Enter => {
                            app.confirm_load_picker_selection();
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.prev_load_picker_item(),
                        KeyCode::Down | KeyCode::Char('j') => app.next_load_picker_item(),
                        KeyCode::Backspace => app.load_picker_backspace(),
                        KeyCode::Char(c) if c != '\n' => {
                            app.load_picker_push_char(c);
                        }
                        _ => {}
                    }
                    continue;
                }

                if app.is_edit_mode() {
                    if app.is_edit_value_mode() {
                        if key.code == KeyCode::Esc {
                            app.close_edit_field_input();
                            continue;
                        }

                        if key.code == KeyCode::Enter {
                            app.commit_edit_field_input();
                            continue;
                        }

                        if key.code == KeyCode::Backspace {
                            app.edit_field_input_backspace();
                            continue;
                        }

                        if key.code == KeyCode::Delete {
                            app.edit_field_input_delete();
                            continue;
                        }

                        if key.code == KeyCode::Left || key.code == KeyCode::Right {
                            if key.code == KeyCode::Left {
                                app.edit_field_move_cursor_left();
                            } else {
                                app.edit_field_move_cursor_right();
                            }
                            continue;
                        }

                        if key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.set_edit_field_normal_mode();
                            app.status = "edit value normal mode".to_string();
                            continue;
                        }

                        if app.is_edit_field_normal_mode() {
                            match key.code {
                                KeyCode::Char('h') => app.edit_field_move_cursor_left(),
                                KeyCode::Char('l') => app.edit_field_move_cursor_right(),
                                KeyCode::Char('i') => app.set_edit_field_insert_mode(),
                                KeyCode::Char('a') => {
                                    app.edit_field_move_cursor_right();
                                    app.set_edit_field_insert_mode();
                                }
                                KeyCode::Char('x') => app.edit_field_input_delete(),
                                _ => {}
                            }
                            continue;
                        }

                        match key.code {
                            KeyCode::Char(c) if c != '\n' => app.edit_field_input_push_char(c),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Esc => app.close_edit_menu(),
                            KeyCode::Enter => app.begin_edit_field_input(),
                            KeyCode::Char(' ') => app.begin_edit_field_input(),
                            KeyCode::Char('j') | KeyCode::Down => app.next_edit_field(),
                            KeyCode::Char('k') | KeyCode::Up => app.prev_edit_field(),
                            _ => {}
                        }
                    }
                    continue;
                }

                if app.is_command_mode() {
                    match key.code {
                        KeyCode::Esc => app.clear_command(),
                        KeyCode::Enter => app.execute_command(),
                        KeyCode::Tab => app.auto_complete_command(),
                        KeyCode::Backspace => app.command_backspace(),
                        KeyCode::Left => app.command_move_cursor_left(),
                        KeyCode::Right => app.command_move_cursor_right(),
                        KeyCode::Delete => app.command_input_delete(),
                        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.set_command_normal_mode();
                            app.status = "command normal mode".to_string();
                        }
                        _ if app.is_command_normal_mode() => {
                            match key.code {
                                KeyCode::Char('h') => app.command_move_cursor_left(),
                                KeyCode::Char('l') => app.command_move_cursor_right(),
                                KeyCode::Char('i') => app.set_command_insert_mode(),
                                _ => {}
                            }
                        }
                        KeyCode::Char(c) => {
                            if c != '\n' && app.is_command_insert_mode() {
                                app.push_command_char(c);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Esc => {}
                    KeyCode::Tab => app.focus_next_panel(),
                    KeyCode::BackTab => app.focus_previous_panel(),
                    KeyCode::Left | KeyCode::Char('h') => app.focus_measures_panel(),
                    KeyCode::Right | KeyCode::Char('l') => app.focus_details_panel(),
                    KeyCode::Enter => app.open_edit_menu(),
                    KeyCode::Char('j') | KeyCode::Down if app.is_measures_panel_focused() => {
                        app.next_measure()
                    }
                    KeyCode::Char('k') | KeyCode::Up if app.is_measures_panel_focused() => {
                        app.prev_measure()
                    }
                    KeyCode::Char('n') => {
                        app.add_measure("new.measure.path")
                    }
                    KeyCode::Char('d') => app.remove_selected(),
                    KeyCode::Char('e') => app.open_edit_menu(),
                    KeyCode::Char('a') => app.add_measure("new.measure.path"),
                    KeyCode::Char('s') => app.save_stub(),
                    KeyCode::Char('?') => {
                        app.toggle_help();
                    }
                    KeyCode::Char('/') => app.begin_command(),
                    KeyCode::Char('r') => {
                        app.status = "Reload not yet implemented".to_string();
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
