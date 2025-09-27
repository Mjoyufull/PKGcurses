mod ui;
mod core;

use std::env;

#[tokio::main]
async fn main() {
    // Set up panic handler to restore terminal
    std::panic::set_hook(Box::new(|_| {
        use crossterm::{execute, terminal};
        let _ = terminal::disable_raw_mode();
        let _ = execute!(std::io::stdout(), terminal::LeaveAlternateScreen);
    }));
    let args: Vec<String> = env::args().skip(1).collect();
    
    let initial_query = if args.get(0) == Some(&"-S".to_string()) {
        // Single-shot mode with query
        args.get(1).cloned()
    } else {
        // Normal mode with optional initial query
        args.get(0).cloned()
    };

    match ui::run_tui(initial_query).await {
        Ok(()) => {},
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Debug: Error type: {}", std::any::type_name_of_val(&e));
            std::process::exit(1);
        }
    }
}
