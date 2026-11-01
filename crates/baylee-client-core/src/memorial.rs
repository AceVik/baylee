//! Baylee's small album. All photographs are local; opening it sends nothing.

/// The number of photographs her owner supplied for the memorial.
pub const PHOTOS: usize = 10;

/// The family's complete remembrance, divided into eight readable chapters.
pub const CHAPTERS: usize = 8;

/// Localized title and unabridged paragraphs, in the family's approved order.
pub const STORY: [(crate::i18n::Phrase, crate::i18n::Phrase); CHAPTERS] = {
    use crate::i18n::Phrase as P;
    [
        (P::MemorialChapter1Title, P::MemorialChapter1Body),
        (P::MemorialChapter2Title, P::MemorialChapter2Body),
        (P::MemorialChapter3Title, P::MemorialChapter3Body),
        (P::MemorialChapter4Title, P::MemorialChapter4Body),
        (P::MemorialChapter5Title, P::MemorialChapter5Body),
        (P::MemorialChapter6Title, P::MemorialChapter6Body),
        (P::MemorialChapter7Title, P::MemorialChapter7Body),
        (P::MemorialChapter8Title, P::MemorialChapter8Body),
    ]
};

/// The remembered photograph and whether its sheet is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Album {
    /// Whether the album currently owns input.
    pub open: bool,
    /// Zero-based index into the local photographs.
    pub photo: usize,
    /// Last requested photo direction, including wrap-around.
    pub photo_forward: bool,
    /// The chapter being read; photographs can change without losing it.
    pub chapter: usize,
    /// Previous chapter, painted only while its paper leaf turns.
    pub previous_chapter: usize,
    /// A new chapter turn survives UI rebuilds without restarting the animation.
    pub revision: u64,
    /// A photograph, enlarged above the book.
    pub viewing: bool,
    /// Half-size steps above fit, bounded to four times the fitted photograph.
    pub zoom: u8,
}

impl Album {
    /// Move through the album, wrapping at either end.
    pub fn turn(&mut self, forward: bool) {
        self.photo_forward = forward;
        self.photo = (self.photo + if forward { 1 } else { PHOTOS - 1 }) % PHOTOS;
        self.zoom = 0;
    }

    /// Chapters stop at the beginning/end; the farewell never wraps abruptly.
    pub fn read(&mut self, forward: bool) {
        self.select(if forward {
            (self.chapter + 1).min(CHAPTERS - 1)
        } else {
            self.chapter.saturating_sub(1)
        });
    }

    /// Selecting the current chapter never restarts its leaf animation.
    pub fn select(&mut self, chapter: usize) {
        if chapter < CHAPTERS && chapter != self.chapter {
            self.previous_chapter = self.chapter;
            self.chapter = chapter;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Magnification is explicit; hovering a photograph never changes its size.
    pub fn magnify(&mut self, closer: bool) {
        self.zoom = if closer {
            (self.zoom + 1).min(6)
        } else {
            self.zoom.saturating_sub(1)
        };
    }

    /// Escape closes the photograph first, keeping the reader's place.
    pub fn dismiss(&mut self) {
        if self.viewing {
            self.viewing = false;
            self.zoom = 0;
        } else {
            self.open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_wraps_in_both_directions_without_losing_its_place() {
        let mut album = Album {
            open: true,
            photo: 0,
            chapter: 0,
            ..Album::default()
        };
        album.turn(false);
        assert_eq!(album.photo, PHOTOS - 1);
        album.turn(true);
        assert_eq!(album.photo, 0);
        album.turn(true);
        album.open = false;
        album.open = true;
        assert_eq!(album.photo, 1);
    }

    #[test]
    fn chapters_stop_at_the_ends_and_photo_turns_keep_the_story() {
        let mut album = Album::default();
        album.read(false);
        assert_eq!(album.chapter, 0);
        for _ in 0..12 {
            album.read(true);
        }
        assert_eq!(album.chapter, CHAPTERS - 1);
        album.turn(true);
        assert_eq!(album.chapter, CHAPTERS - 1);
        album.read(false);
        assert_eq!(album.chapter, CHAPTERS - 2);
        assert_eq!(album.photo, 1);
    }
}

#[cfg(test)]
mod viewer_tests {
    use super::*;

    #[test]
    fn a_photo_closes_before_the_book_and_never_loses_the_chapter() {
        let mut book = Album {
            open: true,
            viewing: true,
            ..Album::default()
        };
        book.select(5);
        for _ in 0..20 {
            book.magnify(true);
        }
        assert_eq!(book.zoom, 6);
        book.turn(true);
        assert_eq!((book.zoom, book.chapter), (0, 5));
        book.magnify(false);
        assert_eq!(book.zoom, 0);
        book.magnify(true);
        book.dismiss();
        assert!(book.open && !book.viewing);
        assert_eq!((book.zoom, book.chapter, book.photo), (0, 5, 1));
        book.dismiss();
        assert!(!book.open);
    }

    #[test]
    fn only_a_new_valid_chapter_starts_a_leaf_and_reversal_keeps_its_origin() {
        let mut book = Album::default();
        book.select(0);
        book.select(CHAPTERS);
        assert_eq!(book.revision, 0);
        book.select(4);
        assert_eq!(
            (book.previous_chapter, book.chapter, book.revision),
            (0, 4, 1)
        );
        book.turn(true);
        book.magnify(true);
        book.select(4);
        assert_eq!(book.revision, 1);
        book.read(false);
        assert_eq!(
            (book.previous_chapter, book.chapter, book.revision),
            (4, 3, 2)
        );
    }
}
