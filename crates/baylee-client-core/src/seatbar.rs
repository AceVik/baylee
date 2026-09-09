//! What a seat's bar is made of, and how much of it fits on the shelf.
//!
//! The bar is the strip of ink written on each seat's ledge
//! ([`crate::tabletop::MAT_LEDGE`]): who the seat is, what they have, and the
//! twelve steps of a turn. It replaces a phase rail pinned across the top of
//! the window and a strip of seat tabs above that — two panels that were
//! about seats and turns, drawn nowhere near the seats or the turns.
//!
//! This module is the part with no renderer in it. It says which cells a bar
//! carries, how wide each one is drawn, and — the only real decision — which
//! **density** a shelf of a given projected length can hold. The renderer
//! projects a seat's ledge, asks here, and builds nodes; a test asks here
//! with a number and needs no window.
//!
//! Two rules shape every width below.
//!
//! **A numeral cell is fixed-width.** Life going from 9 to 10 must move
//! nothing else on the bar, or every count on the shelf twitches sideways
//! whenever anyone draws a card. So a cell is sized for the widest number it
//! will ever hold and the numeral is centred in it.
//!
//! **The densest form that fits wins**, and each form is allowed its own
//! overhang. The full bar is allowed none — its ground is the shelf, and ink
//! hanging off the end of the shelf is ink on the felt. The compact form is
//! allowed a quarter, evenly split, because by the time a shelf is that short
//! the alternative is dropping the counts entirely.

/// How much of a seat's bar is drawn, chosen by how long its shelf is on
/// screen.
///
/// Not a preference and not a window size: the *ledge's* projected length.
/// A seat across a four-player table and the seat next to it have the same
/// bar to draw and very different shelves to draw it on, and the seat on your
/// left is short because it is edge-on to the camera rather than because
/// anything about it is smaller.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Density {
    /// Every cell, and the step tiles carry their labels.
    Full,
    /// Every cell, and the step tiles are their glyphs alone.
    Compact,
    /// Name, life and the hand; the steps are pips. The seat sheet carries
    /// what is dropped.
    Pip,
}

/// One cell of a seat's bar, left to right in the **viewer's** reading order.
///
/// The ground belongs to the seat and the ink reads in the viewer's order:
/// the shelf of the seat across the table is rotated half a turn, and its bar
/// is still written left to right for the person looking at it. A bar that
/// turned over with its mat would be a bar nobody at the table can read but
/// the one player who cannot see it anyway.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    /// The priority caret. Its own column, always present, and invisible
    /// rather than absent when the seat is not holding priority — a caret
    /// that took its width with it would shift the whole bar every time
    /// priority passed.
    Caret,
    /// The seat's accent, three pixels of it: gilt for the viewing seat, the
    /// pie in ring order for everyone else. The same colour the seat's rim
    /// carries, so the bar and the ground under it name the same player.
    Swatch,
    /// The seat's display name, ellipsised.
    Name,
    /// Life, with a heart.
    Life,
    /// One of the four counts.
    Count(Zone),
    /// The turn number on the active seat's bar, a brass tick on every other.
    ///
    /// It is called a hinge because it is what keeps the two halves of the
    /// bar from reading as two things: the number belongs to the seat whose
    /// bar it is *and* to the twelve steps beside it.
    Hinge,
    /// The twelve steps of a turn.
    Steps,
}

/// The four counts a bar carries, in the order they are drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone {
    /// Cards in hand — a count for every seat, including one's own, because
    /// the bar says the same thing about everybody.
    Hand,
    /// Cards left in the library.
    Library,
    /// Cards in the graveyard.
    Graveyard,
    /// Cards in exile.
    Exile,
}

impl Zone {
    /// All four, in bar order.
    pub const ALL: [Self; 4] = [Self::Hand, Self::Library, Self::Graveyard, Self::Exile];

    /// How wide this count's cell is drawn.
    ///
    /// The library is the wide one: it is the only count that starts in three
    /// figures, and a hundred-card commander deck would otherwise push its
    /// own numeral into the graveyard's cell on turn one.
    #[must_use]
    pub const fn width(self) -> f32 {
        match self {
            Self::Library => 40.0,
            _ => 36.0,
        }
    }
}

/// The bar's height on screen, the same for every seat.
///
/// The ledge projects deeper than this at a duel's framing, which is what
/// leaves a few pixels of shelf above and below the ink and makes the bar sit
/// *on* the felt rather than float over it. `camera_tests` is what holds that
/// and not this constant.
pub const BAR_H: f32 = 34.0;

/// The gap between two cells.
pub const CELL_GAP: f32 = 6.0;

/// The gap between two step tiles, per density.
const TILE_GAP: [f32; 3] = [3.0, 3.0, 4.0];

/// How wide one step tile is drawn, per density.
const TILE_W: [f32; 3] = [40.0, 30.0, 14.0];

/// How tall a step tile is. One number: a tile is the same object at every
/// density and only its width and what is written in it change.
pub const TILE_H: f32 = 24.0;

/// How many steps a turn has, and therefore how many tiles a bar carries.
///
/// Stated here rather than counted from `automation::RAIL_ROWS`, because this
/// module is arithmetic about a strip of screen and the twelve steps are the
/// shape it has to hold. The test below is what ties the two together.
pub const STEPS: usize = 12;

/// The caret's column, the swatch, and the life cell.
const CARET_W: f32 = 10.0;
const SWATCH_W: f32 = 3.0;
const LIFE_W: f32 = 48.0;

/// The hinge, and how much wider it is drawn once the game has a day/night
/// designation to put beside the turn number (CR 731).
///
/// It widens exactly once and never narrows: a designation never goes back to
/// neither (CR 731.1), so this is not a reserved slot but a cell that grows
/// on the turn the first werewolf resolves.
const HINGE_W: f32 = 36.0;
const HINGE_W_DESIGNATED: f32 = 52.0;

impl Density {
    /// Densest first, which is the order [`Self::for_length`] tries them in.
    pub const ALL: [Self; 3] = [Self::Full, Self::Compact, Self::Pip];

    /// This density's index into the per-density tables above.
    const fn rank(self) -> usize {
        match self {
            Self::Full => 0,
            Self::Compact => 1,
            Self::Pip => 2,
        }
    }

    /// The densest form that fits on a shelf `length` pixels long.
    ///
    /// [`Self::Pip`] is the floor rather than a fourth answer of "nothing":
    /// a bar too small to read is still where the seat's caret and its life
    /// are, and the seat sheet carries everything the pip form drops.
    #[must_use]
    pub fn for_length(length: f32, designated: bool) -> Self {
        for density in [Self::Full, Self::Compact] {
            if length >= density.min_length(designated) {
                return density;
            }
        }
        Self::Pip
    }

    /// The shortest shelf this density may be drawn on.
    ///
    /// Its own width, less whatever overhang the form is allowed.
    #[must_use]
    pub fn min_length(self, designated: bool) -> f32 {
        self.width(designated) * (1.0 - self.overhang())
    }

    /// How much of its own width this form may hang off the ends of the
    /// shelf, evenly split.
    ///
    /// Zero for the full bar: its ground *is* the shelf, and a label hanging
    /// over the edge is a label on the felt. A quarter for the compact form,
    /// because at that length the alternative is dropping the counts, and
    /// counts overhanging a shelf are still counts.
    #[must_use]
    pub const fn overhang(self) -> f32 {
        match self {
            Self::Full => 0.0,
            Self::Compact | Self::Pip => 0.25,
        }
    }

    /// The cells this density draws, in the viewer's reading order.
    #[must_use]
    pub fn cells(self) -> Vec<Cell> {
        let mut out = vec![Cell::Caret, Cell::Swatch, Cell::Name, Cell::Life];
        match self {
            Self::Full | Self::Compact => {
                out.extend(Zone::ALL.map(Cell::Count));
                out.push(Cell::Hinge);
            }
            // The hand is the one count a player reads about an opponent
            // without being told to, so it is the one that survives. The
            // hinge goes: a turn number with no tiles beside it is a fact
            // with nothing to hinge.
            Self::Pip => out.push(Cell::Count(Zone::Hand)),
        }
        out.push(Cell::Steps);
        out
    }

    /// How wide one cell is drawn at this density.
    #[must_use]
    pub fn cell_width(self, cell: Cell, designated: bool) -> f32 {
        match cell {
            Cell::Caret => CARET_W,
            Cell::Swatch => SWATCH_W,
            Cell::Name => self.name_width(),
            Cell::Life => LIFE_W,
            Cell::Count(zone) => zone.width(),
            Cell::Hinge => {
                if designated {
                    HINGE_W_DESIGNATED
                } else {
                    HINGE_W
                }
            }
            Cell::Steps => self.steps_width(),
        }
    }

    /// The most a name may take before it is ellipsised.
    #[must_use]
    pub const fn name_width(self) -> f32 {
        match self {
            Self::Full => 112.0,
            Self::Compact | Self::Pip => 96.0,
        }
    }

    /// How wide one step tile is drawn, and the gap between two of them.
    #[must_use]
    pub const fn tile_width(self) -> f32 {
        TILE_W[self.rank()]
    }

    /// The gap between two step tiles at this density.
    #[must_use]
    pub const fn tile_gap(self) -> f32 {
        TILE_GAP[self.rank()]
    }

    /// Whether a step tile carries its three-letter label as well as its
    /// glyph. Only the full bar has room; the compact bar's tiles say what
    /// they are through the step slip on hover.
    #[must_use]
    pub const fn tiles_are_labelled(self) -> bool {
        matches!(self, Self::Full)
    }

    /// The twelve tiles and the eleven gaps between them.
    #[must_use]
    pub fn steps_width(self) -> f32 {
        let tiles = self.tile_width() * STEPS as f32;
        tiles + self.tile_gap() * (STEPS - 1) as f32
    }

    /// How wide the whole bar is drawn at this density.
    #[must_use]
    pub fn width(self, designated: bool) -> f32 {
        let cells = self.cells();
        let gaps = CELL_GAP * (cells.len() - 1) as f32;
        cells
            .iter()
            .map(|cell| self.cell_width(*cell, designated))
            .sum::<f32>()
            + gaps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three forms are three sizes, and in the order their names claim.
    ///
    /// Worth a test because the widths are eleven separate numbers and
    /// nothing in the type says they descend: a compact bar wider than a full
    /// one would make `for_length` pick the wrong form at every length and
    /// would look, on screen, like the density logic simply not working.
    #[test]
    fn a_denser_bar_is_a_wider_bar() {
        for designated in [false, true] {
            let widths: Vec<f32> = Density::ALL.map(|d| d.width(designated)).into();
            for pair in widths.windows(2) {
                assert!(
                    pair[0] > pair[1],
                    "the densities are {widths:?} — not descending"
                );
            }
        }
    }

    /// And the same for the shelf each one asks for, which is *not* implied
    /// by the widths: the compact form is allowed an overhang the full form
    /// is not, so a small enough overhang allowance would let the compact
    /// form demand a longer shelf than the full bar does.
    #[test]
    fn a_denser_bar_asks_for_a_longer_shelf() {
        for designated in [false, true] {
            let mins: Vec<f32> = Density::ALL.map(|d| d.min_length(designated)).into();
            for pair in mins.windows(2) {
                assert!(
                    pair[0] > pair[1],
                    "the shelves asked for are {mins:?} — not descending"
                );
            }
        }
    }

    /// The densest form that fits, at every boundary and on both sides of it.
    #[test]
    fn the_densest_form_that_fits_is_the_one_taken() {
        for designated in [false, true] {
            for density in Density::ALL {
                let min = density.min_length(designated);
                assert_eq!(
                    Density::for_length(min, designated),
                    density,
                    "a shelf of exactly {min} should take {density:?}"
                );
                assert_eq!(
                    Density::for_length(min + 1.0, designated),
                    density,
                    "a shelf a pixel over {min} should still take {density:?}"
                );
            }
            // Just under each threshold is the next form down, and below the
            // last one it is pips all the way to nothing — a shelf too short
            // for a bar still has a seat on it.
            let full = Density::Full.min_length(designated);
            assert_eq!(
                Density::for_length(full - 1.0, designated),
                Density::Compact
            );
            let compact = Density::Compact.min_length(designated);
            assert_eq!(Density::for_length(compact - 1.0, designated), Density::Pip);
            assert_eq!(Density::for_length(0.0, designated), Density::Pip);
            assert_eq!(Density::for_length(-40.0, designated), Density::Pip);
        }
    }

    /// A thinner bar never carries a cell a denser one drops.
    ///
    /// The three forms are one bar at three sizes, not three designs. If the
    /// pip form kept the graveyard while the compact form dropped it, a
    /// player would learn the bar twice and the seat sheet — which exists to
    /// carry exactly what a density drops — would have to know which.
    #[test]
    fn a_thinner_bar_is_a_subset_of_a_denser_one() {
        for pair in Density::ALL.windows(2) {
            let denser = pair[0].cells();
            for cell in pair[1].cells() {
                assert!(
                    denser.contains(&cell),
                    "{:?} draws {cell:?} and {:?} does not",
                    pair[1],
                    pair[0]
                );
            }
        }
    }

    /// Every form keeps the four things that answer "who is this and are they
    /// alive", and every form draws the steps.
    #[test]
    fn no_form_drops_the_seat_or_the_turn() {
        for density in Density::ALL {
            let cells = density.cells();
            for wanted in [
                Cell::Caret,
                Cell::Swatch,
                Cell::Name,
                Cell::Life,
                Cell::Steps,
            ] {
                assert!(
                    cells.contains(&wanted),
                    "{density:?} drops {wanted:?}, which is not optional"
                );
            }
            // And the caret is first, because it is the one cell whose
            // meaning is positional: it points at the bar it belongs to.
            assert_eq!(cells[0], Cell::Caret, "{density:?} starts elsewhere");
            assert_eq!(
                cells.last(),
                Some(&Cell::Steps),
                "{density:?} does not end in the turn"
            );
        }
    }

    /// The designation widens the bar and nothing else about it.
    ///
    /// A werewolf resolving must not be able to change which density a shelf
    /// can hold *downwards past a boundary* without the bar visibly
    /// rearranging — this is the test that says the widening is sixteen
    /// pixels on one cell rather than a relayout.
    #[test]
    fn a_designation_widens_one_cell() {
        for density in Density::ALL {
            let grew = density.width(true) - density.width(false);
            let expected = if density.cells().contains(&Cell::Hinge) {
                HINGE_W_DESIGNATED - HINGE_W
            } else {
                0.0
            };
            assert!(
                (grew - expected).abs() < 1e-3,
                "{density:?} grows {grew} when the game gains a designation, \
                 not {expected}"
            );
        }
    }

    /// A duel's own shelf takes the form the spec says it does.
    ///
    /// The local ledge projects to about 0.635 of the window's width at a
    /// duel framing — measured, and re-measured by `camera_tests` against the
    /// real projection. This is the reading of that number: a laptop sees the
    /// compact bar and only a wide window sees labels on the tiles.
    #[test]
    fn a_laptop_duel_sees_the_compact_bar() {
        let shelf = |window: f32| window * 0.635;
        for (window, wanted) in [
            (1280.0, Density::Compact),
            (1440.0, Density::Compact),
            (1728.0, Density::Full),
            (1920.0, Density::Full),
        ] {
            assert_eq!(
                Density::for_length(shelf(window), false),
                wanted,
                "a {window}-wide duel should draw the {wanted:?} bar"
            );
        }
    }

    /// The bar holds exactly the steps a turn has.
    ///
    /// [`STEPS`] is written here because this module is arithmetic about a
    /// strip of screen, and `automation::RailRow` is the turn itself. They
    /// have to agree, and nothing but this notices if a step is ever added:
    /// the bar would draw eleven tiles and the twelfth would silently not be
    /// there.
    #[test]
    fn a_bar_carries_one_tile_per_step_of_a_turn() {
        assert_eq!(
            STEPS,
            crate::automation::RAIL_ROWS.len(),
            "the bar draws {STEPS} tiles for a turn of {} steps",
            crate::automation::RAIL_ROWS.len()
        );
    }

    /// A numeral cell is wide enough for the number it will hold.
    ///
    /// Not a measurement of a font — that is the renderer's problem — but the
    /// ordering the widths encode: the library is the only count that starts
    /// in three figures, so it is the only one drawn wider, and life is wider
    /// than any of them because it carries a heart as well.
    #[test]
    fn the_library_is_the_wide_count_and_life_is_wider_still() {
        for zone in Zone::ALL {
            if zone == Zone::Library {
                continue;
            }
            assert!(
                Zone::Library.width() > zone.width(),
                "the library is not drawn wider than the {zone:?}"
            );
            assert!(
                LIFE_W > zone.width(),
                "life is not drawn wider than a count"
            );
        }
    }
}
