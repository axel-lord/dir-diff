use ::std::path::{Path, PathBuf};

use ::clap::Parser;

use crate::ui::PaneId;

/// Compare the contents of two directories.
#[derive(Debug, Parser)]
#[command(version, about, author, long_about = None)]
pub struct Cli {
    /// First directory/exported file.
    left: Option<PathBuf>,

    /// Second directory/exported file.
    right: Option<PathBuf>,
}

impl Cli {
    pub fn panes(&self) -> impl Iterator<Item = (PaneId, &Path)> {
        [
            self.left.as_deref().map(|path| (PaneId::Left, path)),
            self.right.as_deref().map(|path| (PaneId::Right, path)),
        ]
        .into_iter()
        .flatten()
    }
}
