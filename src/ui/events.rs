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
        
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
        }
        
        // Multi-selection with space
        KeyCode::Char(' ') => {
            if app.active_pane == ActivePane::Results {
                app.toggle_package_selection();
            }
        }
        
        // Pane switching
        KeyCode::Tab => {
            app.switch_pane();
        }
        
        // Enter search mode
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.enter_search_mode();
        }
        
        // Page navigation
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.move_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.move_down();
            }
        }
        
        // Home/End
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
        
        // Start typing to search
        KeyCode::Char(c) if c.is_alphanumeric() || c == '-' || c == '_' => {
            app.enter_search_mode();
            app.add_char(c);
        }
        
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
        
        // Multi-selection with space in search mode
        KeyCode::Char(' ') => {
            // If we have results, toggle selection of current item
            if !app.filtered_packages.is_empty() {
                app.toggle_package_selection();
            } else {
                // Otherwise add space to search
                app.add_char(' ');
            }
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