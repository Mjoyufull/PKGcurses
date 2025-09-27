use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::ui::app::{App, ActivePane, InputMode};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();
    
    // Main layout: horizontal split
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(size);
    
    // Left side: search + results + details
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Search input
            Constraint::Min(10),        // Results
            Constraint::Length(8),      // Details
        ])
        .split(chunks[0]);
    
    // Right side: installed packages
    let right_chunk = chunks[1];
    
    // Draw components
    draw_search_input(f, app, left_chunks[0]);
    draw_results(f, app, left_chunks[1]);
    draw_details(f, app, left_chunks[2]);
    draw_installed(f, app, right_chunk);
}

fn draw_search_input(f: &mut Frame, app: &App, area: Rect) {
    let input_style = if app.active_pane == ActivePane::Search {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };
    
    let block = Block::default()
        .title(" Search ")
        .borders(Borders::ALL)
        .border_style(input_style);
    
    let input_text = if app.search_input.is_empty() {
        "Type to search packages...".to_string()
    } else {
        app.search_input.clone()
    };
    
    let paragraph = Paragraph::new(input_text)
        .block(block)
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
    
    // Show cursor if in editing mode
    if app.input_mode == InputMode::Editing {
        let cursor_x = area.x + app.cursor_position as u16 + 1;
        let cursor_y = area.y + 1;
        f.set_cursor(cursor_x, cursor_y);
    }
}

fn draw_results(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_pane == ActivePane::Results {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };
    
    let title = format!(" Results ({}) ", app.filtered_packages.len());
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    
    // Calculate visible range
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = app.scroll_offset;
    let end = (start + visible_height).min(app.filtered_packages.len());
    
    let items: Vec<ListItem> = app.filtered_packages[start..end]
        .iter()
        .enumerate()
        .map(|(i, package)| {
            let actual_index = start + i;
            let is_selected = actual_index == app.selected_index;
            
            // Format: "  name                    ✓ source"
            let installed_indicator = if package.installed { "✓" } else { " " };
            let content = format!("  {:<40} {} {}", 
                package.name, 
                installed_indicator, 
                package.source
            );
            
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if package.installed {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(content).style(style)
        })
        .collect();
    
    let list = List::new(items).block(block);
    
    let mut list_state = ListState::default();
    if !app.filtered_packages.is_empty() && app.selected_index < app.filtered_packages.len() {
        // Convert absolute index to relative index for display
        if app.selected_index >= start && app.selected_index < end {
            list_state.select(Some(app.selected_index - start));
        }
    }
    
    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_details(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_pane == ActivePane::Details {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };
    
    let block = Block::default()
        .title(" Package Details ")
        .borders(Borders::ALL)
        .border_style(border_style);
    
    let content = if let Some(package) = app.get_selected_package() {
        if let Some(details) = app.get_package_details(package) {
            // Show cached details
            details.lines().map(Line::from).collect()
        } else {
            // Show basic info while loading
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Package: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&package.name),
                ]),
                Line::from(vec![
                    Span::styled("Source: ", Style::default().fg(Color::Cyan)),
                    Span::raw(&package.source),
                ]),
            ];
            
            if let Some(version) = &package.version {
                lines.push(Line::from(vec![
                    Span::styled("Version: ", Style::default().fg(Color::Magenta)),
                    Span::raw(version),
                ]));
            }
            
            if let Some(description) = &package.description {
                lines.push(Line::from(""));
                lines.push(Line::from(description.as_str()));
            }
            
            if app.should_fetch_details() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Loading detailed information...",
                    Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)
                )));
            }
            
            lines
        }
    } else {
        vec![Line::from("No package selected")]
    };
    
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

fn draw_installed(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_pane == ActivePane::Installed {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };
    
    let title = format!(" Installed ({}) ", app.installed_packages.len());
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    
    // Calculate visible range
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = app.installed_scroll;
    let end = (start + visible_height).min(app.installed_packages.len());
    
    let items: Vec<ListItem> = app.installed_packages[start..end]
        .iter()
        .enumerate()
        .map(|(i, package)| {
            let actual_index = start + i;
            let is_selected = actual_index == app.installed_selected;
            
            let content = format!("✓ {:<20} {}", package.name, package.source);
            
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Green)
            };
            
            ListItem::new(content).style(style)
        })
        .collect();
    
    let list = List::new(items).block(block);
    
    let mut list_state = ListState::default();
    if !app.installed_packages.is_empty() && app.installed_selected < app.installed_packages.len() {
        if app.installed_selected >= start && app.installed_selected < end {
            list_state.select(Some(app.installed_selected - start));
        }
    }
    
    f.render_stateful_widget(list, area, &mut list_state);
}