mod app;
mod gitlab;
mod types;

use app::App;
use std::error::Error;

use crate::gitlab::is_glab_installed;

fn main() -> Result<(), Box<dyn Error>> {
    is_glab_installed();
    // TODO: add is_glab_logged_in checker
    ratatui::run(|terminal| App::default().run(terminal))?;

    Ok(())
}
