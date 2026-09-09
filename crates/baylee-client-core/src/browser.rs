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
    /// Its projected name — a clone shows the name it copied.
    pub name: String,
    /// Its picture, when it has one. A token in a graveyard has none.
    pub art: Option<ImageKey>,
    /// Where it is.
    pub zone: BrowseZone,
    /// Whether the pending choice would accept it.
    pub selectable: bool,
    /// Whether it is part of the answer being assembled.
    pub selected: bool,
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
    /// By name.
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

/// The panel's own state — what the player has said about it, nothing more.
#[derive(Clone, Default, Debug)]
pub struct Browser {
    open: bool,
    tab: Option<BrowseZone>,
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
        self.open
    }

    /// The zone tab in force, or `None` for "every zone at once".
    #[must_use]
    pub fn tab(&self) -> Option<BrowseZone> {
        self.tab
    }

    /// What is typed in the filter.
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Opens the panel on every zone.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Opens the panel on one zone — what a tap on a pile does.
    pub fn open_at(&mut self, zone: BrowseZone) {
        self.open = true;
        self.tab = Some(zone);
    }

    /// Closes the panel, keeping the tab and filter for the next time.
    pub fn close(&mut self) {
        self.open = false;
        self.typing = false;
    }

    /// Shows one zone, or every zone when given `None`.
    pub fn show(&mut self, tab: Option<BrowseZone>) {
        self.tab = tab;
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
        self.open = true;
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
    pub fn follow(&mut self, view: &PlayerView, interaction: Option<&Interaction>) {
        if let Some(it) = interaction
            && Self::wanted(view, it)
        {
            self.open = true;
            self.tab = None;
            self.filter.clear();
        }
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
        let now: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
        if now != self.looking_seen {
            if !now.is_empty() {
                self.open = true;
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

    /// The rows to draw, in tab order and then in each zone's own order.
    ///
    /// Pass `None` for the interaction to browse with no question pending —
    /// what tapping a pile does. Nothing is selectable then, which is
    /// the honest answer: there is nothing to select *for*.
    #[must_use]
    pub fn rows(&self, view: &PlayerView, interaction: Option<&Interaction>) -> Vec<BrowseRow> {
        let mine = interaction.filter(|it| it.is_mine());
        let ordering = mine.is_some_and(Interaction::is_ordering);
        let needle = self.filter.trim().to_lowercase();
        let mut out = Vec::new();
        for zone in self.zones(view) {
            if self.tab.is_some_and(|t| t != zone) {
                continue;
            }
            for object in objects_in(view, zone) {
                if !needle.is_empty() && !object.name.to_lowercase().contains(&needle) {
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
                    name: object.name.clone(),
                    // Same fallback as the board's: a token has no printing
                    // but does have a picture, and the row beside its badge
                    // is the one place a player reads a token as a card.
                    art: object
                        .card
                        .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small))
                        .or_else(|| object.token.map(|t| ImageKey::token(t, ArtSize::Small))),
                    zone,
                    selectable,
                    selected: mine.is_some_and(|it| it.is_selected(object.id)),
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
        rows.sort_by(|a, b| {
            let zone = a.zone.cmp(&b.zone);
            if zone != std::cmp::Ordering::Equal {
                return zone;
            }
            let within = match self.sort {
                // The place *is* the key here, not the tie-break, or asking
                // for the pile upside down would compare equal and do nothing.
                SortKey::Place => place(a).cmp(&place(b)),
                SortKey::Name => a.name.cmp(&b.name),
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
    use crate::board::{BoardModel, Openings};
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
        let board = BoardModel::from_view(view, Openings::none(), |_| 100.0);
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

        let rows = b.rows(&view, Some(&it));
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|r| r.zone == BrowseZone::Looking));
        assert!(rows.iter().all(|r| r.selectable), "all four were offered");
        assert!(rows.iter().all(|r| r.art.is_some()), "each has a picture");
    }

    /// A reveal with no question attached opens the sheet by itself.
    ///
    /// This is the job the "Zones" chip used to do and the reason the chip
    /// could not simply be deleted: cards in `looking_at` are shown to a seat
    /// without anything being asked of them, they are drawn on no other
    /// surface in the client, and `follow` only ever runs when a *choice*
    /// arrives. Edge-triggered on the ids, so the player can put it away.
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
            let rows = b.rows(&view, Some(&it));
            for id in it.selectable() {
                let on_table = table.contains(id);
                let in_tray = rows.iter().any(|r| r.id == *id && r.selectable);
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
            b.rows(&view, Some(&it)).iter().all(|r| r.place.is_none()),
            "nothing picked yet"
        );
        it.toggle(obj(8));
        it.toggle(obj(7));
        let rows = b.rows(&view, Some(&it));
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
            b.rows(&view, Some(&it)).iter().all(|r| !r.selectable),
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

        let rows = b.rows(&view, None);
        assert_eq!(rows.len(), 1, "the tab confines it to one pile");
        assert_eq!(rows[0].name, "Birds of Paradise");
        assert!(!rows[0].selectable, "there is nothing to select for");
        assert!(rows[0].place.is_none());

        b.show(None);
        assert_eq!(b.rows(&view, None).len(), 2, "both piles, unfiltered");
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
            b.rows(&view, None).into_iter().map(|r| r.name).collect()
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
            b.rows(&view, None).into_iter().map(|r| r.name).collect()
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
        let rows = b.rows(&view, None);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].zone,
            BrowseZone::Graveyard(PlayerId::new(0)),
            "the exile card sorted ahead of the graveyard it is not in"
        );
        assert_eq!(rows[1].zone, BrowseZone::Exile(PlayerId::new(0)));
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
        let rows = b.rows(&view, None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Llanowar Elves");

        b.set_filter("  ");
        assert_eq!(b.rows(&view, None).len(), 2, "blank is not a filter");
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
                .rows(&view, Some(&it))
                .iter()
                .all(|r| !r.selectable),
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
        assert_eq!(b.rows(&view, None).len(), 2);
        b.start_typing();
        for c in "mou".chars() {
            b.push_filter(c);
        }
        let rows = b.rows(&view, None);
        assert_eq!(rows.len(), 1, "the filter did not reach the rows");
        assert_eq!(rows[0].name, "Mountain");
    }
}
