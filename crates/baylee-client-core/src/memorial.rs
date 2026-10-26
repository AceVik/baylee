//! Baylee's small album. All photographs are local; opening it sends nothing.

/// The number of photographs her owner supplied for the memorial.
pub const PHOTOS: usize = 5;

/// The remembered photograph and whether its sheet is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Album {
    /// Whether the album currently owns input.
    pub open: bool,
    /// Zero-based index into the five local photographs.
    pub photo: usize,
}

impl Album {
    /// Move through the album, wrapping at either end.
    pub fn turn(&mut self, forward: bool) {
        self.photo = (self.photo + if forward { 1 } else { PHOTOS - 1 }) % PHOTOS;
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
}
