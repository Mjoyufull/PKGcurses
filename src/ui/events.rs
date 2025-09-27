use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::ui::app::{App, ActivePane, InputMode};

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Editing => handle_editing_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit();
        }
        
        // Navigation - only works when focused on navigable panes
        KeyCode::Up | KeyCode::Char('k') => {
            match app.active_pane {
                ActivePane::Results | ActivePane::Installed => {
                    app.move_up();
                }
                _ => {}
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            match app.active_pane {
                ActivePane::Results | ActivePane::Installed => {
                    app.move_down();
                }
                _ => {}
            }
        }
        
        // Multi-selection with Ctrl+Space (only in Results pane)
        KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.active_pane == ActivePane::Results {
                app.toggle_package_selection();
            }
        }
        
        // Pane switching with Tab
        KeyCode::Tab => {
            app.switch_pane();
        }
        
        // Enter search mode only with specific keys
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.enter_search_mode();
        }
        
        // Page navigation - only in navigable panes
        KeyCode::PageUp => {
            match app.active_pane {
                ActivePane::Results | ActivePane::Installed => {
                    for _ in 0..10 {
                        app.move_up();
                    }
                }
                _ => {}
            }
        }
        KeyCode::PageDown => {
            match app.active_pane {
                ActivePane::Results | ActivePane::Installed => {
                    for _ in 0..10 {
                        app.move_down();
                    }
                }
                _ => {}
            }
        }
        
        // Home/End - only in navigable panes
        KeyCode::Home | KeyCode::Char('g') => {
            match app.active_pane {
                ActivePane::Results => {
                    app.selected_index = 0;
                    app.scroll_offset = 0;
                }
                ActivePane::Installed => {
                    app.installed_selected = 0;
                    app.installed_scroll = 0;
                }
                _ => {}
            }
        }
        KeyCode::End | KeyCode::Char('G') => {
            match app.active_pane {
                ActivePane::Results => {
                    app.selected_index = app.filtered_packages.len().saturating_sub(1);
                    let visible_items = app.get_results_visible_items();
                    app.scroll_offset = app.selected_index.saturating_sub(visible_items - 1);
                }
                ActivePane::Installed => {
                    app.installed_selected = app.installed_packages.len().saturating_sub(1);
                    let visible_items = app.get_installed_visible_items();
                    app.installed_scroll = app.installed_selected.saturating_sub(visible_items - 1);
                }
                _ => {}
            }
        }
        
        // Clear selection
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.clear_selection();
        }
        
        // Install selected packages
        KeyCode::Enter => {
            if app.get_selected_count() > 0 {
                app.start_installation();
            }
        }
        
        // DO NOT auto-enter search mode on typing - user must explicitly press '/' or 'i'
        
        _ => {}
    }
}

fn handle_editing_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        // Exit editing mode
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.active_pane = ActivePane::Results;
        }
        
        // Confirm search and move to results
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.active_pane = ActivePane::Results;
        }
        
        // Navigation in search - switch to results and navigate
        KeyCode::Up => {
            app.input_mode = InputMode::Normal;
            app.active_pane = ActivePane::Results;
            app.move_up();
        }
        KeyCode::Down => {
            app.input_mode = InputMode::Normal;
            app.active_pane = ActivePane::Results;
            app.move_down();
        }
        
        // Multi-selection with Ctrl+Space in search mode
        KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Toggle selection of current item if we have results
            if !app.filtered_packages.is_empty() {
                app.toggle_package_selection();
            }
        }
        
        // Regular space adds space to search input
        KeyCode::Char(' ') => {
            app.add_char(' ');
        }
        
        // Clear search with Ctrl+U (must come before general Char pattern)
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.clear_search();
        }
        
        // Text editing
        KeyCode::Char(c) => {
            app.add_char(c);
        }
        KeyCode::Backspace => {
            app.delete_char();
        }
        KeyCode::Delete => {
            // Delete character at cursor (not implemented for simplicity)
        }
        
        // Cursor movement
        KeyCode::Left => {
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_position < app.search_input.len() {
                app.cursor_position += 1;
            }
        }
        
        _ => {}
    }
}