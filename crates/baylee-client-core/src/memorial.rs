//! Baylee's small album. All photographs are local; opening it sends nothing.

/// The number of photographs her owner supplied for the memorial.
pub const PHOTOS: usize = 5;

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
    /// Zero-based index into the five local photographs.
    pub photo: usize,
    /// The chapter being read; photographs can change without losing it.
    pub chapter: usize,
}

impl Album {
    /// Move through the album, wrapping at either end.
    pub fn turn(&mut self, forward: bool) {
        self.photo = (self.photo + if forward { 1 } else { PHOTOS - 1 }) % PHOTOS;
    }

    /// Chapters stop at the beginning/end; the farewell never wraps abruptly.
    pub fn read(&mut self, forward: bool) {
        self.chapter = if forward {
            (self.chapter + 1).min(CHAPTERS - 1)
        } else {
            self.chapter.saturating_sub(1)
        };
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
        };
        album.turn(false);
        assert_eq!(album.photo, 4);
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
