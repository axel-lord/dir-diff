use ::std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use ::clap::Parser;
use ::color_eyre::Result;
use ::rfd::AsyncFileDialog;
use ::slint::{ComponentHandle, ModelRc, SharedString, VecModel, Weak};

slint::include_modules!();

/// Compare the contents of two directories.
#[derive(Debug, Parser)]
#[command(version, about, author, long_about = None)]
struct Cli {
    /// First directory/exported file.
    #[arg(short, long)]
    left: Option<PathBuf>,

    /// Second directory/exported file.
    #[arg(short, long)]
    right: Option<PathBuf>,
}

type HashSetCell = Rc<RefCell<HashSet<String>>>;

#[derive(Default)]
struct State {
    right: HashSetCell,
    left: HashSetCell,
}

impl State {
    fn read_dir(path: impl AsRef<Path>) -> HashSet<String> {
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
            .unwrap_or_else(|err: ::std::io::Error| {
                ::log::error!("failed to read directory/file '{path:?}, {err}'");
                Default::default()
            })
    }

    fn lines(set: &HashSet<String>) -> ModelRc<SharedString> {
        ModelRc::new(set.iter().map(SharedString::from).collect::<VecModel<_>>())
    }

    fn diff(a: &HashSet<String>, b: &HashSet<String>) -> ModelRc<SharedString> {
        ModelRc::new(
            a.difference(b)
                .map(SharedString::from)
                .collect::<VecModel<_>>(),
        )
    }

    fn diff_pair(&self) -> [ModelRc<SharedString>; 2] {
        [
            Self::diff(&self.left.borrow(), &self.right.borrow()),
            Self::diff(&self.right.borrow(), &self.left.borrow()),
        ]
    }

    fn lines_pair(&self) -> [ModelRc<SharedString>; 2] {
        [
            Self::lines(&self.left.borrow()),
            Self::lines(&self.right.borrow()),
        ]
    }

    fn update(&self, ui: &AppWindow) {
        let [l_diff, r_diff] = self.diff_pair();
        let [l_lines, r_lines] = self.lines_pair();

        ui.set_l_diff(l_diff);
        ui.set_r_diff(r_diff);

        ui.set_l_lines(l_lines);
        ui.set_r_lines(r_lines);
    }

    fn bind(self: Rc<Self>, ui: &AppWindow) {
        ui.on_l_open(State::open(
            ui.as_weak(),
            self.clone(),
            self.left.clone(),
            State::set_l_title(ui.as_weak()),
        ));
        ui.on_r_open(State::open(
            ui.as_weak(),
            self.clone(),
            self.right.clone(),
            State::set_r_title(ui.as_weak()),
        ));

        ui.on_l_import(State::import(
            ui.as_weak(),
            self.clone(),
            self.left.clone(),
            State::set_l_title(ui.as_weak()),
        ));
        ui.on_r_import(State::import(
            ui.as_weak(),
            self.clone(),
            self.right.clone(),
            State::set_r_title(ui.as_weak()),
        ));

        ui.on_l_export(State::export(self.left.clone()));
        ui.on_r_export(State::export(self.right.clone()));
    }

    fn open(
        ui: Weak<AppWindow>,
        state: Rc<Self>,
        set: HashSetCell,
        set_title: impl 'static + Clone + Fn(String),
    ) -> impl Fn() {
        move || {
            let set = set.clone();
            let state = state.clone();
            let ui = ui.clone();
            let set_title = set_title.clone();
            ::slint::spawn_local(async move {
                let handle = AsyncFileDialog::new()
                    .set_title("Open folder...")
                    .pick_folder()
                    .await?;

                set.replace(Self::read_dir(handle.path()));
                state.update(&ui.unwrap());
                set_title(handle.path().to_string_lossy().into_owned());
                Some(())
            })
            .unwrap();
        }
    }

    fn import(
        ui: Weak<AppWindow>,
        state: Rc<Self>,
        set: HashSetCell,
        set_title: impl 'static + Clone + Fn(String),
    ) -> impl Fn() {
        move || {
            let set = set.clone();
            let state = state.clone();
            let ui = ui.clone();
            let set_title = set_title.clone();
            ::slint::spawn_local(async move {
                let handle = AsyncFileDialog::new()
                    .set_title("Import list...")
                    .add_filter("JSON", &["json"])
                    .pick_file()
                    .await?;

                set.replace(Self::read_dir(handle.path()));
                state.update(&ui.unwrap());
                set_title(handle.path().to_string_lossy().into_owned());
                Some(())
            })
            .unwrap();
        }
    }

    fn export(set: HashSetCell) -> impl Fn() {
        move || {
            let set = set.clone();
            ::slint::spawn_local(async move {
                let handle = AsyncFileDialog::new()
                    .set_title("Export list...")
                    .add_filter("JSON", &["json"])
                    .save_file()
                    .await?;

                let path = handle.path();
                ::std::fs::write(
                    path,
                    ::serde_json::to_string_pretty::<HashSet<String>>(&set.borrow()).unwrap(),
                )
                .unwrap_or_else(|err| ::log::error!("write to {path:?} failed, {err}"));

                Some(())
            })
            .unwrap();
        }
    }

    fn set_l_title(ui: Weak<AppWindow>) -> impl 'static + Clone + Fn(String) {
        move |title| {
            ui.unwrap().set_l_title(title.into());
        }
    }

    fn set_r_title(ui: Weak<AppWindow>) -> impl 'static + Clone + Fn(String) {
        move |title| {
            ui.unwrap().set_r_title(title.into());
        }
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

    let ui = AppWindow::new()?;

    if let Some(path) = left {
        state.left.replace(State::read_dir(&path));
        ui.set_l_title(SharedString::from(path.to_string_lossy().into_owned()));
    }

    if let Some(path) = right {
        state.right.replace(State::read_dir(&path));
        ui.set_r_title(SharedString::from(path.to_string_lossy().into_owned()));
    }

    state.update(&ui);
    state.bind(&ui);

    ui.run()?;

    Ok(())
}
