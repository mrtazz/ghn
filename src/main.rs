use color_eyre::Result;

pub mod app;
pub mod cache;
pub mod config;
pub mod github;
pub mod notifications;

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| app::App::default().run(terminal))
}
