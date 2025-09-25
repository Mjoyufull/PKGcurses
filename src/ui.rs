pub mod tui;
pub mod components;

use tui::App;

pub fn run_tui(initial_query: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(initial_query)?;
    app.run()
}
