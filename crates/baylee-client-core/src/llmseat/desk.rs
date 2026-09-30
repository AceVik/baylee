//! The settings panel on disk, on native targets: the file and the spend
//! book beside it, read when the panel opens, read again when either
//! changes (a bridge's game wrote the book, the player edited the file by
//! hand), and the file written through [`store::save`] alone.
//!
//! Whether a file changed is told by its length and modification time,
//! which the caller asks for as often as it likes: a quiet poll reads no
//! file. Nothing here reads a clock; the caller says what time it is.

use super::ledger::{Book, Moment};
use super::panel::{Act, Disk, SeatPanel};
use super::store;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file as the file system describes it: its length and when it was
/// last written, or `None` for no file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp(Option<(u64, Option<SystemTime>)>);

impl Stamp {
    fn of(path: &Path) -> Self {
        Self(
            std::fs::metadata(path)
                .ok()
                .map(|meta| (meta.len(), meta.modified().ok())),
        )
    }
}

/// The panel, the settings file it edits, and the spend book it shows.
#[derive(Debug)]
pub struct Desk {
    path: PathBuf,
    book: Book,
    panel: SeatPanel,
    file: Stamp,
    spend: Stamp,
}

impl Desk {
    /// The panel over the settings file at `path` and the book beside it,
    /// as they are at `now`.
    #[must_use]
    pub fn open(path: PathBuf, now: Moment) -> Self {
        let book = Book::beside(&path);
        let file = Stamp::of(&path);
        let spend = Stamp::of(book.path());
        let mut panel = SeatPanel::new(Disk::of(store::load(&path)));
        panel.read_book(book.read(), now);
        Self {
            path,
            book,
            panel,
            file,
            spend,
        }
    }

    /// Reads again whichever file changed since it was last read, and
    /// moves the spend to `now`'s day. Whether anything the panel shows
    /// changed.
    pub fn poll(&mut self, now: Moment) -> bool {
        let mut changed = false;
        let file = Stamp::of(&self.path);
        if file != self.file {
            self.file = file;
            changed |= self.panel.found(Disk::of(store::load(&self.path)));
        }
        let spend = Stamp::of(self.book.path());
        if spend == self.spend {
            changed |= self.panel.tick(now);
        } else {
            self.spend = spend;
            self.panel.read_book(self.book.read(), now);
            changed = true;
        }
        changed
    }

    /// Does what a press asks: [`Act::Save`] writes the file, and every
    /// other act is the panel's.
    pub fn act(&mut self, act: Act) {
        if act == Act::Save {
            self.save();
        } else {
            self.panel.act(act);
        }
    }

    /// Writes the panel's settings, when it offers any
    /// ([`SeatPanel::to_save`]), through [`store::save`], which reads them
    /// back before a byte is written.
    pub fn save(&mut self) {
        let Some(settings) = self.panel.to_save() else {
            return;
        };
        match store::save(&self.path, &settings) {
            Ok(()) => {
                self.file = Stamp::of(&self.path);
                self.panel.saved(settings);
            }
            Err(why) => self.panel.not_saved(why),
        }
    }

    /// The panel.
    #[must_use]
    pub fn panel(&self) -> &SeatPanel {
        &self.panel
    }

    /// The panel, to type into.
    pub fn panel_mut(&mut self) -> &mut SeatPanel {
        &mut self.panel
    }

    /// Where the settings file is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}
