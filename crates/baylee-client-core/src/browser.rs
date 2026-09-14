//! The zone browser: everything a choice can point at that is not on the
//! table.
//!
//! The battlefield is drawn as cards and the hand is drawn as cards, so a
//! choice among *those* needs no help — the player clicks the thing. Every
//! other zone a pending choice can reach is a number on a mat: a graveyard,
//! an exile pile, a command zone, the stack, and above all `looking_at` —
//! the cards the engine is *showing* this seat, which live in no zone it can
//! see at all. A library search offers seven object ids that appear nowhere
//! on screen; before this module the client's only honest answer was to
//! confirm whatever the interaction had defaulted to.
//!
//! # What is deliberately not in here
//!
//! [`BrowseZone`] has no `Hand` and no `Battlefield` variant, and that is
//! load-bearing rather than an omission. The browser is the *complement* of
//! what the table and the hand bar already make clickable, which is what
//! lets the invariant test mean something: "every id the engine offered is
//! drawn somewhere" is only a real claim while `BoardModel` and `Browser`
//! cover disjoint halves of it. A browser that also listed the hand would
//! satisfy that test on its own and prove nothing.
//!
//! # The state it keeps, and the state it does not
//!
//! [`Interaction`] remains the single truth about the answer being
//! assembled. A [`Browser`] holds only what the *player* has said about the
//! panel — whether it is open, which zone tab is showing, what is typed in
//! the filter — and derives every row from `(&PlayerView, &Interaction)` the
//! way [`BoardModel`](crate::BoardModel) derives lanes from a view. Two
//! copies of a selection cannot disagree if there is only one.

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_view::PlayerView;
use std::collections::{BTreeSet, HashMap};

use crate::i18n::Phrase;
use crate::images::{ArtSize, ImageKey};
use crate::interaction::Interaction;

/// A zone the browser can show.
///
/// Ordered as the tabs are: what the engine is showing first, because a
/// choice that opens the tray is nearly always about those cards. `Ord` is
/// that same tab order and the sort relies on it: whatever the rows are sorted
/// by, they stay grouped by zone, because the tabs are the panel's first
/// structure and a list that interleaved a graveyard with an exile would have
/// thrown it away.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum BrowseZone {
    /// Cards the engine is showing this seat — a search, a scry, a reveal.
    /// They belong to no zone the seat can otherwise see.
    Looking,
    /// The stack. Listed because a spell or ability can be a target, and
    /// because a player wants to read what is about to resolve.
    Stack,
    /// A seat's graveyard.
    Graveyard(PlayerId),
    /// A seat's public exile.
    Exile(PlayerId),
    /// A seat's command zone.
    Command(PlayerId),
}

impl BrowseZone {
    /// Which seat's pile this is, when it belongs to one.
    #[must_use]
    pub fn seat(self) -> Option<PlayerId> {
        match self {
            Self::Looking | Self::Stack => None,
            Self::Graveyard(p) | Self::Exile(p) | Self::Command(p) => Some(p),
        }
    }

    /// The panel a pile standing beside the table opens, when it opens one.
    ///
    /// `None` for a library, and that is the whole reason this is an
    /// `Option`: nobody may look through a library, its owner included
    /// (CR 401.2), so there is no panel for one to open and the pile beside
    /// the mat has to be inert rather than merely empty.
    #[must_use]
    pub const fn of_pile(pile: crate::layout::PileKind, player: PlayerId) -> Option<Self> {
        use crate::layout::PileKind;
        match pile {
            PileKind::Library => None,
            PileKind::Graveyard => Some(Self::Graveyard(player)),
            PileKind::Exile => Some(Self::Exile(player)),
            PileKind::Command | PileKind::Command2 => Some(Self::Command(player)),
        }
    }

    /// How many cards are in it.
    ///
    /// The one number the deleted pile chips carried that nothing else on the
    /// sheet did. Read from the same fields [`Browser::zones`] reads, so a tab
    /// that exists is a tab with a non-zero count.
    #[must_use]
    pub fn count_in(self, view: &PlayerView) -> usize {
        let pile = |zones: &[Vec<baylee_view::PublicObject>], seat: PlayerId| {
            zones.get(seat.get() as usize).map_or(0, Vec::len)
        };
        match self {
            Self::Looking => view.looking_at.len(),
            Self::Stack => view.stack.len(),
            Self::Graveyard(p) => pile(&view.graveyards, p),
            Self::Exile(p) => pile(&view.exile, p),
            Self::Command(p) => pile(&view.command, p),
        }
    }

    /// What the zone is called.
    #[must_use]
    pub fn label(self) -> Phrase {
        match self {
            Self::Looking => Phrase::BrowseLooking,
            Self::Stack => Phrase::StackTitle,
            Self::Graveyard(_) => Phrase::BrowseGraveyard,
            Self::Exile(_) => Phrase::BrowseExile,
            Self::Command(_) => Phrase::BrowseCommand,
        }
    }
}

/// One card in the browser.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BrowseRow {
    /// The object this row stands for.
    pub id: ObjectId,
    /// The name a player reads on this row.
    ///
    /// The projected one — a clone shows the name it copied — put through
    /// whatever [`Names`] the caller brought, so it is in the language the
    /// printing this seat chose is printed in. It is the string the row
    /// draws **and** the string the filter and [`SortKey::Name`] work on,
    /// which is the whole of why the seam exists: a panel that showed
    /// "Wald" and then found nothing when "Wald" was typed into the box
    /// under it was showing one name and searching another.
    pub name: String,
    /// Its picture, when it has one. A token in a graveyard has none.
    pub art: Option<ImageKey>,
    /// Where it is.
    pub zone: BrowseZone,
    /// How the pending question stands towards this row.
    pub standing: RowStanding,
    /// Its one-based place in an ordering, for `Pending::OrderObjects`.
    ///
    /// `None` for every other choice: a number beside a card in a plain
    /// "choose two" would be claiming the order matters when it does not.
    pub place: Option<usize>,
    /// Its projected mana value — what [`SortKey::ManaValue`] sorts on, and
    /// what the row shows beside the name.
    pub mana_value: u32,
    /// Its projected types, for [`SortKey::Type`] and the row's type line.
    pub types: TypeSet,
    /// Whether it is a token rather than a card.
    ///
    /// A graveyard holds both, and they are not the same thing: a token
    /// ceases to exist the next time state-based actions are checked (CR
    /// 111.7), so a row that looked like a card there would be inviting a
    /// player to plan around something that is about to be gone.
    pub token: bool,
}

/// How the pending question stands towards one row.
///
/// Three independent facts and not a ladder — a row can be offered and not
/// chosen, chosen and not stood on, stood on and not offered, because the
/// list shows cards the offer does not reach and the focus walks the offer
/// rather than the list.
///
/// They travel together because they are drawn together and because the last
/// of them is what made four bools on a row: grouping them says out loud
/// what the three have in common, which is that each is about *this
/// question* and none is about the card.
///
/// The split inside the group is worth knowing. `selectable` and `selected`
/// are claims about the **answer** — what the engine will take, and what
/// this seat has put in it. `focused` is a claim about the **keyboard**:
/// where the next press would land, decided by the client alone and true
/// even when no answer has been touched.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct RowStanding {
    /// Whether the pending choice would accept this row.
    pub selectable: bool,
    /// Whether it is part of the answer being assembled.
    pub selected: bool,
    /// Whether the keyboard's focus is standing on it.
    pub focused: bool,
}

/// The name a card is printed with, in the language this seat reads.
///
/// The same seam as [`board::Registry`](crate::board::Registry) and for the
/// same reason: the answer is in the *catalog*, which is a printing's text
/// in a player's own language, and this crate reaches neither the catalog
/// nor the card registry — so the lookup arrives as an argument.
///
/// A view's `name` is the **projection**: what the rules say the object is
/// called, in the engine's one language, which is English. Everywhere else
/// on the screen the client has already replaced it — the table, the hand,
/// the hover preview and this panel's own rows all draw the printing's name
/// — and the browser was the one place that drew the translated name while
/// deciding with the untranslated one.
///
/// One lookup and not two, unlike `Registry`: a row shows a name and sorts
/// by it, and both are that one string. What it must **not** do is replace
/// `object.name` outright, which is why [`Browser::rows`] keeps matching the
/// filter against both — a player who learned a card in English is still
/// allowed to type it.
#[derive(Clone, Copy)]
pub struct Names<'a> {
    /// What this seat's chosen printing calls the object, if anything does.
    ///
    /// `None` is the honest answer and not a failure: a token, an ability on
    /// the stack whose source has no text, or a client running against a
    /// gateway with no catalog behind it. The projected name stands.
    pub shown: &'a dyn Fn(&baylee_view::PublicObject) -> Option<String>,
}

/// The answer a caller with no card text in reach has: none at all.
fn nothing_shown(_: &baylee_view::PublicObject) -> Option<String> {
    None
}

impl Names<'_> {
    /// The names the view already carries, untranslated.
    ///
    /// Not a test stub — a real answer, for a caller that genuinely has no
    /// printing text to offer: the integration tests today, an embedder
    /// with no gateway behind it later. (A seat *waiting* on its catalog is
    /// not one of them: the renderer always builds the closure, and the
    /// falling back happens inside it.) The panel then behaves exactly as
    /// it did before any of this existed,
    /// which is the property that makes every test in this file that is not
    /// about language keep being about what it is about.
    #[must_use]
    pub fn projected() -> Names<'static> {
        static SHOWN: fn(&baylee_view::PublicObject) -> Option<String> = nothing_shown;
        Names { shown: &SHOWN }
    }
}

/// What the browser sorts its rows by.
///
/// Every one of these reads a field the view already projects, so none of it
/// is a rules decision — arithmetic on projected numbers, which is the line
/// `docs/design.md` §6 draws around what the client may compute for itself.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum SortKey {
    /// The zone's own order, which is the order the cards are in.
    ///
    /// The default, and the only one of the four that is not a sort at all: a
    /// graveyard is a stack of cards in the order they arrived, and that order
    /// is information — it is what "the top card of your graveyard" means.
    #[default]
    Place,
    /// By name — the one on the row, in that name's own alphabet.
    ///
    /// Through [`prose::sort_key`](crate::prose::sort_key), so it is the
    /// reader's alphabet and not the byte order that files every accented
    /// letter above `z`.
    Name,
    /// By mana value.
    ManaValue,
    /// By type, in the order a permanent is usually read: creatures, then
    /// other nonland permanents, then lands, then instants and sorceries.
    Type,
}

impl SortKey {
    /// All four, in the order a control should offer them.
    pub const ALL: [Self; 4] = [Self::Place, Self::Name, Self::ManaValue, Self::Type];

    /// The button's label.
    #[must_use]
    pub const fn label(self) -> Phrase {
        match self {
            Self::Place => Phrase::SortByPlace,
            Self::Name => Phrase::SortByName,
            Self::ManaValue => Phrase::SortByCost,
            Self::Type => Phrase::SortByType,
        }
    }

    /// The next key round the ring — one button rather than a menu, which is
    /// what four options deserve.
    #[must_use]
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }
}

/// Which shape the same rows are drawn in.
///
/// It changes **nothing** about which cards are on the sheet: the filter, the
/// ticked zones and the sort decide the rows, and this decides how each one is
/// written. That is why it is not a field of [`Browser`] — there is no model
/// question it answers — and why the sheet's own arithmetic stays the detailed
/// row's: `TRAY_ROWS`, [`Placement::DEFAULT_H`] and [`Placement::MIN_H`]
/// describe the panel a player opens, and a panel that resized itself when the
/// view changed would move a sheet the player had put somewhere.
///
/// The value lives in the client's settings beside the sheet's rectangle, for
/// the reason `prefer_text_view` does: it is taste, it is worth keeping across
/// launches, and an account is not needed to have it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ViewMode {
    /// A row is a checkbox, a thumbnail, a name, the cost, the type line and
    /// the pile it is in.
    ///
    /// The default, because the sheet exists to be *chosen from* and choosing
    /// is reading: a fetchland asks which of ninety lands, and the answer is
    /// in the type line and the cost. The other two are for browsing, which is
    /// the rarer half.
    #[default]
    Detailed,
    /// The same list with the asides dropped and the picture doubled.
    ///
    /// For the player who knows the pile and is looking for a card they can
    /// already name by its art.
    Large,
    /// Every card in the zone as a tile, laid out across and wrapped.
    ///
    /// The owner's *"wie auf einer Produktseite"*. It is the one view that
    /// grows sideways, which is why it took a mode to justify: see the module
    /// doc of `hud::tray`, where the list and the grid argue it out.
    Grid,
}

impl ViewMode {
    /// All three, in the order a control should offer them — least card to
    /// most.
    pub const ALL: [Self; 3] = [Self::Detailed, Self::Large, Self::Grid];

    /// The spelling this is stored under, and the only name a mode has.
    ///
    /// There is deliberately no translated label beside it. The three
    /// segments are icons, nothing in the client reads a word for them, and
    /// a `Phrase` nobody renders is prose the i18n tests bless and no player
    /// ever sees — `BrowseTitle` was retired for exactly that. The words
    /// come back with whatever first needs them: a tooltip, the keyboard
    /// map, or the menu the actions bar is about to grow.
    ///
    /// A stored name would not be that label in any case: a label is
    /// translated and this may never be.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Detailed => "detailed",
            Self::Large => "large",
            Self::Grid => "grid",
        }
    }
}

/// One row of the grid: how many tiles fit across `measure`, how wide each
/// one is, and how much air stands between two of them.
///
/// The grid's whole layout, and it is here rather than in the renderer for
/// the reason [`Placement`] is: it is arithmetic, and arithmetic with a
/// window in front of it is arithmetic nobody tests.
///
/// The rule is `seatbar::Density::for_length`'s, applied to cards instead of
/// step tiles: **fit as many as will go at `min`, then grow them together
/// towards `max` and put whatever is left over in the gaps.** Growing into
/// the gaps rather than into the picture is the point — card art has exactly
/// one useful size (see `TRAY_BIG_THUMB_W` in `hud::tray`) and a tile wider
/// than `max` is a blurred card, so the slack has to go somewhere that is not
/// the card.
///
/// At least one column always comes back, even from a measure narrower than
/// one tile: a sheet dragged to its floor shows a card that overhangs by a
/// few pixels, which the list's own clip takes, and a grid of zero columns
/// would be a blank panel with a hundred cards behind it.
#[must_use]
pub fn grid_across(measure: f32, min: f32, max: f32, gap: f32) -> (usize, f32, f32) {
    // `+ gap` on both sides is the fencepost: n tiles have n-1 gaps, so
    // measuring in "tile plus gap" units over-counts by exactly one gap and
    // the numerator has to carry it too.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let across = (((measure + gap) / (min + gap)).floor() as usize).max(1);
    #[allow(clippy::cast_precision_loss)]
    let n = across as f32;
    let each = ((measure - (n - 1.0) * gap) / n).clamp(min, max);
    // Only a capped row has anything left over, and a single column has no
    // gap to put it in — so both of those keep the gap they were given and
    // the row simply ends short of the right edge, which is what a last row
    // of three tiles does anyway.
    let air = if across > 1 && each >= max {
        ((measure - n * each) / (n - 1.0)).max(gap)
    } else {
        gap
    };
    (across, each, air)
}

impl serde::Serialize for ViewMode {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_str(self.name())
    }
}

/// A name this build does not know is read as the default, not as a refusal.
///
/// The derived reader would answer an unknown variant with an error, and the
/// client's settings store is `serde_json::from_str(…).ok().unwrap_or_default()`
/// — so one view mode retired in a later build would take that player's sheet
/// placement, their language and their remembered address down with it. That
/// is exactly the hole `Keymap` fell into over a retired action, written out
/// here before it can be fallen into a second time.
impl<'de> serde::Deserialize<'de> for ViewMode {
    fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let said = String::deserialize(input)?;
        Ok(Self::ALL
            .into_iter()
            .find(|mode| mode.name() == said)
            .unwrap_or_default())
    }
}

/// Where a type line sits in [`SortKey::Type`]'s order.
///
/// A permanent is several types at once, so this is a precedence and not a
/// lookup — the same shape as `board::lane_of`, and for the same reason.
fn type_rank(types: TypeSet) -> u8 {
    if types.contains(TypeSet::CREATURE) {
        0
    } else if types.contains(TypeSet::LAND) {
        3
    } else if types.contains(TypeSet::INSTANT) || types.contains(TypeSet::SORCERY) {
        4
    } else {
        1
    }
}

/// Where the sheet stands, and how big it is.
///
/// Logical pixels, and **relative to the band** — the strip of screen between
/// the phase rail and the hand bar, which is the only part of the window the
/// sheet is ever allowed into. Pixels rather than fractions of the band on
/// purpose: what is inside is a grid of cards at a fixed size, so a sheet that
/// scaled with the window would show a different number of columns on every
/// screen and none of them the number the player chose.
///
/// It lives here rather than in the renderer because the whole of it is
/// arithmetic — clamping a rectangle into another rectangle — and arithmetic
/// with a window in front of it is arithmetic nobody tests.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Placement {
    /// Distance from the band's left edge.
    pub left: f32,
    /// Distance from the band's top edge.
    pub top: f32,
    /// Width of the sheet.
    pub width: f32,
    /// Height of the sheet.
    pub height: f32,
}

impl Placement {
    /// A row with ten characters of name left in it: the fixed furniture a
    /// row carries — the checkbox, the thumbnail, four pips of cost and the
    /// zone badge — plus the shortest name worth reading. Below this the
    /// prose is squeezed out and the row is a line of marks.
    pub const MIN_W: f32 = 356.0;
    /// The head, the footer, and two whole rows between them.
    ///
    /// It was 314, with the zone tabs on a row of their own. They went into
    /// the title bar on 14.09.2026, which is `TRAY_TAB_H` and one
    /// `TRAY_HEAD_GAP` — 30 px — that the chrome no longer spends, and the
    /// floor comes down by exactly that so it stays "two whole rows" rather
    /// than two and a bit.
    pub const MIN_H: f32 = 284.0;
    /// One row wide — what `TRAY_PANEL_W` in the renderer computes.
    ///
    /// It was 854, ten columns of a card grid, because the sheet used to draw
    /// cards and a search wanted forty of them at once. The dialog is a list
    /// now (`docs/redesign-proposal.md` §6), and a list is read down rather
    /// than across: past a comfortable measure, width buys a longer blank
    /// stretch in the middle of every row and nothing else. Height is what
    /// buys rows, and it is the axis that grew.
    ///
    /// It moved from 661 to 662 on the change of text face: the zone badge is
    /// measured out of the shipped font and `Kommandozone` sets 1.2 px wider
    /// in Alegreya Sans than it did in Inter. That is the seam this number
    /// and `TRAY_PANEL_W` exist to keep honest, and it is the whole of what a
    /// change of family cost the layout.
    ///
    /// It moved again, 662 → 692, when `TRAY_TYPE_W` was measured against the
    /// type lines a **German** catalog prints rather than against the one
    /// English line it had been taken from. 30 px is what it costs to stop
    /// clipping one row in thirteen at a table of legends; the reasoning is
    /// on that constant.
    ///
    /// And 692 → 702 on 14.09.2026, which is the thumbnail column and nothing
    /// else: the owner asked for a bigger picture on each row, and a column
    /// ten pixels wider is ten pixels of row. [`Self::MIN_W`] moved with it,
    /// both being the same row with a different amount of name left in it.
    pub const DEFAULT_W: f32 = 702.0;
    /// The chrome, and seven rows and a **half**.
    ///
    /// The half row is the point. A grid was cut to four whole rows because
    /// most of a fifth was space nothing could ever be put in; a list is the
    /// other way round — a row cut through by the bottom edge is what says
    /// the list continues, and it says it without a scrollbar.
    ///
    /// The chrome it is derived from cannot be computed here: the header, the
    /// zone tabs, the controls and the footer are text line boxes, which are
    /// font metrics rather than constants. `TRAY_CHROME_H` in the renderer is
    /// that measurement and `the_default_height_shows_half_a_row` holds this
    /// number to the arithmetic over it.
    ///
    /// It is derived with the **footer present**, which a hand-opened sheet
    /// has no use for: the sheet a question opens is the one whose size
    /// nobody chose, because a sheet opened by hand is furniture the player
    /// drags, resizes and keeps ([`Browser::placement`]).
    ///
    /// It was 649 and eight and a half rows. The owner asked for taller rows
    /// on 14.09.2026, which raises a row from 55.9 to 69.9 — so the count came
    /// down half a row with it and the sheet opens 49 px taller rather than
    /// 119. Spending the whole of a taller row on more sheet would have put
    /// this at 768 against the 850 the band has on the screen it was measured
    /// on, which is a dialog that reads as a screen.
    ///
    /// It was 698 until the zone tabs moved into the title bar later the same
    /// day. That is one row of chrome and one gap — 30 px — and the sheet
    /// gives them back rather than keeping them as an eighth row: the count
    /// of rows is what the owner chose, the chrome is what it costs.
    pub const DEFAULT_H: f32 = 668.0;
    /// The clear the sheet keeps between itself and the band's edge.
    const MARGIN: f32 = 12.0;

    /// The sheet a player who has never moved it gets: the default size, or
    /// as much of it as the band has room for, in the middle.
    #[must_use]
    pub fn centred(band: (f32, f32)) -> Self {
        let width = Self::DEFAULT_W;
        let height = Self::DEFAULT_H;
        Self {
            left: (band.0 - width) / 2.0,
            top: (band.1 - height) / 2.0,
            width,
            height,
        }
        .fit(band)
    }

    /// The same rectangle, brought inside the band.
    ///
    /// Size first and position second, and that order is the whole of it: a
    /// sheet moved before it was shrunk can be pushed off the far edge by its
    /// own width. It never writes itself back to the store either — a window
    /// briefly dragged small must not overwrite where the player put the
    /// sheet on the screen they actually play on.
    ///
    /// A band smaller than the minimum loses: the sheet keeps its minimum and
    /// overflows, because a sheet clamped to nothing is a sheet that is not
    /// there.
    #[must_use]
    pub fn fit(self, band: (f32, f32)) -> Self {
        let room = |band: f32, min: f32| (band - 2.0 * Self::MARGIN).max(min);
        let width = self.width.clamp(Self::MIN_W, room(band.0, Self::MIN_W));
        let height = self.height.clamp(Self::MIN_H, room(band.1, Self::MIN_H));
        let place = |at: f32, size: f32, band: f32| {
            at.clamp(Self::MARGIN, (band - Self::MARGIN - size).max(Self::MARGIN))
        };
        Self {
            left: place(self.left, width, band.0),
            top: place(self.top, height, band.1),
            width,
            height,
        }
    }

    /// The whole band, less the margin every other placement keeps.
    ///
    /// It is deliberately *exactly* [`Self::fit`]'s ceiling rather than a
    /// larger rectangle trusting the clamp, so that a maximised sheet is a
    /// fixed point: fitting it again on a window that has not changed leaves
    /// it alone, and [`Self::is_maximised`] can therefore be an equality.
    #[must_use]
    pub fn maximised(band: (f32, f32)) -> Self {
        Self {
            left: Self::MARGIN,
            top: Self::MARGIN,
            width: band.0 - 2.0 * Self::MARGIN,
            height: band.1 - 2.0 * Self::MARGIN,
        }
        .fit(band)
    }

    /// Whether this is the band filled.
    ///
    /// A tolerance and not an equality, because the sheet is written to a
    /// `Node` in logical pixels and read back through a band that a resized
    /// window recomputes: a maximised sheet on a window dragged one pixel
    /// wider must still restore rather than maximise a second time.
    #[must_use]
    pub fn is_maximised(self, band: (f32, f32)) -> bool {
        let full = Self::maximised(band);
        (self.left - full.left).abs() < 2.0
            && (self.top - full.top).abs() < 2.0
            && (self.width - full.width).abs() < 2.0
            && (self.height - full.height).abs() < 2.0
    }

    /// The same rectangle moved by a pointer delta, still inside the band.
    #[must_use]
    pub fn moved_by(self, delta: (f32, f32), band: (f32, f32)) -> Self {
        Self {
            left: self.left + delta.0,
            top: self.top + delta.1,
            ..self
        }
        .fit(band)
    }

    /// The same rectangle resized from its bottom-right corner.
    #[must_use]
    pub fn resized_by(self, delta: (f32, f32), band: (f32, f32)) -> Self {
        Self {
            width: self.width + delta.0,
            height: self.height + delta.1,
            ..self
        }
        .fit(band)
    }
}

/// Whether the sheet is showing, and — while it is — *whose doing* that is.
///
/// The second half is what lets the sheet shut itself when the search that
/// opened it is answered: a fetchland puts the library on screen, the player
/// picks a land, and the sheet has nothing left to say. A panel the player
/// opened by hand is never closed by anything but their hand.
///
/// Three states rather than two flags, and not only because clippy counts
/// bools: "shut but opened for a choice" is not a state, and a pair of
/// booleans is a type that can spell it.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Opening {
    /// Not showing.
    #[default]
    Shut,
    /// The player opened it and the player closes it.
    ByHand,
    /// The client opened it for a question, and takes it away with the
    /// question.
    ForChoice,
}

/// The panel's own state — what the player has said about it, nothing more.
#[derive(Clone, Default, Debug)]
pub struct Browser {
    open: Opening,
    /// Which zones are ticked, and **empty means every one of them**.
    ///
    /// A set rather than the `Option<BrowseZone>` this was, because the owner
    /// asked on 14.09.2026 for a checkbox at the start of each zone's name so
    /// that ticking several *merges* them. The empty set is not a fourth
    /// state to remember: it is what "Alle" means, so unticking the last zone
    /// lands back on everything instead of on a panel showing nothing, and
    /// there is no arrangement of ticks the model cannot spell.
    ///
    /// A `BTreeSet` and not a `HashSet` for the reason [`BrowseZone`]'s `Ord`
    /// exists: the tab order is the panel's first structure, and a set that
    /// iterated in a different order every run would be a third opinion about
    /// it.
    tabs: BTreeSet<BrowseZone>,
    /// The tab the *question* pins, when every card it offers is in one zone.
    ///
    /// Beside `tabs` rather than inside it because the two answer different
    /// questions: `tabs` is what is showing, `locked` is whether the player
    /// may change it. A search puts seven library cards on the sheet and the
    /// graveyard beside them has nothing to do with the question — so the
    /// other tabs are drawn and are not buttons, which is the owner's
    /// "ausgrauen" said in state.
    locked: Option<BrowseZone>,
    /// The search box, and a whole field rather than a string.
    ///
    /// It used to be a `String` that keystrokes were pushed onto and popped
    /// off, which is a field with no caret, no selection, no arrow keys and
    /// no paste — and the owner named the lobby's boxes as the thing this one
    /// should be. It is the same [`crate::textbuf::TextBuffer`] those use, so
    /// the client has one answer to what a text field does and both places
    /// give it.
    filter: crate::textbuf::TextBuffer,
    sort: SortKey,
    descending: bool,
    typing: bool,
    typing_epoch: u64,
    /// The cards the engine was showing this seat the last time a view came
    /// in — the memory [`Self::saw_reveal`] needs to spot a *new* one.
    ///
    /// Ids and not a count, because a reveal that ends and another that
    /// begins in the same frame is two reveals and the same length.
    looking_seen: Vec<ObjectId>,
}

impl Browser {
    /// A closed browser showing everything.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the panel is showing.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open != Opening::Shut
    }

    /// The zones that are ticked. Empty is "Alle", and means every one.
    ///
    /// Read by the renderer to draw the boxes and by the redraw gate to tell
    /// one arrangement of ticks from another. Ask [`Self::shows`] rather than
    /// this when the question is "does this zone's list appear": the empty
    /// set answers that differently from how it reads.
    #[must_use]
    pub const fn ticked(&self) -> &BTreeSet<BrowseZone> {
        &self.tabs
    }

    /// Whether this zone's box carries a tick.
    #[must_use]
    pub fn is_ticked(&self, zone: BrowseZone) -> bool {
        self.tabs.contains(&zone)
    }

    /// Whether nothing is ticked, which is what "Alle" says.
    #[must_use]
    pub fn shows_every_zone(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Whether this zone's cards are in the list.
    ///
    /// The one place the empty set's meaning lives. Every reader that wants
    /// "is this zone being listed" goes through here, so a ticked set and a
    /// merged list cannot disagree about what "Alle" is.
    #[must_use]
    pub fn shows(&self, zone: BrowseZone) -> bool {
        self.tabs.is_empty() || self.tabs.contains(&zone)
    }

    /// The tab the pending question pins, when it pins one.
    ///
    /// The renderer draws every other tab and makes none of them a button.
    /// Drawn rather than hidden: a tab that vanished would say the graveyard
    /// is empty, and what is true is that it is not part of *this* question.
    #[must_use]
    pub fn locked(&self) -> Option<BrowseZone> {
        self.locked
    }

    /// Whether a *question* is what put the sheet on screen.
    ///
    /// It decides where the sheet stands ([`Self::placement`]) and whether it
    /// can be dragged at all — a sheet the player did not arrange is not
    /// furniture. It does **not** decide the dim; see [`Self::dims_the_table`]
    /// for why that is a narrower question than this one.
    #[must_use]
    pub fn for_choice(&self) -> bool {
        self.open == Opening::ForChoice
    }

    /// Whether the table behind the sheet should go dark.
    ///
    /// The owner's W2, and the predicate is [`Self::locked`] rather than
    /// [`Self::for_choice`]. A question opens this sheet whenever any of its
    /// answers is somewhere the table cannot show — which is not the same as
    /// *every* answer being in here. A `ChooseCards` that spans the cards
    /// being revealed and the player's own hand opens the sheet `ForChoice`
    /// and locks no tab, and a dim on that question would darken the hand the
    /// player has to click. `locked` is already the stronger claim — one zone
    /// holds every option — so it is the one that says "there is nothing else
    /// to do".
    ///
    /// A reveal with no choice attached is `ForChoice` too and locks nothing,
    /// which falls out the same way and is right for the same reason: cards
    /// being shown are not a question, and nothing is waiting on the player.
    #[must_use]
    pub fn dims_the_table(&self) -> bool {
        self.is_open() && self.locked.is_some()
    }

    /// Whether this sheet is where the pending question gets answered.
    ///
    /// Two surfaces can draw a Confirm for one question — the dialog's own
    /// footer and the prompt slip's answer row — and for a `ChooseCards` they
    /// both did, so the player was shown the same button twice, in two
    /// places, and had to work out whether the two meant the same thing. They
    /// do. The dialog's is the one to keep: §6 of the table design says a
    /// dialog is a place you *work*, and the footer stands under the rows the
    /// answer is made of. This is the single predicate both surfaces read —
    /// the sheet draws its tally and its footer exactly when it is true, and
    /// the slip draws no answers exactly then.
    ///
    /// It is [`Self::for_choice`] and **not** [`Self::dims_the_table`]: a
    /// choice whose answers span this sheet and the player's hand is still
    /// sent from here, there being nowhere else to send it from, even though
    /// the table behind it stays lit.
    ///
    /// And it is narrower than the interaction's own [`Interaction::bounds`],
    /// which is what the footer read before: a graveyard opened **by hand**
    /// while the engine is asking about the battlefield is not that question's
    /// dialog, and it grew a tally and a Confirm for a question none of its
    /// rows could answer.
    #[must_use]
    pub fn answers_here(&self, interaction: Option<&Interaction>) -> bool {
        self.for_choice() && interaction.is_some_and(|it| it.is_mine() && it.bounds().is_some())
    }

    /// Where the sheet stands this time.
    ///
    /// A sheet the player opened is where they last dragged it, because that
    /// is what dragging it means. A sheet a *question* opened is **centred,
    /// every time**, and never reads the store at all.
    ///
    /// That second rule is the owner's W1, and the measurement says why a
    /// clamp was not enough: the remembered rectangle was 854 wide at
    /// `left: 437`, which is centred in a 1728-pixel band and 164 pixels left
    /// of centre in the 2056-pixel one it was drawn in. [`Placement::fit`]
    /// brings a rectangle *inside* a band and has no opinion about the middle
    /// of it, so a placement remembered from a smaller window stays where it
    /// was and simply looks misplaced. Nobody dragged that sheet anywhere; it
    /// was put there by a window that is not this one.
    #[must_use]
    pub fn placement(&self, band: (f32, f32), stored: Option<Placement>) -> Placement {
        match stored {
            Some(place) if !self.for_choice() => place.fit(band),
            _ => Placement::centred(band),
        }
    }

    /// What is typed in the filter.
    #[must_use]
    pub fn filter(&self) -> &str {
        self.filter.text()
    }

    /// The box itself — its text, its caret and its selection.
    ///
    /// What a renderer needs to draw a field rather than a string, through
    /// [`crate::textbuf::TextBuffer::segments`], which is the same door the
    /// lobby's boxes are drawn through.
    #[must_use]
    pub const fn filter_field(&self) -> &crate::textbuf::TextBuffer {
        &self.filter
    }

    /// Opens the panel on every zone.
    pub fn open(&mut self) {
        self.open = Opening::ByHand;
    }

    /// Opens the panel on one zone — what a tap on a pile does.
    ///
    /// Refused while the panel is holding a question, for the reason
    /// [`Self::show`] gives and one harder half: this is the only door that
    /// writes the ticks without asking the lock, and it also turns a
    /// `ForChoice` opening into a by-hand one — which takes the sheet away
    /// from the question it was opened for, so `answers_here` goes false and
    /// the question's own keys stop working on a dialog that is still on the
    /// screen. A tap that lands on a pile while a search is standing open is
    /// not a request to abandon the search.
    ///
    /// It **replaces** the ticks rather than adding one: a tap on a pile says
    /// "show me that pile", and a pile quietly joining a merge the player
    /// built earlier would be answering a question nobody asked.
    pub fn open_at(&mut self, zone: BrowseZone) {
        if self.open == Opening::ForChoice {
            return;
        }
        self.open = Opening::ByHand;
        self.tabs = std::iter::once(zone).collect();
    }

    /// Closes the panel, keeping the tab and filter for the next time.
    pub fn close(&mut self) {
        self.open = Opening::Shut;
        self.typing = false;
        // The lock belongs to the question, and the question is over. Kept
        // any longer it would pin the next sheet the player opened by hand to
        // a zone chosen by something that has already been answered.
        self.locked = None;
    }

    /// Shows one zone alone, or every zone when given `None`.
    ///
    /// Refused while a question has pinned a tab: the rows in every other
    /// zone answer nothing, so the model says no rather than relying on the
    /// renderer to have drawn no button.
    pub fn show(&mut self, tab: Option<BrowseZone>) {
        if self.locked.is_none() {
            self.tabs = tab.into_iter().collect();
        }
    }

    /// Ticks a zone's box, or unticks it — which is what merges two piles into
    /// one list.
    ///
    /// Unticking the last one leaves the empty set, and the empty set is
    /// "Alle": there is no way to arrive at a panel showing nothing, and no
    /// rule about a minimum to remember, because [`Self::shows`] is where the
    /// meaning of empty lives.
    ///
    /// Refused while a question has pinned a tab, for [`Self::show`]'s reason.
    pub fn tick(&mut self, zone: BrowseZone) {
        if self.locked.is_some() {
            return;
        }
        if !self.tabs.remove(&zone) {
            self.tabs.insert(zone);
        }
    }

    /// Narrows the list to cards whose name contains `text`.
    ///
    /// Control characters are dropped here for the same reason
    /// [`Self::push_filter`] drops them, and this is the path that needs it
    /// more: a keystroke is one character a player meant, while this is where
    /// a whole value arrives from autofill or a paste — which is exactly
    /// where a stray newline or tab comes from.
    pub fn set_filter(&mut self, text: impl Into<String>) {
        let mut text: String = text.into();
        text.retain(|c| !c.is_control());
        let end = text.len();
        self.filter.set(&text, end, None);
    }

    /// The same, with the caret and selection the platform reports beside the
    /// value — `Lobby::set_field_at`'s shape and for its reason: an `<input>`
    /// owns both, and this writes even when the text has not changed, because
    /// moving the caret inside unchanged text is exactly what an arrow key in
    /// one does.
    ///
    /// Offsets are byte offsets into `text`, which is what a `SoftKey` is
    /// documented to carry. Dropping a control character would move every
    /// offset after it, so a value that had one is taken with the caret at the
    /// end rather than with offsets that no longer point where the platform
    /// meant.
    pub fn set_filter_state(&mut self, text: &str, cursor: usize, anchor: Option<usize>) {
        let mut clean: String = text.to_string();
        clean.retain(|c| !c.is_control());
        if clean.len() == text.len() {
            self.filter.set(&clean, cursor, anchor);
        } else {
            let end = clean.len();
            self.filter.set(&clean, end, None);
        }
    }

    /// Empties the box, and asks for the platform's own field to be re-seeded.
    ///
    /// A field the *browser* owns holds its own copy of the text, so emptying
    /// ours behind its back would leave the old letters on screen and put
    /// them straight back on the next keystroke. Bumping the epoch is how
    /// `browser_softkeys` is told to point the `<input>` at the new value;
    /// where the client does its own typing nothing reads it and the bump
    /// costs nothing.
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        if self.typing {
            self.typing_epoch += 1;
        }
    }

    /// Whether the filter box has the keyboard.
    ///
    /// Normally it is handed over as the panel opens and the player takes it
    /// back with `Esc` or `Enter`, after which the letters belong to the game
    /// again and the sheet can stand open through a turn. It was the other
    /// way round once — the box took the keyboard only on a click — and the
    /// price was paid in a single sitting: a search term typed into an open
    /// panel was fifteen bound letters fired at the table, one of which
    /// latched the text view on and persisted it. `K`/`B` and `Y`/`N` reach
    /// the *engine*, and there is no undo.
    ///
    /// **This half does not decide that**, and a reader who takes the
    /// paragraph above as the whole rule will be wrong on two platforms.
    /// [`Self::start_typing`] is called by the shell —
    /// `input::browser_takes_the_keyboard` in `baylee-client` — which refuses
    /// on two counts a model cannot see: a platform that owns its own typing
    /// (a phone's `<input>` is raised by a tap and by nothing else, so
    /// seizing the keyboard here would draw a focused box nobody can type
    /// into), and an ordering, which draws no filter box at all. An open
    /// panel with this `false` is therefore not a defect by itself; it is one
    /// of those two, or the player pressed `Esc`.
    #[must_use]
    pub const fn is_typing(&self) -> bool {
        self.typing
    }

    /// How many times the box has been focused.
    ///
    /// A platform with its own text input (a browser's `<input>`, and the
    /// only thing that raises a phone's keyboard) has to be *pointed* at a
    /// field, and it needs an edge rather than a level to do it on.
    #[must_use]
    pub const fn typing_epoch(&self) -> u64 {
        self.typing_epoch
    }

    /// Gives the filter box the keyboard, opening the panel if it was shut.
    ///
    /// **Only** if it was shut: writing `ByHand` over a sheet a question
    /// opened is the theft [`Self::open_at`] refuses for the same reason.
    /// [`Self::answers_here`] reads the opening, so a sheet promoted this way
    /// goes on standing in front of the question with the question's own keys
    /// dead — which is what a click in the filter box did to a search prompt
    /// for as long as the box could be clicked.
    pub fn start_typing(&mut self) {
        if self.open == Opening::Shut {
            self.open = Opening::ByHand;
        }
        if !self.typing {
            self.typing = true;
            self.typing_epoch += 1;
        }
    }

    /// Takes the keyboard back. The text stays.
    pub fn stop_typing(&mut self) {
        self.typing = false;
    }

    /// One typed character, at the caret and over the selection.
    pub fn push_filter(&mut self, c: char) {
        if !c.is_control() {
            self.filter.insert(c.encode_utf8(&mut [0u8; 4]));
        }
    }

    /// A whole run at once — a paste, or what an IME commits.
    pub fn type_text(&mut self, text: &str) {
        let mut clean: String = text.to_string();
        clean.retain(|c| !c.is_control());
        if !clean.is_empty() {
            self.filter.insert(&clean);
        }
    }

    /// Rubs out what is selected, or the character before the caret, and says
    /// whether there was anything to rub out.
    pub fn pop_filter(&mut self) -> bool {
        let before = self.filter.text().len();
        self.filter.delete_back();
        self.filter.text().len() != before
    }

    /// The same forwards — Delete.
    pub fn delete_forward(&mut self) {
        self.filter.delete_forward();
    }

    /// Moves the caret, extending the selection when `select`.
    pub fn move_filter_caret(
        &mut self,
        step: crate::textbuf::Step,
        dir: crate::textbuf::Dir,
        select: bool,
    ) {
        self.filter.move_caret(step, dir, select);
    }

    /// Selects the whole box — ⌘A.
    pub fn select_all_filter(&mut self) {
        self.filter.select_all();
    }

    /// Puts the caret at a byte offset, with `anchor` the other end of a
    /// selection — what a click or a drag in the box asks for.
    pub fn place_filter_caret(&mut self, cursor: usize, anchor: Option<usize>) {
        self.filter.place(cursor, anchor);
    }

    /// What the rows are sorted by.
    #[must_use]
    pub const fn sort(&self) -> SortKey {
        self.sort
    }

    /// Whether the sort runs backwards.
    #[must_use]
    pub const fn descending(&self) -> bool {
        self.descending
    }

    /// Sets the key, leaving the direction alone.
    pub const fn sort_by(&mut self, key: SortKey) {
        self.sort = key;
    }

    /// One button's worth: the next key, and back to ascending with it.
    ///
    /// The direction resets because a key and a direction are one control
    /// here, and carrying "descending" from mana value into name would answer
    /// a question the player did not ask again.
    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.descending = false;
    }

    /// Turns the current sort round.
    pub const fn reverse(&mut self) {
        self.descending = !self.descending;
    }

    /// Reacts to the choice changing.
    ///
    /// Called at the one point the interaction is replaced, never per frame:
    /// a panel that re-decided every frame whether to be open could not be
    /// closed. A choice that wants the tray opens it; a choice that does not
    /// leaves it exactly as the player left it, and clears the tab so the
    /// next question is not answered through last question's filter.
    ///
    /// The one exception to "leaves it alone" is a sheet this method opened
    /// itself: the question it was opened for has been answered, so it shuts
    /// again. That is the whole of the fetchland's round trip — the search
    /// puts the library on screen, the pick sends, the next question wants
    /// nothing from the sheet and the sheet gets out of the way.
    pub fn follow(&mut self, view: &PlayerView, interaction: Option<&Interaction>) {
        // A pile the player ticked can empty — the graveyard they were
        // merging gets exiled whole — and a tick on a zone that no longer
        // exists is a tick nothing draws and nothing can take back. Dropping
        // it here rather than in `rows` keeps one answer to "what is ticked":
        // a set that listed a zone the panel has no chip for would be a state
        // the player could see the effect of and not the cause.
        let alive: BTreeSet<BrowseZone> = zones_of(view).into_iter().collect();
        self.tabs.retain(|zone| alive.contains(zone));
        if let Some(it) = interaction.filter(|it| Self::wanted(view, it)) {
            self.open = Opening::ForChoice;
            // Which tab the question itself asks for, before "every zone at
            // once", which is the answer for a question that spans them. It
            // also settles an ordering this used to lose: a reveal arrives as
            // a *view* and `saw_reveal` pins `Looking` on it, then the choice
            // arrives and this ran a frame later and wrote that pin away.
            self.locked = Self::sole_zone(view, it);
            self.tabs = self.locked.into_iter().collect();
            self.filter.clear();
        } else if self.open == Opening::ForChoice {
            self.close();
        } else {
            self.locked = None;
        }
    }

    /// The one zone every card this question offers is in, when there is one.
    ///
    /// `None` the moment the offer reaches two zones — or reaches the table
    /// or the hand, which the sheet does not draw at all: a question that can
    /// be answered by clicking a permanent must not pin the sheet to the
    /// pile it *also* offers, because the tabs would then be claiming the
    /// permanent is not an answer.
    fn sole_zone(view: &PlayerView, interaction: &Interaction) -> Option<BrowseZone> {
        let mut only = None;
        for id in interaction.selectable() {
            let found = zones_of(view).into_iter().find(|zone| {
                objects_in(view, *zone)
                    .iter()
                    .any(|object| object.id == *id)
            })?;
            match only {
                None => only = Some(found),
                Some(zone) if zone == found => {}
                Some(_) => return None,
            }
        }
        only
    }

    /// Reacts to a *view* arriving, which is a different event.
    ///
    /// [`Self::follow`] answers a question being asked; this answers cards
    /// being **shown**. `looking_at` can fill up with no choice attached at
    /// all — a reveal, the top of a library turned over — and those cards
    /// live in no zone the seat can otherwise see, so the sheet is the only
    /// surface in the client that draws them. Until the pile chips were
    /// removed there was a button standing above the board that would open
    /// it; now the reveal opens it itself.
    ///
    /// Edge-triggered on the ids, for the reason `follow` gives about
    /// per-frame decisions: a panel re-deciding every frame whether to be
    /// open could not be closed. So it opens on the frame a reveal *becomes*
    /// something else and leaves the player alone afterwards — and a reveal
    /// that ends and another that begins is two openings, which a length
    /// comparison would have merged into none.
    pub fn saw_reveal(&mut self, view: &PlayerView) {
        // Sorted, because *which* cards is the trigger and their order is
        // not: a scry re-sends the same cards rearranged, and a sheet the
        // player had closed would reopen itself under their hand halfway
        // through putting them in order.
        let mut now: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
        now.sort_unstable();
        if now != self.looking_seen {
            if now.is_empty() {
                // The reveal is over. A sheet that opened itself for it has
                // nothing left to draw — `BrowseZone::Looking` is not even a
                // tab any more — so it shuts, on the same edge it opened on.
                if self.open == Opening::ForChoice {
                    self.close();
                }
            } else {
                self.open = Opening::ForChoice;
                // Alone, and not added to whatever the player had ticked: the
                // cards being shown are the event, and a reveal landing inside
                // a merge of two graveyards would be a reveal nobody could
                // find.
                self.tabs = std::iter::once(BrowseZone::Looking).collect();
            }
            self.looking_seen = now;
        }
    }

    /// Whether this choice needs the tray at all.
    ///
    /// True when the engine offered an object that is neither on the
    /// battlefield nor in this seat's hand — the two places a client can
    /// already click. An ordering always wants it: the places are numbers
    /// beside the cards, and the table has nowhere to draw a number that
    /// says "third" without lying about the battlefield.
    #[must_use]
    pub fn wanted(view: &PlayerView, interaction: &Interaction) -> bool {
        if !interaction.is_mine() {
            return false;
        }
        if interaction.is_ordering() {
            return true;
        }
        interaction.selectable().iter().any(|id| {
            !view.battlefield.iter().any(|o| o.id == *id) && !view.hand.iter().any(|h| h.id == *id)
        })
    }

    /// Every zone with something in it, in tab order.
    ///
    /// The viewing seat's own piles come before its opponents', because a
    /// player looking for a card is usually looking in their own graveyard.
    #[must_use]
    pub fn zones(&self, view: &PlayerView) -> Vec<BrowseZone> {
        zones_of(view)
    }
}

/// Every zone with something in it, in tab order.
///
/// Free of the panel because [`Browser::sole_zone`] asks it while deciding
/// what the panel's state should *be*: a method reading nothing of `self`
/// that `self` is being built from is a borrow waiting to be a problem.
fn zones_of(view: &PlayerView) -> Vec<BrowseZone> {
    let mut out = Vec::new();
    if !view.looking_at.is_empty() {
        out.push(BrowseZone::Looking);
    }
    if !view.stack.is_empty() {
        out.push(BrowseZone::Stack);
    }
    for seat in seats_from(view) {
        let i = seat.get() as usize;
        if view.graveyards.get(i).is_some_and(|z| !z.is_empty()) {
            out.push(BrowseZone::Graveyard(seat));
        }
        if view.exile.get(i).is_some_and(|z| !z.is_empty()) {
            out.push(BrowseZone::Exile(seat));
        }
        if view.command.get(i).is_some_and(|z| !z.is_empty()) {
            out.push(BrowseZone::Command(seat));
        }
    }
    out
}

impl Browser {
    /// The rows to draw, in tab order and then in each zone's own order.
    ///
    /// Pass `None` for the interaction to browse with no question pending —
    /// what tapping a pile does. Nothing is selectable then, which is
    /// the honest answer: there is nothing to select *for*.
    ///
    /// `names` is the seam [`Names`] documents; [`Names::projected`] is what
    /// a caller with no card text brings.
    #[must_use]
    pub fn rows(
        &self,
        view: &PlayerView,
        interaction: Option<&Interaction>,
        names: Names<'_>,
    ) -> Vec<BrowseRow> {
        let mine = interaction.filter(|it| it.is_mine());
        let ordering = mine.is_some_and(Interaction::is_ordering);
        // Folded, not merely lowercased: `strasse` has to find `Straße`, and
        // a player whose keyboard has no `ß` types the first of those. See
        // [`crate::prose::sort_key`].
        let needle = crate::prose::sort_key(self.filter.text().trim());
        let mut out = Vec::new();
        for zone in self.zones(view) {
            if !self.shows(zone) {
                continue;
            }
            for object in objects_in(view, zone) {
                let shown = (names.shown)(object).unwrap_or_else(|| object.name.clone());
                // Both names, because they are two ways of naming the same
                // card and a player knows the one they learned it under. The
                // projection is also the only name a *token* has.
                if !needle.is_empty()
                    && !crate::prose::sort_key(&shown).contains(&needle)
                    && !crate::prose::sort_key(&object.name).contains(&needle)
                {
                    continue;
                }
                // Membership of the offered list, not `is_selectable`: a
                // choice whose options the engine leaves implicit (a
                // discard) accepts anything, and asking that question here
                // would light up every graveyard card in the game as a
                // legal discard.
                let selectable = mine.is_some_and(|it| it.selectable().contains(&object.id));
                out.push(BrowseRow {
                    id: object.id,
                    name: shown,
                    // Same fallback as the board's: a token has no printing
                    // but does have a picture, and the row beside its badge
                    // is the one place a player reads a token as a card.
                    art: object
                        .card
                        .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small))
                        .or_else(|| object.token.map(|t| ImageKey::token(t, ArtSize::Small))),
                    zone,
                    standing: RowStanding {
                        selectable,
                        selected: mine.is_some_and(|it| it.is_selected(object.id)),
                        focused: mine
                            .and_then(crate::interaction::Interaction::aim)
                            .is_some_and(|pick| {
                                pick == crate::interaction::Pick::Object(object.id)
                            }),
                    },
                    place: ordering
                        .then(|| mine.and_then(|it| it.selected().position(|o| o == object.id)))
                        .flatten()
                        .map(|p| p + 1),
                    mana_value: object.mana_value,
                    types: object.types,
                    token: object.token.is_some(),
                });
            }
        }
        self.arrange(&mut out);
        out
    }

    /// Puts the rows in the order the sort control asks for.
    ///
    /// Zone always wins, whatever the key: the tabs are the panel's first
    /// structure, and a list that interleaved a graveyard with an exile
    /// because both hold a two-drop would have thrown that away. Within a
    /// zone, the key decides, and every key falls back to the zone's own
    /// order — so the sort is total and two runs of it agree, which a sort on
    /// a `Vec` of equal keys does not otherwise guarantee.
    fn arrange(&self, rows: &mut [BrowseRow]) {
        if self.sort == SortKey::Place && !self.descending {
            return;
        }
        // The zone's own order, captured before anything moves: `sort_by` is
        // stable, but the descending pass reverses within a key and would
        // otherwise turn "the order they arrived in" upside down as a side
        // effect of asking for Z–A.
        let places: HashMap<ObjectId, usize> = rows
            .iter()
            .enumerate()
            .map(|(at, row)| (row.id, at))
            .collect();
        let place = |row: &BrowseRow| places.get(&row.id).copied().unwrap_or(0);
        // Folded once per row and not once per comparison, which is the same
        // reason `places` is a map: a sort asks its key n log n times.
        let keys: HashMap<ObjectId, String> = if self.sort == SortKey::Name {
            rows.iter()
                .map(|row| (row.id, crate::prose::sort_key(&row.name)))
                .collect()
        } else {
            HashMap::new()
        };
        let key = |row: &BrowseRow| keys.get(&row.id).map_or("", String::as_str);
        rows.sort_by(|a, b| {
            let zone = a.zone.cmp(&b.zone);
            if zone != std::cmp::Ordering::Equal {
                return zone;
            }
            let within = match self.sort {
                // The place *is* the key here, not the tie-break, or asking
                // for the pile upside down would compare equal and do nothing.
                SortKey::Place => place(a).cmp(&place(b)),
                SortKey::Name => key(a).cmp(key(b)),
                SortKey::ManaValue => a.mana_value.cmp(&b.mana_value),
                SortKey::Type => type_rank(a.types).cmp(&type_rank(b.types)),
            };
            let within = if self.descending {
                within.reverse()
            } else {
                within
            };
            within.then_with(|| place(a).cmp(&place(b)))
        });
    }
}

/// Seats in browsing order: the viewing seat, then the rest in seat order.
fn seats_from(view: &PlayerView) -> Vec<PlayerId> {
    let n = view.seats.len();
    let me = view.seat.get() as usize;
    (0..n)
        .map(|i| {
            let seat = (me + i) % n;
            PlayerId::new(u8::try_from(seat).unwrap_or(0))
        })
        .collect()
}

/// The objects one zone holds, in the order the view lists them.
///
/// A seat index out of range is a malformed view rather than an empty zone,
/// but a client must not panic on one — so it reads as empty.
fn objects_in(view: &PlayerView, zone: BrowseZone) -> &[baylee_view::PublicObject] {
    match zone {
        BrowseZone::Looking => &view.looking_at,
        BrowseZone::Stack => &view.stack,
        BrowseZone::Graveyard(p) => pile(&view.graveyards, p),
        BrowseZone::Exile(p) => pile(&view.exile, p),
        BrowseZone::Command(p) => pile(&view.command, p),
    }
}

/// One seat's pile out of a per-seat zone list.
fn pile(zones: &[Vec<baylee_view::PublicObject>], seat: PlayerId) -> &[baylee_view::PublicObject] {
    zones.get(seat.get() as usize).map_or(&[], Vec::as_slice)
}

#[cfg(test)]
mod tests;
