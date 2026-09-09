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
//!
//! **A shelf has two measurements, and the second one only started being
//! asked here with [`Density::Split`].** The four single-row forms are a
//! ladder in *length*: they say the same things at four sizes, and a shelf
//! deep enough for the tallest of them is deep enough for all four. The
//! two-row form is the one that trades the other way — it wants half the
//! length and twice the depth — so [`Density::for_shelf`] is what the
//! renderer asks, and [`Density::for_length`] is the ladder underneath it.

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
    /// Two rows: the twelve steps alone along the shelf's outer edge, and the
    /// seat's identity — caret, colour, name, life, the four counts — on its
    /// own row beneath them.
    ///
    /// The steps get the **whole** length of the shelf, which is the point of
    /// the form and the reason it is worth a second row. Sharing one row with
    /// the identity cells leaves the twelve tiles about four hundred pixels
    /// even on a wide duel, and a tile at the end of that division is a
    /// glyph with no room for its label; alone on the row they grow to
    /// [`Density::tile_width_max`] and the phase line reads as a line.
    ///
    /// It is not a rung of the length ladder. It asks for a **shorter** shelf
    /// than [`Self::Full`] (the two rows are as long as the longer of them,
    /// not as long as their sum) and a **deeper** one than any single-row
    /// form, so the two are chosen together in [`Self::for_shelf`].
    Split,
    /// Every cell, and the step tiles carry their labels.
    Full,
    /// Every cell, and the step tiles are their glyphs alone.
    Compact,
    /// Name, life and the hand; the steps are small blank pips.
    Pip,
    /// The caret, the seat's colour, and the twelve steps. Nothing else.
    ///
    /// This is what a seat gets on a six- to eight-player ring, where its
    /// whole board is under 230 logical pixels wide and its cards are drawn
    /// eighteen pixels across. There is no bar that fits there and says more,
    /// and a bar that overhung its own mat to say more would be writing on
    /// the seats beside it. What survives is the promise §7 of the design
    /// makes — **every bar marks the step the game is in** — plus the colour
    /// that says whose bar it is, which is the same colour that seat's rim
    /// wears. The seat sheet carries the rest, and framing the seat (a click
    /// on its board) widens it back to a bar that can talk.
    Mark,
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

/// The gap between two cells.
pub const CELL_GAP: f32 = 6.0;

/// The gap between two step tiles, per density.
const TILE_GAP: [f32; 5] = [3.0, 3.0, 3.0, 3.0, 2.0];

/// How wide and how tall one step tile is drawn, per density.
///
/// A pip is a *smaller tile*, not the same tile with less written in it, and
/// that is what makes the pip form reachable at all. A five-seat ring is the
/// table the camera has least room for — its ledge projects under thirty
/// pixels deep at the reference window — so a bar whose tiles were 24 tall
/// everywhere could not be drawn on it without standing on the creature lane.
const TILE_W: [f32; 5] = [36.0, 40.0, 30.0, 11.0, 8.0];
const TILE_H: [f32; 5] = [16.0, 24.0, 24.0, 12.0, 10.0];

/// The widest a [`Density::Split`] tile is drawn.
///
/// The split form's tiles are the only ones that **grow**: they have the
/// shelf to themselves, so the renderer lets them take the slack rather than
/// leaving four hundred pixels of ink centred on a twelve-hundred-pixel
/// ledge. The cap is four times the tile's own height, past which a tile
/// stops reading as a tile and starts reading as a ribbon — and the slack
/// left over goes into the gaps, so the row still spans the whole shelf.
const SPLIT_TILE_W_MAX: f32 = 64.0;

/// The gap between the two rows of a [`Density::Split`] bar.
///
/// Two pixels, and deliberately less than [`CELL_GAP`]: the rows are one bar
/// about one seat, and a gap wide enough to read as a division would make the
/// phase line look like it belonged to the table rather than to the mat it is
/// written on.
const SPLIT_ROW_GAP: f32 = 0.0;

/// How tall the identity row of a [`Density::Split`] bar is drawn.
///
/// It carries no tiles — only text and a three-pixel swatch — so it is sized
/// for the tallest numeral on it rather than for a control, which is what
/// makes the two rows fit on a shelf a single full-size row nearly fills.
const SPLIT_IDENTITY_H: f32 = 14.0;

/// How far outside a tile the now-ring stands, and how thick it is.
///
/// It is the outermost ink on a bar, so it is what the shelf has to be deep
/// enough to hold — see [`Density::ink_height`].
pub const HALO_OUT: f32 = 2.0;

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
    /// Every form there is, densest first.
    pub const ALL: [Self; 5] = [
        Self::Split,
        Self::Full,
        Self::Compact,
        Self::Pip,
        Self::Mark,
    ];

    /// The single-row ladder, densest first — the order [`Self::for_length`]
    /// tries them in.
    ///
    /// [`Self::Split`] is not on it. The ladder's whole shape is that a
    /// denser form is a wider one, so a shelf that grows never gets a poorer
    /// bar; the split form breaks that by being *narrower* than the full bar
    /// and deeper than any of them, which is why it is chosen a rung above
    /// the ladder rather than on it.
    pub const LADDER: [Self; 4] = [Self::Full, Self::Compact, Self::Pip, Self::Mark];

    /// This density's index into the per-density tables above.
    const fn rank(self) -> usize {
        match self {
            Self::Split => 0,
            Self::Full => 1,
            Self::Compact => 2,
            Self::Pip => 3,
            Self::Mark => 4,
        }
    }

    /// Whether this form is written on two rows.
    #[must_use]
    pub const fn is_split(self) -> bool {
        matches!(self, Self::Split)
    }

    /// The densest form that fits on a shelf `length` pixels long.
    ///
    /// [`Self::Pip`] is the floor rather than a fourth answer of "nothing":
    /// a bar too small to read is still where the seat's caret and its life
    /// are, and the seat sheet carries everything the pip form drops.
    #[must_use]
    pub fn for_length(length: f32, designated: bool) -> Self {
        for density in [Self::Full, Self::Compact, Self::Pip] {
            if length >= density.min_length(designated) {
                return density;
            }
        }
        Self::Mark
    }

    /// The best form a shelf `length` pixels long and `depth` pixels deep can
    /// hold — which is what the renderer asks.
    ///
    /// Two measurements rather than one, because [`Self::Split`] is the only
    /// form that wants the second: it needs less shelf along than the full
    /// bar and more across it than any single-row form, and a shelf that
    /// cannot hold two rows of ink would be drawing the second one on the
    /// creature lane behind it.
    ///
    /// Depth is asked *only* about the split form. The four single-row forms
    /// are a ladder in length and answering "too deep for a bar at all" with
    /// a poorer bar would be answering the wrong question: a shelf too
    /// shallow for the mark form is a table nothing can be written on, and
    /// dropping to a form that is no shorter would not help.
    #[must_use]
    pub fn for_shelf(length: f32, depth: f32, designated: bool) -> Self {
        if length >= Self::Split.min_length(designated) && depth >= Self::Split.ink_height() {
            return Self::Split;
        }
        Self::for_length(length, designated)
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
            Self::Split | Self::Full => 0.0,
            Self::Compact | Self::Pip | Self::Mark => 0.25,
        }
    }

    /// The cells this density draws, in the viewer's reading order.
    #[must_use]
    pub fn cells(self) -> Vec<Cell> {
        let mut out = vec![Cell::Caret, Cell::Swatch];
        match self {
            Self::Split | Self::Full | Self::Compact => {
                out.push(Cell::Name);
                out.push(Cell::Life);
                out.extend(Zone::ALL.map(Cell::Count));
                out.push(Cell::Hinge);
            }
            // The hand is the one count a player reads about an opponent
            // without being told to, so it is the one that survives. The
            // hinge goes: a turn number with no room for the tiles it belongs
            // to is a fact with nothing to hinge.
            Self::Pip => {
                out.push(Cell::Name);
                out.push(Cell::Life);
                out.push(Cell::Count(Zone::Hand));
            }
            // Nothing but the promise: the caret, the seat's colour, and the
            // step the game is in.
            Self::Mark => {}
        }
        out.push(Cell::Steps);
        out
    }

    /// The same cells, dealt into the rows they are drawn on.
    ///
    /// One row for every form but [`Self::Split`], which puts the steps on
    /// the first — the one along the shelf's outer edge, furthest from the
    /// lanes — and the seat's identity on the second. The hinge goes *with
    /// the steps*, because that is what a hinge is for: the turn number
    /// belongs to the seat whose bar it is and to the twelve tiles beside it,
    /// and a two-row bar is exactly the arrangement that could let those two
    /// halves drift into reading as two separate things.
    ///
    /// [`Self::cells`] stays the canonical list of what a form carries, and
    /// `every_row_is_dealt_from_the_cells_the_form_carries` is what stops the
    /// two from disagreeing.
    #[must_use]
    pub fn rows(self) -> Vec<Vec<Cell>> {
        if !self.is_split() {
            return vec![self.cells()];
        }
        let (mut steps, mut identity) = (Vec::new(), Vec::new());
        for cell in self.cells() {
            if matches!(cell, Cell::Hinge | Cell::Steps) {
                steps.push(cell);
            } else {
                identity.push(cell);
            }
        }
        vec![steps, identity]
    }

    /// How wide one row of cells is drawn, cells and the gaps between them.
    #[must_use]
    pub fn row_width(self, row: &[Cell], designated: bool) -> f32 {
        if row.is_empty() {
            return 0.0;
        }
        let gaps = CELL_GAP * (row.len() - 1) as f32;
        row.iter()
            .map(|cell| self.cell_width(*cell, designated))
            .sum::<f32>()
            + gaps
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
            Self::Split | Self::Full => 112.0,
            Self::Compact => 96.0,
            Self::Pip | Self::Mark => 56.0,
        }
    }

    /// How wide one step tile is drawn, and the gap between two of them.
    #[must_use]
    pub const fn tile_width(self) -> f32 {
        TILE_W[self.rank()]
    }

    /// The widest one step tile is drawn.
    ///
    /// The same as [`Self::tile_width`] for every form but [`Self::Split`],
    /// whose tiles have the shelf to themselves and grow into it. This is the
    /// only place a bar's ink is not a fixed number of pixels, and it is
    /// allowed to be one because the tiles grow *together*: nothing on the
    /// row can twitch relative to anything else on it.
    #[must_use]
    pub const fn tile_width_max(self) -> f32 {
        if self.is_split() {
            SPLIT_TILE_W_MAX
        } else {
            TILE_W[self.rank()]
        }
    }

    /// How tall one step tile is drawn.
    #[must_use]
    pub const fn tile_height(self) -> f32 {
        TILE_H[self.rank()]
    }

    /// How tall the bar's box is: the tile plus room above and below.
    ///
    /// The box is what answers the pointer, and it is allowed to be taller
    /// than the shelf — it draws nothing, so a box overhanging onto the empty
    /// top of the creature lane covers nothing and hides nothing. What the
    /// shelf has to hold is [`Self::ink_height`].
    #[must_use]
    pub fn height(self) -> f32 {
        self.ink_height() + 10.0 - HALO_OUT * 2.0
    }

    /// How tall the identity row of a two-row bar is drawn, and zero for
    /// every form that has no second row.
    #[must_use]
    pub const fn identity_height(self) -> f32 {
        if self.is_split() {
            SPLIT_IDENTITY_H
        } else {
            0.0
        }
    }

    /// The gap between the two rows, and zero for a bar that has one row.
    #[must_use]
    pub const fn row_gap(self) -> f32 {
        if self.is_split() { SPLIT_ROW_GAP } else { 0.0 }
    }

    /// How tall the **drawn** part of a bar is: the tile and its now-ring.
    ///
    /// This is the number a shelf's projected depth is measured against,
    /// because it is the ink that must sit on the ledge rather than on the
    /// row of creatures behind it.
    #[must_use]
    pub fn ink_height(self) -> f32 {
        self.tile_height() + HALO_OUT * 2.0 + self.row_gap() + self.identity_height()
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
        matches!(self, Self::Split | Self::Full)
    }

    /// Whether a step tile carries a glyph at all.
    ///
    /// The two small forms do not. At eleven pixels across a glyph is a
    /// smudge, and the frame, the fill and the ink already carry the standing
    /// order, where the game is and how much of the turn has gone — which is
    /// everything the tile is for. A form that drops the label always drops
    /// the glyph one size later, never the other way round, which is what
    /// `a_labelled_tile_is_a_tile_with_a_glyph_on_it` holds.
    #[must_use]
    pub const fn tiles_have_glyphs(self) -> bool {
        matches!(self, Self::Split | Self::Full | Self::Compact)
    }

    /// The twelve tiles and the eleven gaps between them.
    #[must_use]
    pub fn steps_width(self) -> f32 {
        let tiles = self.tile_width() * STEPS as f32;
        tiles + self.tile_gap() * (STEPS - 1) as f32
    }

    /// How wide the whole bar is drawn at this density.
    ///
    /// The **longer** of the rows for a two-row bar, not their sum: the two
    /// rows are stacked, so what the shelf has to be long enough for is
    /// whichever of them reaches further.
    #[must_use]
    pub fn width(self, designated: bool) -> f32 {
        self.rows()
            .iter()
            .map(|row| self.row_width(row, designated))
            .fold(0.0_f32, f32::max)
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
    ///
    /// Over the **ladder**, which is what `for_length` walks.
    /// [`Density::Split`] is deliberately narrower than the full bar — that
    /// is what a second row buys — and is chosen by
    /// [`Density::for_shelf`] instead.
    #[test]
    fn a_denser_bar_is_a_wider_bar() {
        for designated in [false, true] {
            let widths: Vec<f32> = Density::LADDER.map(|d| d.width(designated)).to_vec();
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
            let mins: Vec<f32> = Density::LADDER.map(|d| d.min_length(designated)).to_vec();
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
            for density in Density::LADDER {
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
            // last one it is marks all the way to nothing — a shelf too short
            // for a bar still has a seat on it.
            let full = Density::Full.min_length(designated);
            assert_eq!(
                Density::for_length(full - 1.0, designated),
                Density::Compact
            );
            let compact = Density::Compact.min_length(designated);
            assert_eq!(Density::for_length(compact - 1.0, designated), Density::Pip);
            let pip = Density::Pip.min_length(designated);
            assert_eq!(Density::for_length(pip - 1.0, designated), Density::Mark);
            assert_eq!(Density::for_length(0.0, designated), Density::Mark);
            assert_eq!(Density::for_length(-40.0, designated), Density::Mark);
        }
    }

    /// A tile that is labelled is a tile that has a glyph.
    ///
    /// The two go in one order — the label goes first, the glyph one size
    /// later — and the renderer used to ask the question the other way round:
    /// it drew a glyph unless the form was exactly [`Density::Pip`], which was
    /// right while `Pip` was the smallest form and put a ten-pixel glyph in an
    /// eight-by-ten tile the moment [`Density::Mark`] existed.
    #[test]
    fn a_labelled_tile_is_a_tile_with_a_glyph_on_it() {
        for density in Density::ALL {
            assert!(
                !density.tiles_are_labelled() || density.tiles_have_glyphs(),
                "{density:?} writes a label with no glyph beside it"
            );
            assert!(
                !density.tiles_have_glyphs() || density.tile_width() >= 12.0,
                "{density:?} draws a glyph in a {} pixel tile",
                density.tile_width()
            );
        }
    }

    /// A thinner bar never carries a cell a denser one drops.
    ///
    /// The four forms are one bar at four sizes, not four designs. If the
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

    /// Every form keeps the promise the design makes about a bar.
    ///
    /// §7 of the spec answers "where is the game" with *every bar marks the
    /// step the game is in*, so the steps are not optional at any size; the
    /// swatch is what says whose bar it is, in the same colour that seat's
    /// own rim wears; and the caret is what says who is holding priority.
    /// Life, the name and the counts all go at the floor and are carried by
    /// the seat sheet instead — because a bar wider than the mat it is
    /// written on is a bar written across the seats beside it, and on an
    /// eight-player ring a mat is a hundred and fifty pixels wide.
    #[test]
    fn no_form_drops_the_seat_or_the_turn() {
        for density in Density::ALL {
            let cells = density.cells();
            for wanted in [Cell::Caret, Cell::Swatch, Cell::Steps] {
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

    /// The ladder answers in the right order and changes hands where it says
    /// it does.
    ///
    /// Arithmetic about a strip of screen, which is all this crate can
    /// honestly claim: how long a *particular* table's shelf actually
    /// projects needs the camera, and the answer to "does a laptop duel get
    /// the compact bar" is therefore
    /// `table::camera_tests::a_duel_gets_the_bar_its_window_can_hold`, where
    /// there is a `Lens` to ask.
    ///
    /// This used to be that test, written against a shelf modelled as
    /// `window.x * 0.635`. A constant cannot be wrong about the projection it
    /// is standing in for and so it never failed: the real ratio at a duel is
    /// 0.659, which puts a 1440-wide window at 949 px and the **full** bar,
    /// while the test went on promising the compact one.
    #[test]
    fn the_ladder_hands_over_at_its_own_boundaries() {
        for designated in [false, true] {
            for density in Density::LADDER {
                let least = density.min_length(designated);
                assert_eq!(
                    Density::for_length(least, designated),
                    density,
                    "{density:?} should be the answer at its own floor"
                );
                // And a hair under it is the next form down, so no length
                // falls between two rungs. `Mark` is the floor and has no
                // rung below it.
                if density != Density::Mark {
                    assert_ne!(
                        Density::for_length(least - 0.5, designated),
                        density,
                        "{density:?} still answers below its floor"
                    );
                }
            }
            // Densest first, so a longer shelf never gets a poorer bar.
            let mut last = Density::Mark;
            for length in (0..2000u16).map(f32::from) {
                let got = Density::for_length(length, designated);
                assert!(
                    got.rank() <= last.rank(),
                    "a {length}px shelf got {got:?} after a shorter one got \
                     {last:?}"
                );
                last = got;
            }
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

    /// The rows of a bar hold exactly the cells the form says it carries.
    ///
    /// Two lists of the same thing, and the renderer builds from the rows
    /// while every invariant above is written about the cells — so a cell
    /// added to one and not the other is a cell that is either drawn twice
    /// or drawn nowhere, and nothing else would notice.
    #[test]
    fn every_row_is_dealt_from_the_cells_the_form_carries() {
        for density in Density::ALL {
            let mut dealt: Vec<Cell> = density.rows().concat();
            let mut carried = density.cells();
            assert_eq!(
                dealt.len(),
                carried.len(),
                "{density:?} deals {} cells into rows and carries {}",
                dealt.len(),
                carried.len()
            );
            // Order differs by design — the hinge leads the steps row while
            // it trails the cell list — so this is a comparison of contents.
            let key = |c: &Cell| format!("{c:?}");
            dealt.sort_by_key(key);
            carried.sort_by_key(key);
            assert_eq!(dealt, carried, "{density:?} deals cells it does not carry");
            assert_eq!(
                density.rows().len(),
                usize::from(density.is_split()) + 1,
                "{density:?} draws the wrong number of rows"
            );
        }
    }

    /// The split form is the trade it claims to be: shorter and deeper.
    ///
    /// Both halves matter and for different reasons. Shorter is what makes it
    /// reachable at all — a shelf that could only hold the compact bar in one
    /// row can hold every cell in two — and deeper is what
    /// [`Density::for_shelf`] has to check, because a bar that fits along a
    /// ledge and not across it is a row of ink on the creature lane.
    #[test]
    fn the_split_bar_asks_for_a_shorter_shelf_and_a_deeper_one() {
        for designated in [false, true] {
            assert!(
                Density::Split.min_length(designated) < Density::Full.min_length(designated),
                "the split bar asks for {} of shelf and the full bar for {}",
                Density::Split.min_length(designated),
                Density::Full.min_length(designated)
            );
            for other in Density::LADDER {
                assert!(
                    Density::Split.ink_height() > other.ink_height(),
                    "the split bar draws {} of ink and {other:?} draws {}",
                    Density::Split.ink_height(),
                    other.ink_height()
                );
            }
        }
    }

    /// A shelf long enough for two rows and too shallow for them gets the
    /// ladder, and the same shelf one pixel deeper gets the split bar.
    ///
    /// This is the whole of what the second measurement buys, and it is the
    /// boundary that would silently not exist if `for_shelf` forwarded to
    /// `for_length` — every wide duel would draw two rows of ink on a shelf
    /// with room for one.
    #[test]
    fn a_shelf_too_shallow_for_two_rows_gets_the_ladder() {
        for designated in [false, true] {
            let long = Density::Full.min_length(designated) + 400.0;
            let deep = Density::Split.ink_height();
            assert_eq!(
                Density::for_shelf(long, deep, designated),
                Density::Split,
                "a {long}×{deep} shelf holds two rows"
            );
            assert_eq!(
                Density::for_shelf(long, deep - 0.5, designated),
                Density::for_length(long, designated),
                "a shelf a hair too shallow should fall back to the ladder"
            );
            // And too short is the ladder too, however deep the shelf is.
            let short = Density::Split.min_length(designated) - 0.5;
            assert_eq!(
                Density::for_shelf(short, deep * 4.0, designated),
                Density::for_length(short, designated),
                "a shelf too short for two rows should fall back to the ladder"
            );
        }
    }

    /// Neither measurement can make a bar worse by growing.
    ///
    /// The ladder has this property along its one axis and
    /// `the_ladder_hands_over_at_its_own_boundaries` holds it there. With two
    /// axes it is easier to lose: a camera pulling back shortens *and*
    /// shallows a shelf at once, and a form that appeared halfway through
    /// that movement would be a bar that flickered as the player orbited.
    #[test]
    fn a_bigger_shelf_never_gets_a_poorer_bar() {
        for designated in [false, true] {
            for depth in (0..60u16).map(f32::from) {
                let mut last = Density::Mark;
                for length in (0..1400u16).step_by(7).map(f32::from) {
                    let got = Density::for_shelf(length, depth, designated);
                    assert!(
                        got <= last,
                        "a {length}×{depth} shelf got {got:?} after a shorter \
                         one got {last:?}"
                    );
                    last = got;
                }
            }
            for length in (0..1400u16).step_by(7).map(f32::from) {
                let mut last = Density::Mark;
                for depth in (0..60u16).map(f32::from) {
                    let got = Density::for_shelf(length, depth, designated);
                    assert!(
                        got <= last,
                        "a {length}×{depth} shelf got {got:?} after a \
                         shallower one got {last:?}"
                    );
                    last = got;
                }
            }
        }
    }

    /// A tile that grows never grows into a ribbon, and one that does not
    /// grow reports the width it is drawn at.
    #[test]
    fn only_the_split_tile_grows() {
        for density in Density::ALL {
            assert!(
                density.tile_width_max() >= density.tile_width(),
                "{density:?} caps its tile below the width it is drawn at"
            );
            assert_eq!(
                density.tile_width_max() > density.tile_width(),
                density.is_split(),
                "{density:?} disagrees with itself about whether it grows"
            );
            assert!(
                density.tile_width_max() <= density.tile_height() * 4.0,
                "a {density:?} tile grows to {} on a {} tall tile",
                density.tile_width_max(),
                density.tile_height()
            );
        }
    }
}
