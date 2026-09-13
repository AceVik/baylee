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
use std::collections::HashMap;

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
    pub const MIN_W: f32 = 346.0;
    /// The head, the footer, and two whole rows between them.
    pub const MIN_H: f32 = 286.0;
    /// One row wide — what `TRAY_PANEL_W` in the renderer computes.
    ///
    /// It was 854, ten columns of a card grid, because the sheet used to draw
    /// cards and a search wanted forty of them at once. The dialog is a list
    /// now (`docs/redesign-proposal.md` §6), and a list is read down rather
    /// than across: past a comfortable measure, width buys a longer blank
    /// stretch in the middle of every row and nothing else. Height is what
    /// buys rows, and it is the axis that grew.
    pub const DEFAULT_W: f32 = 661.0;
    /// The chrome, and eight rows and a **half**.
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
    pub const DEFAULT_H: f32 = 649.0;
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
    tab: Option<BrowseZone>,
    /// The tab the *question* pins, when every card it offers is in one zone.
    ///
    /// Beside `tab` rather than inside it because the two answer different
    /// questions: `tab` is what is showing, `locked` is whether the player
    /// may change it. A search puts seven library cards on the sheet and the
    /// graveyard beside them has nothing to do with the question — so the
    /// other tabs are drawn and are not buttons, which is the owner's
    /// "ausgrauen" said in state.
    locked: Option<BrowseZone>,
    filter: String,
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

    /// The zone tab in force, or `None` for "every zone at once".
    #[must_use]
    pub fn tab(&self) -> Option<BrowseZone> {
        self.tab
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
    /// writes `tab` without asking the lock, and it also turns a `ForChoice`
    /// opening into a by-hand one — which takes the sheet away from the
    /// question it was opened for, so `answers_here` goes false and the
    /// question's own keys stop working on a dialog that is still on the
    /// screen. A tap that lands on a pile while a search is standing open is
    /// not a request to abandon the search.
    pub fn open_at(&mut self, zone: BrowseZone) {
        if self.open == Opening::ForChoice {
            return;
        }
        self.open = Opening::ByHand;
        self.tab = Some(zone);
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

    /// Shows one zone, or every zone when given `None`.
    ///
    /// Refused while a question has pinned a tab: the rows in every other
    /// zone answer nothing, so the model says no rather than relying on the
    /// renderer to have drawn no button.
    pub fn show(&mut self, tab: Option<BrowseZone>) {
        if self.locked.is_none() {
            self.tab = tab;
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
        self.filter = text.into();
        self.filter.retain(|c| !c.is_control());
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
    /// A box that took every keystroke while the panel merely stood open
    /// would be the end of playing with the graveyard visible, so this is a
    /// focus a player gives it and takes back — the same bargain the lobby's
    /// fields make.
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
    pub fn start_typing(&mut self) {
        self.open = Opening::ByHand;
        if !self.typing {
            self.typing = true;
            self.typing_epoch += 1;
        }
    }

    /// Takes the keyboard back. The text stays.
    pub fn stop_typing(&mut self) {
        self.typing = false;
    }

    /// One typed character.
    pub fn push_filter(&mut self, c: char) {
        if !c.is_control() {
            self.filter.push(c);
        }
    }

    /// Rubs one character out, and says whether there was one.
    pub fn pop_filter(&mut self) -> bool {
        self.filter.pop().is_some()
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
        if let Some(it) = interaction.filter(|it| Self::wanted(view, it)) {
            self.open = Opening::ForChoice;
            // Which tab the question itself asks for, before "every zone at
            // once", which is the answer for a question that spans them. It
            // also settles an ordering this used to lose: a reveal arrives as
            // a *view* and `saw_reveal` pins `Looking` on it, then the choice
            // arrives and this ran a frame later and wrote that pin away.
            self.locked = Self::sole_zone(view, it);
            self.tab = self.locked;
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
                self.tab = Some(BrowseZone::Looking);
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
        let needle = crate::prose::sort_key(self.filter.trim());
        let mut out = Vec::new();
        for zone in self.zones(view) {
            if self.tab.is_some_and(|t| t != zone) {
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
mod tests {
    use super::*;
    // The registry here is the empty one, and deliberately: a zone browser
    // lists cards in hidden zones, and a card that arrives in one is a new
    // object with no memory of its previous existence (CR 400.7), so whatever
    // it was copying on the battlefield it is not copying in a graveyard.
    // Nothing a browser draws is ever wearing another card's face.
    use crate::board::{BoardModel, Openings, Registry};
    use crate::test_support::{ViewBuilder, printed};
    use baylee_engine::choice::{ChoicePrompt, Pending, TargetPrompt};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// Everything a client can already click without the browser: the
    /// battlefield as drawn cards, and the seat's own hand.
    fn drawn_on_the_table(view: &PlayerView) -> Vec<ObjectId> {
        let board = BoardModel::from_view(view, Openings::none(), |_| 100.0, Registry::none());
        let mut ids: Vec<ObjectId> = board
            .pods
            .iter()
            .flat_map(|p| p.lanes.iter())
            .flat_map(|l| l.groups.iter())
            .flat_map(|g| g.members.iter().copied())
            .collect();
        ids.extend(board.hand.iter().map(|c| c.id));
        ids
    }

    #[test]
    fn a_library_search_is_shown_where_the_board_cannot_show_it() {
        // Four cards the engine is *showing* the seat. They are in nobody's
        // graveyard and on no battlefield, so before the browser existed the
        // only thing on screen was the prompt.
        let shown: Vec<_> = (10..14).map(|s| printed(s, 0, "Forest", 1)).collect();
        let view = ViewBuilder::new(2).with_looking_at(shown).build();
        let it = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: (10..14).map(obj).collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );

        assert!(Browser::wanted(&view, &it), "nothing else can draw these");
        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        assert!(b.is_open());

        let rows = b.rows(&view, Some(&it), Names::projected());
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|r| r.zone == BrowseZone::Looking));
        assert!(
            rows.iter().all(|r| r.standing.selectable),
            "all four were offered"
        );
        assert!(rows.iter().all(|r| r.art.is_some()), "each has a picture");
    }

    /// W1, as the owner measured it: the sheet sat 164 pixels left of the
    /// middle of a 2056-pixel window, at exactly the place it would be
    /// centred in a 1728-pixel one.
    ///
    /// Nobody had dragged it there — it was remembered from a smaller screen,
    /// and `fit` has no opinion about the middle of a band. So a question's
    /// sheet reads no store at all, and the two window widths have to answer
    /// the same word: centred.
    #[test]
    fn a_sheet_a_question_opened_is_centred_on_whatever_window_it_meets() {
        let shown: Vec<_> = (10..14).map(|s| printed(s, 0, "Forest", 1)).collect();
        let view = ViewBuilder::new(2).with_looking_at(shown).build();
        let it = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: (10..14).map(obj).collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            },
            me(),
        );

        // What the store held: the sheet centred on the smaller window.
        //
        // That is how the fault was found. The owner measured the dialog at
        // `x ≈ 437..1290` on a 2056-pixel window, and 437 is exactly
        // `(1728 - 854) / 2` — the sheet's width at the time, centred in a
        // band 1728 wide. So it had been centred, on a *different* screen,
        // and remembered; `fit` clamps a rectangle inside a band and has no
        // opinion about the middle of one.
        let small = (1728.0, 776.0);
        let remembered = Placement::centred(small);
        assert!(
            (remembered.left - (small.0 - Placement::DEFAULT_W) / 2.0).abs() < 1.0,
            "the arithmetic that found the fault"
        );

        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        assert!(b.for_choice(), "a question is what opened it");

        let middle = |place: Placement| place.left + place.width / 2.0;
        for band in [small, (2056.0, 776.0)] {
            let place = b.placement(band, Some(remembered));
            assert!(
                (middle(place) - band.0 / 2.0).abs() < 1.0,
                "a question's sheet is centred in {band:?}, not where a smaller window left it"
            );
        }

        // The other half of the same rule: a sheet the *player* opened is
        // where they put it, because that is what dragging it means.
        let mut by_hand = Browser::new();
        by_hand.open_at(BrowseZone::Looking);
        let held = by_hand.placement((2056.0, 776.0), Some(remembered));
        assert!(
            (held.left - remembered.left).abs() < 1.0,
            "a dragged sheet stays dragged"
        );
    }

    /// W3: a search is about the cards being shown and about nothing else,
    /// so the tabs beside them are not part of the question.
    #[test]
    fn a_question_that_lives_in_one_zone_pins_the_tab_to_it() {
        let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
        let buried: Vec<_> = (20..22).map(|s| printed(s, 0, "Mountain", 1)).collect();
        let view = ViewBuilder::new(2)
            .with_looking_at(shown)
            .with_graveyard(0, buried)
            .build();
        let search = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: (10..13).map(obj).collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            },
            me(),
        );

        let mut b = Browser::new();
        b.follow(&view, Some(&search));
        assert_eq!(b.locked(), Some(BrowseZone::Looking));
        assert_eq!(b.tab(), Some(BrowseZone::Looking));
        assert!(
            b.rows(&view, Some(&search), Names::projected())
                .iter()
                .all(|r| r.zone == BrowseZone::Looking),
            "the graveyard has nothing to answer here"
        );

        // The pin is the model's, not the renderer's: a click that reached
        // "every zone" anyway changes nothing.
        b.show(None);
        assert_eq!(b.tab(), Some(BrowseZone::Looking), "the pin holds");

        // A question that reaches two zones pins nothing — there is no one
        // tab that could answer it.
        let across = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: [obj(10), obj(20)].into(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );
        b.follow(&view, Some(&across));
        assert_eq!(b.locked(), None);
        assert_eq!(b.tab(), None, "every zone at once");

        // And the pin belongs to the question: answered, the sheet the player
        // opens next is theirs to steer again.
        b.follow(&view, Some(&search));
        assert_eq!(b.locked(), Some(BrowseZone::Looking));
        b.close();
        assert_eq!(b.locked(), None);
        b.open();
        b.show(Some(BrowseZone::Graveyard(me())));
        assert_eq!(b.tab(), Some(BrowseZone::Graveyard(me())));
    }

    /// W2: the dim says "there is nothing else to do", so it is drawn only
    /// when that is true — which is a narrower thing than "a question opened
    /// this sheet".
    #[test]
    fn the_table_goes_dark_only_when_every_answer_is_in_the_sheet() {
        let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
        let view = ViewBuilder::new(2)
            .with_looking_at(shown)
            .with_hand(vec![("Ornithopter", 0, 30)])
            .build();
        let mut b = Browser::new();
        assert!(!b.dims_the_table(), "a shut sheet darkens nothing");

        // A search: every card it offers is in the one pile it put on screen.
        let search = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: (10..13).map(obj).collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            },
            me(),
        );
        b.follow(&view, Some(&search));
        assert!(b.dims_the_table(), "nothing outside the sheet is an answer");

        // A question that also offers a card in hand. The sheet still opens —
        // the revealed cards are nowhere else — but the hand is an answer,
        // and a veil over it would be darkening the thing to click.
        let spanning = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: [obj(10), obj(30)].into(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );
        b.follow(&view, Some(&spanning));
        assert!(b.for_choice(), "a question is still what opened it");
        assert!(
            !b.dims_the_table(),
            "the hand holds an answer and must stay lit"
        );

        // And a pile the player opened to read stands over a live game.
        let mut by_hand = Browser::new();
        by_hand.open_at(BrowseZone::Graveyard(me()));
        assert!(!by_hand.dims_the_table(), "the game goes on underneath");
    }

    /// One question, one Confirm. The dialog's footer is where the answer is
    /// sent from, and the prompt slip reads this same predicate to keep out of
    /// its way.
    #[test]
    fn only_the_sheet_the_question_opened_draws_its_footer() {
        let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
        let view = ViewBuilder::new(2)
            .with_looking_at(shown)
            .with_hand(vec![("Ornithopter", 0, 30)])
            .with_battlefield(0, [printed(40, 0, "Grizzly Bears", 2)])
            .with_graveyard(0, vec![printed(50, 0, "Lightning Bolt", 3)])
            .build();

        let search = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: (10..13).map(obj).collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            },
            me(),
        );
        let mut b = Browser::new();
        assert!(
            !b.answers_here(Some(&search)),
            "a shut sheet answers nothing"
        );
        b.follow(&view, Some(&search));
        assert!(b.answers_here(Some(&search)), "this is the question's home");
        assert!(
            !b.answers_here(None),
            "and a sheet with no question left in it draws no footer either"
        );

        // A choice that spans the sheet and the hand: the table stays lit, and
        // the send still happens here, because there is nowhere else for it.
        let spanning = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: [obj(10), obj(30)].into(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );
        b.follow(&view, Some(&spanning));
        assert!(!b.dims_the_table(), "the hand is an answer");
        assert!(
            b.answers_here(Some(&spanning)),
            "and this is still the send"
        );

        // The case the footer used to get wrong: a question about the
        // battlefield, and a graveyard the player opened to read while they
        // think about it. Nothing in that pile is an answer.
        let on_the_table = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: [obj(40)].into(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );
        let mut by_hand = Browser::new();
        by_hand.open_at(BrowseZone::Graveyard(me()));
        by_hand.follow(&view, Some(&on_the_table));
        assert!(by_hand.is_open(), "the pile the player opened stays open");
        assert!(
            !by_hand.answers_here(Some(&on_the_table)),
            "the table is where that question is answered"
        );
    }

    /// The sheet is put where it fits, and shrunk before it is moved.
    #[test]
    fn a_sheet_is_brought_inside_the_band_it_stands_in() {
        let band = (1728.0, 776.0);
        let home = Placement::centred(band);
        assert!((home.width - Placement::DEFAULT_W).abs() < f32::EPSILON);
        assert!(
            (home.left - (band.0 - home.width) / 2.0).abs() < 0.01,
            "a sheet nobody has moved stands in the middle"
        );

        // A remembered position from a wider screen comes home rather than
        // hanging off the edge, taking its resize handle with it.
        let strayed = Placement {
            left: 3000.0,
            top: 2000.0,
            ..home
        };
        let back = strayed.fit(band);
        assert!(back.left + back.width <= band.0, "off the right edge");
        assert!(back.top + back.height <= band.1, "off the bottom edge");

        // Size before position: a sheet wider than the band is narrowed
        // first, so its own width cannot then push it off the far side.
        let huge = Placement {
            left: 900.0,
            top: 600.0,
            width: 5000.0,
            height: 5000.0,
        }
        .fit(band);
        assert!(huge.width < band.0 && huge.height < band.1);
        assert!(huge.left + huge.width <= band.0);
        assert!(huge.top + huge.height <= band.1);

        // And the minimum wins over a band with no room for it: a sheet
        // clamped to nothing is a sheet that is not there.
        let cramped = Placement::centred((120.0, 90.0));
        assert!((cramped.width - Placement::MIN_W).abs() < f32::EPSILON);
        assert!((cramped.height - Placement::MIN_H).abs() < f32::EPSILON);
    }

    /// Dragging moves it, the corner resizes it, and neither can leave the
    /// band — which is what keeps the resize handle reachable.
    #[test]
    fn a_sheet_cannot_be_dragged_or_stretched_out_of_reach() {
        let band = (1728.0, 776.0);
        let home = Placement::centred(band);

        let nudged = home.moved_by((40.0, -25.0), band);
        assert!((nudged.left - (home.left + 40.0)).abs() < 0.01);
        assert!((nudged.top - (home.top - 25.0)).abs() < 0.01);
        assert!(
            (nudged.width - home.width).abs() < f32::EPSILON,
            "a drag is not a resize"
        );

        let shoved = home.moved_by((-9999.0, -9999.0), band);
        assert!(shoved.left >= 0.0 && shoved.top >= 0.0);

        let stretched = home.resized_by((60.0, 40.0), band);
        assert!((stretched.width - (home.width + 60.0)).abs() < 0.01);
        assert!(
            (stretched.left - home.left).abs() < f32::EPSILON,
            "the corner drags the corner, not the sheet"
        );

        let squashed = home.resized_by((-9999.0, -9999.0), band);
        assert!((squashed.width - Placement::MIN_W).abs() < f32::EPSILON);
        assert!((squashed.height - Placement::MIN_H).abs() < f32::EPSILON);
    }

    /// A reveal with no question attached opens the sheet by itself.
    ///
    /// This is the job the "Zones" chip used to do and the reason the chip
    /// could not simply be deleted: cards in `looking_at` are shown to a seat
    /// without anything being asked of them, they are drawn on no other
    /// surface in the client, and `follow` only ever runs when a *choice*
    /// arrives. Edge-triggered on the ids, so the player can put it away.
    /// A library has no tab, so a tap on one has nowhere to go.
    ///
    /// The second of the two readings that enforce CR 401.2 — `ZonePile::
    /// is_browsable` is the other — and the one that would silently start
    /// working if a `Looking`-shaped variant were ever added for libraries.
    #[test]
    fn a_library_has_no_tab_to_open() {
        use crate::layout::PileKind;

        let seat = PlayerId::new(0);
        assert_eq!(BrowseZone::of_pile(PileKind::Library, seat), None);
        assert_eq!(
            BrowseZone::of_pile(PileKind::Graveyard, seat),
            Some(BrowseZone::Graveyard(seat))
        );
        assert_eq!(
            BrowseZone::of_pile(PileKind::Exile, seat),
            Some(BrowseZone::Exile(seat))
        );
        // Both command piles are one zone (CR 408.1): a seat with two
        // commanders has two places on the mat and one tab.
        assert_eq!(
            BrowseZone::of_pile(PileKind::Command2, seat),
            BrowseZone::of_pile(PileKind::Command, seat)
        );
    }

    #[test]
    fn cards_shown_to_a_seat_open_the_sheet_by_themselves() {
        let mut b = Browser::new();
        let nothing = ViewBuilder::new(2).build();
        b.saw_reveal(&nothing);
        assert!(!b.is_open(), "an empty reveal is not a reveal");

        let shown = ViewBuilder::new(2)
            .with_looking_at(vec![printed(10, 0, "Ponder", 1)])
            .build();
        b.saw_reveal(&shown);
        assert!(
            b.is_open(),
            "cards being shown open the sheet that draws them"
        );
        assert_eq!(b.tab(), Some(BrowseZone::Looking));

        // …and it stays closed once the player closes it, however many views
        // arrive carrying the same cards. A per-frame decision would make the
        // panel impossible to dismiss.
        b.close();
        for _ in 0..5 {
            b.saw_reveal(&shown);
            assert!(!b.is_open(), "the same reveal re-opened it");
        }

        // A reveal that ends and another that begins is two reveals, and the
        // second one opens it again — which a length comparison would miss,
        // because both are one card.
        let other = ViewBuilder::new(2)
            .with_looking_at(vec![printed(11, 0, "Brainstorm", 2)])
            .build();
        b.saw_reveal(&other);
        assert!(b.is_open(), "a different reveal is a new one");

        // But the *same* cards in a different order are the same reveal. A
        // scry is exactly that — every rearrangement comes back as a view —
        // and a sheet the player had closed must not reappear on each one.
        let top = printed(20, 0, "Island", 3);
        let under = printed(21, 0, "Opt", 4);
        let ordered = ViewBuilder::new(2)
            .with_looking_at(vec![top.clone(), under.clone()])
            .build();
        let swapped = ViewBuilder::new(2)
            .with_looking_at(vec![under, top])
            .build();
        b.saw_reveal(&ordered);
        b.close();
        b.saw_reveal(&swapped);
        assert!(!b.is_open(), "reordering the same cards reopened the sheet");
    }

    /// The invariant the whole module exists for: an id the engine offered
    /// is an id somebody draws. `BoardModel` covers the table and the hand,
    /// `Browser` covers everything else, and the two are disjoint by
    /// construction — which is why [`BrowseZone`] has no `Battlefield`.
    #[test]
    fn every_offered_object_is_drawn_somewhere() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .with_hand(vec![("Lightning Bolt", 1, 2)])
            .with_stack(vec![printed(3, 1, "Counterspell", 2)])
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .with_exile(1, vec![printed(5, 1, "Path to Exile", 4)])
            .with_command(0, vec![printed(6, 0, "Sisay", 5)])
            .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
            .build();

        let choices = [
            Pending::ChooseTargets {
                player: me(),
                options: vec![obj(1), obj(3), obj(4), obj(5), obj(6), obj(7)],
                player_options: Vec::new(),
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            Pending::ChooseCards {
                player: me(),
                options: vec![obj(4), obj(7)],
                min: 1,
                max: 2,
                prompt: ChoicePrompt::Generic,
            },
            Pending::LegendChoice {
                player: me(),
                options: vec![obj(1)],
            },
            Pending::OrderObjects {
                player: me(),
                objects: vec![obj(7), obj(4)],
            },
        ];

        let table = drawn_on_the_table(&view);
        for pending in choices {
            let it = Interaction::new(pending.clone(), me());
            let mut b = Browser::new();
            b.follow(&view, Some(&it));
            let rows = b.rows(&view, Some(&it), Names::projected());
            for id in it.selectable() {
                let on_table = table.contains(id);
                let in_tray = rows.iter().any(|r| r.id == *id && r.standing.selectable);
                assert!(
                    on_table || in_tray,
                    "{pending:?} offers {id:?} and nothing draws it"
                );
                assert!(
                    !(on_table && in_tray),
                    "{id:?} is drawn twice — the two models are meant to be disjoint"
                );
            }
        }
    }

    #[test]
    fn a_choice_confined_to_the_table_leaves_the_tray_shut() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .with_hand(vec![("Lightning Bolt", 1, 2)])
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .build();
        let it = Interaction::new(
            Pending::ChooseTargets {
                player: me(),
                options: vec![obj(1), obj(2)],
                player_options: Vec::new(),
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            me(),
        );
        assert!(!Browser::wanted(&view, &it));
        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        assert!(
            !b.is_open(),
            "a target on the board is clicked on the board"
        );
    }

    /// A search that opened the sheet closes it again when it is answered.
    ///
    /// The fetchland's round trip, which is what the owner asked for: the
    /// library goes on screen, a land is picked, and the next question wants
    /// nothing from the sheet — so the sheet gets out of the way instead of
    /// standing over the board until somebody closes it.
    #[test]
    fn a_sheet_opened_for_a_search_shuts_when_the_search_is_answered() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .build();
        let search = Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: vec![obj(4)],
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );
        let mut b = Browser::new();
        b.follow(&view, Some(&search));
        assert!(b.is_open(), "the search did not open it");

        // The answer went in; the engine's next question is about the board.
        let after = Interaction::new(
            Pending::ChooseTargets {
                player: me(),
                options: vec![obj(1)],
                player_options: Vec::new(),
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            me(),
        );
        b.follow(&view, Some(&after));
        assert!(!b.is_open(), "the sheet stayed open with nothing to say");
    }

    /// But a sheet the *player* opened is never closed behind their back.
    #[test]
    fn a_sheet_opened_by_hand_survives_the_next_question() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .build();
        let mut b = Browser::new();
        b.open_at(BrowseZone::Graveyard(me()));
        let it = Interaction::new(
            Pending::ChooseTargets {
                player: me(),
                options: vec![obj(1)],
                player_options: Vec::new(),
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            me(),
        );
        b.follow(&view, Some(&it));
        assert!(b.is_open(), "the player's own sheet was closed for them");
    }

    /// A reveal opens the sheet and the reveal ending closes it.
    #[test]
    fn a_reveal_takes_its_sheet_away_with_it() {
        let shown = ViewBuilder::new(2)
            .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
            .build();
        let done = ViewBuilder::new(2).build();
        let mut b = Browser::new();
        b.saw_reveal(&shown);
        assert!(b.is_open(), "the reveal did not open it");
        b.saw_reveal(&done);
        assert!(!b.is_open(), "the sheet outlived what it was showing");
    }

    /// The maximised sheet is the band filled, and it is a fixed point.
    #[test]
    fn a_maximised_sheet_fills_the_band_and_says_so() {
        let band = (1728.0, 866.0);
        let full = Placement::maximised(band);
        assert!(full.is_maximised(band));
        assert!(full.fit(band).is_maximised(band), "fitting it moved it");
        assert!(!Placement::centred(band).is_maximised(band));
        // And it stays inside: the margin is kept on all four sides.
        assert!(full.left + full.width <= band.0);
        assert!(full.top + full.height <= band.1);
    }

    #[test]
    fn an_ordering_opens_the_tray_and_numbers_each_pick() {
        let view = ViewBuilder::new(2)
            .with_looking_at(vec![
                printed(7, 0, "Ponder", 6),
                printed(8, 0, "Brainstorm", 7),
            ])
            .build();
        let mut it = Interaction::new(
            Pending::OrderObjects {
                player: me(),
                objects: vec![obj(7), obj(8)],
            },
            me(),
        );
        assert!(Browser::wanted(&view, &it), "an ordering always wants it");

        let b = Browser::new();
        assert!(
            b.rows(&view, Some(&it), Names::projected())
                .iter()
                .all(|r| r.place.is_none()),
            "nothing picked yet"
        );
        it.toggle(obj(8));
        it.toggle(obj(7));
        let rows = b.rows(&view, Some(&it), Names::projected());
        let place = |id| rows.iter().find(|r| r.id == id).and_then(|r| r.place);
        assert_eq!(place(obj(8)), Some(1), "picked first, so it goes first");
        assert_eq!(place(obj(7)), Some(2));
    }

    /// A discard leaves its options implicit — the engine means "your hand".
    /// `is_selectable` says yes to anything for those, so a browser that
    /// asked *that* question would offer every graveyard card as a discard.
    #[test]
    fn an_implicit_choice_does_not_light_up_the_whole_table() {
        let view = ViewBuilder::new(2)
            .with_hand(vec![("Lightning Bolt", 1, 2)])
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .build();
        let it = Interaction::new(
            Pending::DiscardChoice {
                player: me(),
                count: 1,
            },
            me(),
        );
        assert!(it.is_selectable(obj(4)), "the interaction accepts anything");
        assert!(
            !Browser::wanted(&view, &it),
            "but the hand is already drawn"
        );
        let b = Browser::new();
        assert!(
            b.rows(&view, Some(&it), Names::projected())
                .iter()
                .all(|r| !r.standing.selectable),
            "a graveyard card is not a legal discard"
        );
    }

    #[test]
    fn a_pile_can_be_read_with_no_question_pending() {
        let view = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
            .build();
        let mut b = Browser::new();
        b.open_at(BrowseZone::Graveyard(PlayerId::new(1)));

        let rows = b.rows(&view, None, Names::projected());
        assert_eq!(rows.len(), 1, "the tab confines it to one pile");
        assert_eq!(rows[0].name, "Birds of Paradise");
        assert!(
            !rows[0].standing.selectable,
            "there is nothing to select for"
        );
        assert!(rows[0].place.is_none());

        b.show(None);
        assert_eq!(
            b.rows(&view, None, Names::projected()).len(),
            2,
            "both piles, unfiltered"
        );
    }

    /// The graveyard's own order is the default and is information: it is
    /// what "the top card of your graveyard" means.
    #[test]
    fn the_default_order_is_the_pile_s_own_and_survives_being_reversed() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![
                    printed(4, 0, "Zealous Persecution", 3),
                    printed(5, 0, "Ancestral Vision", 4),
                    printed(6, 0, "Mox Diamond", 5),
                ],
            )
            .build();
        let mut b = Browser::new();
        assert_eq!(b.sort(), SortKey::Place);
        let names = |b: &Browser| -> Vec<String> {
            b.rows(&view, None, Names::projected())
                .into_iter()
                .map(|r| r.name)
                .collect()
        };
        assert_eq!(
            names(&b),
            [
                "Zealous Persecution".to_string(),
                "Ancestral Vision".to_string(),
                "Mox Diamond".to_string()
            ],
            "the pile was re-ordered with no sort asked for"
        );

        b.reverse();
        assert_eq!(
            names(&b),
            [
                "Mox Diamond".to_string(),
                "Ancestral Vision".to_string(),
                "Zealous Persecution".to_string()
            ],
            "reversing the pile order did not reverse it"
        );
    }

    #[test]
    fn sorting_by_name_and_by_cost_are_both_stable_and_reversible() {
        let mut cheap = printed(4, 0, "Zealous Persecution", 3);
        cheap.mana_value = 2;
        let mut dear = printed(5, 0, "Ancestral Vision", 4);
        dear.mana_value = 9;
        let mut also_cheap = printed(6, 0, "Mox Diamond", 5);
        also_cheap.mana_value = 2;
        let view = ViewBuilder::new(2)
            .with_graveyard(0, vec![cheap, dear, also_cheap])
            .build();
        let mut b = Browser::new();
        let names = |b: &Browser| -> Vec<String> {
            b.rows(&view, None, Names::projected())
                .into_iter()
                .map(|r| r.name)
                .collect()
        };

        b.sort_by(SortKey::Name);
        assert_eq!(
            names(&b),
            [
                "Ancestral Vision".to_string(),
                "Mox Diamond".to_string(),
                "Zealous Persecution".to_string()
            ]
        );

        b.sort_by(SortKey::ManaValue);
        assert_eq!(
            names(&b),
            [
                // Two twos, and the tie is broken by the pile's own order —
                // never by whatever the previous sort happened to leave.
                "Zealous Persecution".to_string(),
                "Mox Diamond".to_string(),
                "Ancestral Vision".to_string()
            ]
        );
        b.reverse();
        assert_eq!(
            names(&b),
            [
                "Ancestral Vision".to_string(),
                "Zealous Persecution".to_string(),
                "Mox Diamond".to_string()
            ],
            "descending reversed the tie-break as well as the key"
        );
    }

    /// Whatever the key, the tabs are the panel's first structure.
    #[test]
    fn a_sort_never_interleaves_two_zones() {
        let view = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(4, 0, "Mox Diamond", 3)])
            .with_exile(0, vec![printed(5, 0, "Ancestral Vision", 4)])
            .build();
        let mut b = Browser::new();
        b.sort_by(SortKey::Name);
        let rows = b.rows(&view, None, Names::projected());
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].zone,
            BrowseZone::Graveyard(PlayerId::new(0)),
            "the exile card sorted ahead of the graveyard it is not in"
        );
        assert_eq!(rows[1].zone, BrowseZone::Exile(PlayerId::new(0)));
    }

    /// Where the keyboard is standing has to reach the row, or the key that
    /// ticks it is ticking something the player cannot pick out of a list.
    ///
    /// One row at a time, and never the chosen one by accident: `focused`
    /// and `selected` are two different claims about the same row — the
    /// client saying where a press would land, and the answer itself.
    #[test]
    fn the_row_the_keyboard_stands_on_says_so() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![
                    printed(4, 0, "Llanowar Elves", 3),
                    printed(5, 0, "Forest", 4),
                ],
            )
            .build();
        let mut it = Interaction::new(
            baylee_engine::choice::Pending::ChooseCards {
                player: PlayerId::new(0),
                options: vec![ObjectId::new(4, 0), ObjectId::new(5, 0)],
                min: 0,
                max: 2,
                prompt: baylee_engine::choice::ChoicePrompt::Generic,
            },
            PlayerId::new(0),
        );
        let b = Browser::new();
        let focus_of = |it: &Interaction| -> Vec<bool> {
            b.rows(&view, Some(it), Names::projected())
                .iter()
                .map(|row| row.standing.focused)
                .collect()
        };
        assert_eq!(focus_of(&it), vec![true, false], "it starts on the first");
        it.cycle_focus(1);
        assert_eq!(focus_of(&it), vec![false, true], "and the walk moves it");
        // Ticking the second leaves the focus exactly where it was: one row
        // is chosen, the same row is focused, and they are still two flags.
        it.toggle_focused();
        let rows = b.rows(&view, Some(&it), Names::projected());
        assert_eq!(
            rows.iter()
                .map(|r| (r.standing.focused, r.standing.selected))
                .collect::<Vec<_>>(),
            vec![(false, false), (true, true)]
        );
    }

    /// A panel with no question in front of it stands on nothing.
    #[test]
    fn a_browse_with_no_question_focuses_no_row() {
        let view = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(4, 0, "Forest", 3)])
            .build();
        let rows = Browser::new().rows(&view, None, Names::projected());
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].standing.focused);
    }

    /// A tap that lands on a pile is not a request to abandon the search.
    ///
    /// `open_at` is the only door that writes `tab` without asking the lock,
    /// and it also turns a `ForChoice` opening into a by-hand one — so a tap
    /// on a graveyard while a library search stood open took the sheet away
    /// from the question, `answers_here` went false, and the question's own
    /// keys stopped working on a dialog that was still on the screen.
    #[test]
    fn a_tap_on_a_pile_does_not_take_the_sheet_from_a_question() {
        let view = ViewBuilder::new(2)
            .with_looking_at(vec![printed(4, 0, "Forest", 3)])
            .build();
        let it = Interaction::new(
            baylee_engine::choice::Pending::ChooseCards {
                player: PlayerId::new(0),
                options: vec![ObjectId::new(4, 0)],
                min: 1,
                max: 1,
                prompt: baylee_engine::choice::ChoicePrompt::Generic,
            },
            PlayerId::new(0),
        );
        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        assert!(b.answers_here(Some(&it)), "the sheet holds the question");

        b.open_at(BrowseZone::Graveyard(PlayerId::new(0)));

        assert!(
            b.answers_here(Some(&it)),
            "a tap on a pile took the sheet away from the question"
        );
        assert_eq!(
            b.tab(),
            Some(BrowseZone::Looking),
            "and it must not have moved the tab either"
        );
    }

    /// The counter-test: with no question standing, a tap on a pile is
    /// exactly what opens that pile, which is the whole job of `open_at`.
    #[test]
    fn a_tap_on_a_pile_still_opens_that_pile() {
        let mut b = Browser::new();
        b.open_at(BrowseZone::Graveyard(PlayerId::new(0)));
        assert!(b.is_open());
        assert_eq!(b.tab(), Some(BrowseZone::Graveyard(PlayerId::new(0))));
    }

    /// The cycle is one control, so a direction must not survive a key change.
    #[test]
    fn cycling_the_sort_key_starts_it_the_right_way_up() {
        let mut b = Browser::new();
        b.reverse();
        assert!(b.descending());
        b.cycle_sort();
        assert_eq!(b.sort(), SortKey::Name);
        assert!(!b.descending(), "descending carried into a new key");
    }

    #[test]
    fn the_filter_narrows_by_name_and_ignores_case() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![
                    printed(4, 0, "Llanowar Elves", 3),
                    printed(5, 0, "Forest", 4),
                ],
            )
            .build();
        let mut b = Browser::new();
        b.set_filter("ELV");
        let rows = b.rows(&view, None, Names::projected());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Llanowar Elves");

        b.set_filter("  ");
        assert_eq!(
            b.rows(&view, None, Names::projected()).len(),
            2,
            "blank is not a filter"
        );
    }

    /// The panel drew one name and searched another.
    ///
    /// A seat reading German sees *Wald* on the row — the renderer has
    /// translated the drawn name since the catalog existed — and typing
    /// `Wald` into the box under it found nothing, because the filter was
    /// asking `object.name`, which the engine keeps in its one language.
    /// Both names answer now: the one on the row, and the one the card is
    /// known by everywhere outside this client.
    #[test]
    fn the_filter_answers_the_name_on_the_row_and_the_one_under_it() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![
                    printed(4, 0, "Forest", 0),
                    printed(5, 0, "Swamp", 0),
                    printed(6, 0, "Llanowar Elves", 3),
                ],
            )
            .build();
        // The catalog, stood in for: this seat's printings are German.
        let german = |object: &baylee_view::PublicObject| match object.name.as_str() {
            "Forest" => Some("Wald".to_string()),
            "Swamp" => Some("Sumpf".to_string()),
            _ => None,
        };
        let names = Names { shown: &german };
        let found = |needle: &str| {
            let mut b = Browser::new();
            b.set_filter(needle);
            b.rows(&view, None, names)
                .into_iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
        };

        assert_eq!(found("wald"), ["Wald"], "the name the player is looking at");
        assert_eq!(found("forest"), ["Wald"], "the name they learned it under");
        assert_eq!(found("wal"), ["Wald"], "a prefix, which is how one types");
        assert!(found("sumpf") == ["Sumpf"] && found("mountain").is_empty());
        // A card the catalog has no German printing of keeps its own name,
        // and is still found by it — the fallback is a row, not a hole.
        assert_eq!(found("elves"), ["Llanowar Elves"]);
    }

    /// Both ends of the panel read the same alphabet.
    ///
    /// The seam put German names on the rows and left them being compared as
    /// bytes, which files every accented letter above `z`: a graveyard sorted
    /// by name put *Ätherfluss* at the bottom, under *Zombie*. The filter had
    /// the other half of it — nothing typed on a keyboard without an `ß`
    /// could ever find a card printed with one.
    #[test]
    fn the_panel_alphabetises_and_searches_in_the_readers_own_letters() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![
                    printed(4, 0, "Zombie", 2),
                    printed(5, 0, "Aetherflux", 3),
                    printed(6, 0, "Brainstorm", 1),
                ],
            )
            .build();
        // The same three cards, as this seat's printings name them.
        let german = |object: &baylee_view::PublicObject| {
            Some(match object.name.as_str() {
                "Aetherflux" => "Ätherfluss".to_string(),
                same => same.to_string(),
            })
        };
        let names = Names { shown: &german };

        let mut b = Browser::new();
        b.sort_by(SortKey::Name);
        let order: Vec<String> = b
            .rows(&view, None, names)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(
            order,
            ["Ätherfluss", "Brainstorm", "Zombie"],
            "the accent belongs at the front, with the A it is one of"
        );

        b.set_filter("atherfluss");
        assert_eq!(
            b.rows(&view, None, names).len(),
            1,
            "a keyboard with no umlaut still finds the card"
        );
        b.set_filter("ss");
        assert_eq!(
            b.rows(&view, None, names).len(),
            1,
            "and so does the ss in it"
        );
    }

    #[test]
    fn the_viewing_seats_own_piles_come_first() {
        let view = ViewBuilder::new(2)
            .with_stack(vec![printed(3, 1, "Counterspell", 2)])
            .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
            .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
            .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
            .build();
        assert_eq!(
            Browser::new().zones(&view),
            vec![
                BrowseZone::Looking,
                BrowseZone::Stack,
                BrowseZone::Graveyard(PlayerId::new(0)),
                BrowseZone::Graveyard(PlayerId::new(1)),
            ],
            "shown cards, then the stack, then mine, then theirs"
        );
    }

    #[test]
    fn a_choice_for_another_seat_offers_nothing() {
        let view = ViewBuilder::new(2)
            .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
            .build();
        let it = Interaction::new(
            Pending::ChooseCards {
                player: PlayerId::new(1),
                options: vec![obj(7)],
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            me(),
        );
        assert!(!Browser::wanted(&view, &it));
        assert!(
            Browser::new()
                .rows(&view, Some(&it), Names::projected())
                .iter()
                .all(|r| !r.standing.selectable),
            "watching another seat choose is not choosing"
        );
    }

    /// The filter box is a field a player focuses, not a keyboard trap.
    ///
    /// `set_filter` existed from the start and nothing ever called it: the
    /// panel could sort and scroll, and the one thing the owner asked for by
    /// name — "durchsuchbar" — had no way in. It is typed into now, and the
    /// bargain is that it has to be *given* the keyboard: a box that took
    /// every keystroke while the panel merely stood open would end playing
    /// with the graveyard visible.
    #[test]
    fn the_filter_box_only_types_while_it_holds_the_keyboard() {
        let mut b = Browser::new();
        assert!(!b.is_typing(), "a fresh panel does not own the keyboard");

        b.start_typing();
        assert!(b.is_open(), "focusing the box opens the panel it lives in");
        assert!(b.is_typing());
        let focused = b.typing_epoch();

        for c in "Elv".chars() {
            b.push_filter(c);
        }
        b.push_filter('\n');
        assert_eq!(b.filter(), "Elv", "a control character reached the text");
        assert!(b.pop_filter());
        assert_eq!(b.filter(), "El");

        // The same rule on the path that needs it more. A keystroke is one
        // character a player meant; `set_filter` is a whole value arriving
        // from autofill or a paste, which is where a newline actually comes
        // from — and a filter holding one matches nothing at all.
        b.set_filter("Ll\tanowar\n");
        assert_eq!(
            b.filter(),
            "Llanowar",
            "a pasted value kept its control codes"
        );

        // Focusing again while already focused is not a new focus: a platform
        // input pointed at the box on every frame would fight the player for
        // the caret.
        b.start_typing();
        assert_eq!(b.typing_epoch(), focused);

        // Emptying the box, on the other hand, *is* one — the platform's own
        // field is still holding the old letters until something points it at
        // the new value.
        b.clear_filter();
        assert_eq!(b.filter(), "");
        assert!(b.typing_epoch() > focused, "the field was not re-seeded");
        b.set_filter("El");

        b.stop_typing();
        assert!(!b.is_typing());
        assert_eq!(
            b.filter(),
            "El",
            "letting go of the box threw the text away"
        );

        // And closing the panel lets go: the keyboard belongs to the game
        // again the moment the panel is not on screen.
        b.start_typing();
        b.close();
        assert!(!b.is_typing());
    }

    /// Typing narrows the rows, which is the whole point of the box.
    #[test]
    fn what_is_typed_is_what_is_listed() {
        let view = ViewBuilder::new(2)
            .with_graveyard(
                0,
                vec![
                    printed(1, 0, "Elvish Mystic", 1),
                    printed(2, 0, "Mountain", 2),
                ],
            )
            .build();
        let mut b = Browser::new();
        b.open();
        assert_eq!(b.rows(&view, None, Names::projected()).len(), 2);
        b.start_typing();
        for c in "mou".chars() {
            b.push_filter(c);
        }
        let rows = b.rows(&view, None, Names::projected());
        assert_eq!(rows.len(), 1, "the filter did not reach the rows");
        assert_eq!(rows[0].name, "Mountain");
    }
}
