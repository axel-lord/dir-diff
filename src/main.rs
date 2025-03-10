#![windows_subsystem = "windows"]

use ::std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use ::clap::Parser;
use ::color_eyre::Result;
use ::rfd::AsyncFileDialog;
use ::slint::{
    ComponentHandle, Image, ModelExt, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString,
    VecModel,
};

slint::include_modules!();

const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));

/// Compare the contents of two directories.
#[derive(Debug, Parser)]
#[command(version, about, author, long_about = None)]
struct Cli {
    /// First directory/exported file.
    left: Option<PathBuf>,

    /// Second directory/exported file.
    right: Option<PathBuf>,
}

type HashSetCell = Rc<RefCell<(PathBuf, HashSet<String>)>>;

#[derive(Default)]
struct State {
    right: HashSetCell,
    left: HashSetCell,
}

impl State {
    fn read_dir(path: impl AsRef<Path>) -> (PathBuf, HashSet<String>) {
        let path = path.as_ref();
        ::std::fs::read_dir(path)
            .map(|read_dir| {
                read_dir
                    .filter_map(|entry| {
                        let entry = entry
                            .inspect_err(|err| {
                                ::log::warn!("failed get direntry in '{path:?}', {err}")
                            })
                            .ok()?;
                        let name = entry.file_name().to_string_lossy().into_owned();
                        Some(name)
                    })
                    .collect()
            })
            .or_else(|_| {
                let buf = ::std::fs::read(path)?;
                Ok(::serde_json::from_slice(&buf).unwrap_or_else(|err| {
                    ::log::error!("could not parse {path:?} as json, {err}");
                    Default::default()
                }))
            })
            .map(|set| (path.to_path_buf(), set))
            .unwrap_or_else(|err: ::std::io::Error| {
                ::log::error!("failed to read directory/file '{path:?}, {err}'");
                Default::default()
            })
    }

    fn lines(set: &HashSet<String>) -> ModelRc<Line> {
        ModelRc::new(
            set.iter()
                .map(|text| Line {
                    striked: false,
                    text: SharedString::from(text),
                })
                .collect::<VecModel<_>>()
                .sort_by(|a, b| a.text.cmp(&b.text)),
        )
    }

    fn diff(a: &HashSet<String>, b: &HashSet<String>) -> ModelRc<Line> {
        ModelRc::new(
            a.difference(b)
                .map(|text| Line {
                    striked: false,
                    text: SharedString::from(text),
                })
                .collect::<VecModel<_>>()
                .sort_by(|a, b| a.text.cmp(&b.text)),
        )
    }

    fn get_set(&self, id: PaneId) -> &HashSetCell {
        match id {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    fn complement_id(id: PaneId) -> PaneId {
        match id {
            PaneId::Left => PaneId::Right,
            PaneId::Right => PaneId::Left,
        }
    }

    fn update(&self, ui: &AppWindow) {
        for id in [PaneId::Left, PaneId::Right] {
            let set = self.get_set(id).borrow();

            ui.invoke_set_lines(id, Self::lines(&set.1));
            ui.invoke_set_diff(
                id,
                Self::diff(&set.1, &self.get_set(Self::complement_id(id)).borrow().1),
            );
            ui.invoke_set_title(id, set.0.to_string_lossy().into_owned().into());
        }
    }

    fn bind(self: Rc<Self>, ui: &AppWindow) {
        ui.on_open({
            let ui = ui.as_weak();
            let state = self.clone();
            move |id| {
                let ui = ui.clone();
                let state = state.clone();
                ::slint::spawn_local(async move {
                    let handle = AsyncFileDialog::new()
                        .set_title("Open folder...")
                        .pick_folder()
                        .await?;

                    state.get_set(id).replace(Self::read_dir(handle.path()));
                    state.update(&ui.unwrap());

                    Some(())
                })
                .unwrap();
            }
        });
        ui.on_import({
            let ui = ui.as_weak();
            let state = self.clone();
            move |id| {
                let ui = ui.clone();
                let state = state.clone();
                ::slint::spawn_local(async move {
                    let handle = AsyncFileDialog::new()
                        .set_title("Import list...")
                        .add_filter("JSON", &["json"])
                        .pick_file()
                        .await?;

                    state.get_set(id).replace(Self::read_dir(handle.path()));
                    state.update(&ui.unwrap());

                    Some(())
                })
                .unwrap();
            }
        });
        ui.on_export({
            let state = self.clone();
            move |id| {
                let state = state.clone();
                ::slint::spawn_local(async move {
                    let handle = AsyncFileDialog::new()
                        .set_title("Export list...")
                        .add_filter("JSON", &["json"])
                        .save_file()
                        .await?;

                    let path = handle.path();
                    ::std::fs::write(
                        path,
                        ::serde_json::to_string_pretty::<HashSet<String>>(
                            &state.get_set(id).borrow().1,
                        )
                        .unwrap(),
                    )
                    .unwrap_or_else(|err| ::log::error!("write to {path:?} failed, {err}"));

                    Some(())
                })
                .unwrap();
            }
        });

        ui.on_reload({
            let ui = ui.as_weak();
            let state = self.clone();
            move |id| {
                let set = state.get_set(id);
                let content = Self::read_dir(&set.borrow().0);

                set.replace(content);
                state.update(&ui.unwrap());
            }
        });
    }
}

fn main() -> Result<()> {
    ::color_eyre::install()?;
    ::env_logger::builder()
        .filter_module("dir_diff", ::log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let Cli { left, right } = Cli::parse();

    let state = Rc::new(State::default());
    let icon = ::image::load_from_memory_with_format(ICON, ::image::ImageFormat::Png)?.into_rgba8();
    let icon_buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        icon.as_raw(),
        icon.width(),
        icon.height(),
    );
    let icon = Image::from_rgba8(icon_buf);

    let ui = AppWindow::new()?;
    ui.set_app_icon(icon);

    if let Some(path) = left {
        state.left.replace(State::read_dir(&path));
    }

    if let Some(path) = right {
        state.right.replace(State::read_dir(&path));
    }

    state.update(&ui);
    state.bind(&ui);

    ui.run()?;

    Ok(())
}
