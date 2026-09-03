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
use baylee_view::PlayerView;

use crate::i18n::Phrase;
use crate::images::{ArtSize, ImageKey};
use crate::interaction::Interaction;

/// A zone the browser can show.
///
/// Ordered as the tabs are: what the engine is showing first, because a
/// choice that opens the tray is nearly always about those cards.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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
}

/// The panel's own state — what the player has said about it, nothing more.
#[derive(Clone, Default, Debug)]
pub struct Browser {
    open: bool,
    tab: Option<BrowseZone>,
    filter: String,
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

    /// Opens the panel on one zone — what a pile chip does.
    pub fn open_at(&mut self, zone: BrowseZone) {
        self.open = true;
        self.tab = Some(zone);
    }

    /// Closes the panel, keeping the tab and filter for the next time.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Shows one zone, or every zone when given `None`.
    pub fn show(&mut self, tab: Option<BrowseZone>) {
        self.tab = tab;
    }

    /// Narrows the list to cards whose name contains `text`.
    pub fn set_filter(&mut self, text: impl Into<String>) {
        self.filter = text.into();
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
    /// what clicking a pile chip does. Nothing is selectable then, which is
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
                    art: object
                        .card
                        .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small)),
                    zone,
                    selectable,
                    selected: mine.is_some_and(|it| it.is_selected(object.id)),
                    place: ordering
                        .then(|| {
                            mine.and_then(|it| it.selected().iter().position(|o| *o == object.id))
                        })
                        .flatten()
                        .map(|p| p + 1),
                });
            }
        }
        out
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
    use baylee_engine::choice::{ChoicePrompt, Pending};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// Everything a client can already click without the browser: the
    /// battlefield as drawn cards, and the seat's own hand.
    fn drawn_on_the_table(view: &PlayerView) -> Vec<ObjectId> {
        let board = BoardModel::from_view(view, Openings::none(), 100.0);
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
}
