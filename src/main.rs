#![windows_subsystem = "windows"]

use ::std::rc::Rc;

use ::clap::Parser as _;
use ::color_eyre::Result;
use ::dir_diff::{cli::Cli, state::State, ui::AppWindow};
use ::slint::ComponentHandle;

fn main() -> Result<()> {
    ::color_eyre::install()?;
    ::env_logger::builder()
        .filter_module("dir_diff", ::log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let cli = Cli::parse();

    let state = Rc::new(State::default());

    let ui = AppWindow::new()?;
    ui.set_app_icon(::dir_diff::icon()?);

    for (id, path) in cli.panes() {
        state.read_path(id, path);
    }

    state.bind(&ui);

    ui.run()?;

    Ok(())
}
