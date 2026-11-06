//! From the question in the view to what the sheet shows, and from what the
//! player sent back to what they taught.
//!
//! `docs/protocol.md` §"The order of triggers": the rows are
//! `PlayerView::trigger_order`, and an answer lists its handles top to
//! bottom, the first resolving first, which is the order everything here
//! already speaks in.

use super::facts::{self, Facts, Follower, Reading};
use super::memory::TriggerMemory;
use super::{Reason, Standing, Table, TriggerKey, Waiting, suggest};
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};
use baylee_view::{CardIdentity, GameStatic, OrderedTrigger, PlayerView, StackItem};

/// What the player's teaching is filed under, when anything can be: a
/// keyword's trigger is `AbilityRef::SYNTHETIC`, which two keywords of one
/// card share, and a rule's own trigger names nothing. Those are always
/// asked, because the teaching could not tell them from another.
#[must_use]
pub fn key_of(trigger: &OrderedTrigger) -> Option<TriggerKey> {
    let StackItem::Ability { ability, token, .. } = &trigger.item else {
        return None;
    };
    match (ability, token) {
        (Some(ability), _) if ability.index != AbilityRef::SYNTHETIC => {
            Some(TriggerKey::Card(*ability))
        }
        (None, Some(token)) => Some((*token).into()),
        _ => None,
    }
}

/// The card a row's picture comes from: its source while this seat can see
/// the printing, else the card the ability is printed on.
#[must_use]
pub fn card_of(trigger: &OrderedTrigger, view: &PlayerView) -> Option<CardIdentity> {
    let StackItem::Ability { source, rules, .. } = trigger.item else {
        return None;
    };
    let shown = view.object(source).and_then(|object| object.card);
    match (shown, rules) {
        (Some(card), Some(rules)) if card.index == rules.card => Some(card),
        (_, Some(rules)) => Some(CardIdentity {
            index: rules.card,
            print: baylee_core::ids::PrintRef::UNKNOWN,
            face: rules.face,
        }),
        (shown, None) => shown,
    }
}

/// Whether the player may decline this row as it resolves.
#[must_use]
pub fn is_optional(trigger: &OrderedTrigger, view: &PlayerView) -> bool {
    let reading = Reading {
        me: view.seat,
        opponents: &[],
        event_player: trigger.about_player,
        event_amount: trigger.amount,
    };
    key_of(trigger)
        .and_then(facts::ability)
        .is_some_and(|ability| {
            facts::of_ability(ability, &reading)
                .deeds
                .has(facts::Deeds::MAY)
        })
}

/// The table as the seat plates show it.
fn table_of(view: &PlayerView, statics: &GameStatic) -> Table {
    let team_of = |seat: PlayerId| {
        statics
            .seats
            .iter()
            .find(|s| s.player == seat)
            .and_then(|s| s.team)
    };
    let mine = team_of(view.seat);
    Table {
        me: view.seat,
        players: view
            .seats
            .iter()
            .map(|seat| Standing {
                seat: seat.player,
                life: seat.life,
                poison: u32::from(seat.poison),
                library: seat.library_count,
                ally: seat.player == view.seat || (mine.is_some() && team_of(seat.player) == mine),
            })
            .collect(),
    }
}

/// What the player's own permanents would do if a trigger set them off.
fn followers_of(view: &PlayerView, reading: &Reading<'_>) -> Vec<Follower> {
    view.battlefield
        .iter()
        .filter(|object| object.controller == view.seat)
        .filter_map(|object| baylee_cards::by_index(object.card?.index))
        .flat_map(|card| card.abilities)
        .filter_map(|ability| facts::follower(ability, reading))
        .collect()
}

/// One row of the question, as the suggestion reads it.
fn waiting_of(trigger: &OrderedTrigger, me: PlayerId, opponents: &[PlayerId]) -> Waiting {
    let key = key_of(trigger);
    let reading = Reading {
        me,
        opponents,
        event_player: trigger.about_player,
        event_amount: trigger.amount,
    };
    let source = match trigger.item {
        StackItem::Ability { source, .. } => Some(source),
        StackItem::Spell => None,
    };
    Waiting {
        key,
        source,
        twin: trigger.twin,
        facts: key
            .and_then(facts::ability)
            .map_or_else(Facts::default, |ability| {
                facts::of_ability(ability, &reading)
            }),
    }
}

/// What is shown for one question, and what its answer is measured against.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Sheet {
    /// The handles as the question offered them.
    offered: Vec<ObjectId>,
    /// The suggestion, the first to resolve first.
    suggested: Vec<ObjectId>,
    /// Why, in one line.
    reason: Reason,
    /// The handles the player's teaching placed.
    remembered: Vec<ObjectId>,
    /// What each handle is filed under.
    keys: Vec<(ObjectId, Option<TriggerKey>)>,
    /// The player allowed this one to go on without asking.
    unasked: bool,
    /// Twins the player wants to place one by one.
    split: Vec<u32>,
}

impl Sheet {
    /// The sheet for the question in this view, if it asks one.
    #[must_use]
    pub fn of(view: &PlayerView, statics: &GameStatic, memory: &TriggerMemory) -> Option<Self> {
        if view.trigger_order.is_empty() {
            return None;
        }
        let me = view.seat;
        let table = table_of(view, statics);
        let opponents: Vec<PlayerId> = table
            .players
            .iter()
            .filter(|player| !player.ally)
            .map(|player| player.seat)
            .collect();
        let reading = Reading {
            me,
            opponents: &opponents,
            event_player: None,
            event_amount: None,
        };
        let followers = followers_of(view, &reading);
        let waiting: Vec<Waiting> = view
            .trigger_order
            .iter()
            .map(|trigger| {
                let mut read = waiting_of(trigger, me, &opponents);
                read.facts = read.facts.with_followers(&followers);
                read
            })
            .collect();
        let suggestion = suggest(&waiting, &table, memory);
        let handle = |index: usize| view.trigger_order[index].handle;
        Some(Self {
            offered: view.trigger_order.iter().map(|t| t.handle).collect(),
            suggested: suggestion.order.iter().map(|i| handle(*i)).collect(),
            reason: suggestion.reason,
            remembered: suggestion
                .remembered
                .iter()
                .enumerate()
                .filter(|(_, remembered)| **remembered)
                .map(|(index, _)| handle(index))
                .collect(),
            keys: view
                .trigger_order
                .iter()
                .map(|t| (t.handle, key_of(t)))
                .collect(),
            unasked: suggestion.goes_unasked(memory),
            split: Vec::new(),
        })
    }

    /// Whether this sheet is for the question in this view.
    #[must_use]
    pub fn answers(&self, view: &PlayerView) -> bool {
        self.offered
            .iter()
            .copied()
            .eq(view.trigger_order.iter().map(|t| t.handle))
    }

    /// The suggestion, the first to resolve first.
    #[must_use]
    pub fn suggested(&self) -> &[ObjectId] {
        &self.suggested
    }

    /// The handles as the question offered them, which is what the clock
    /// sends when it runs out.
    #[must_use]
    pub fn offered(&self) -> &[ObjectId] {
        &self.offered
    }

    /// Why the suggestion stands as it does.
    #[must_use]
    pub const fn reason(&self) -> Reason {
        self.reason
    }

    /// Whether the player's teaching placed this row.
    #[must_use]
    pub fn is_remembered(&self, handle: ObjectId) -> bool {
        self.remembered.contains(&handle)
    }

    /// Whether this question goes on without being asked.
    #[must_use]
    pub const fn goes_unasked(&self) -> bool {
        self.unasked
    }

    /// The pile as the sheet's rows: twins side by side stand as one row, an
    /// answer listing every handle all the same, unless the player asked to
    /// place them one by one.
    #[must_use]
    pub fn runs(&self, pile: &[ObjectId], rows: &[OrderedTrigger]) -> Vec<Vec<ObjectId>> {
        let twin_of = |handle: ObjectId| {
            rows.iter()
                .find(|row| row.handle == handle)
                .map(|row| row.twin)
        };
        let mut runs: Vec<Vec<ObjectId>> = Vec::new();
        for &handle in pile {
            let twin = twin_of(handle);
            let together = twin.is_some_and(|twin| !self.is_split(twin))
                && runs
                    .last()
                    .and_then(|run| run.first())
                    .is_some_and(|first| twin_of(*first) == twin);
            match runs.last_mut().filter(|_| together) {
                Some(run) => run.push(handle),
                None => runs.push(vec![handle]),
            }
        }
        runs
    }

    /// Whether these twins are placed one by one.
    #[must_use]
    pub fn is_split(&self, twin: u32) -> bool {
        self.split.contains(&twin)
    }

    /// Places these twins one by one, or together again.
    pub fn toggle_split(&mut self, twin: u32) {
        if let Some(at) = self.split.iter().position(|t| *t == twin) {
            self.split.remove(at);
        } else {
            self.split.push(twin);
        }
    }

    fn keys_of(&self, order: &[ObjectId]) -> Vec<Option<TriggerKey>> {
        order
            .iter()
            .map(|handle| {
                self.keys
                    .iter()
                    .find(|(id, _)| id == handle)
                    .and_then(|(_, key)| *key)
            })
            .collect()
    }

    /// What the player's answer taught. Answers whether it taught anything.
    pub fn teach(&self, chosen: &[ObjectId], memory: &mut TriggerMemory) -> bool {
        memory.learn(&self.keys_of(&self.suggested), &self.keys_of(chosen))
    }
}

/// Where a card's row stands among the rows.
fn row_of(runs: &[Vec<ObjectId>], card: ObjectId) -> Option<usize> {
    runs.iter().position(|run| run.contains(&card))
}

/// The pile with `card`'s row one row earlier or later, or `None` at the end
/// of the list.
#[must_use]
pub fn stepped(runs: &[Vec<ObjectId>], card: ObjectId, later: bool) -> Option<Vec<ObjectId>> {
    let at = row_of(runs, card)?;
    let to = if later { at + 1 } else { at.checked_sub(1)? };
    if to >= runs.len() {
        return None;
    }
    let mut runs = runs.to_vec();
    runs.swap(at, to);
    Some(runs.concat())
}

/// The pile with `card`'s row where `onto`'s row stands: the answer to
/// dropping one on the other. `None` when they are one row.
#[must_use]
pub fn dropped(runs: &[Vec<ObjectId>], card: ObjectId, onto: ObjectId) -> Option<Vec<ObjectId>> {
    let from = row_of(runs, card)?;
    let to = row_of(runs, onto)?;
    if from == to {
        return None;
    }
    let mut runs = runs.to_vec();
    let row = runs.remove(from);
    runs.insert(to, row);
    Some(runs.concat())
}

/// The first card of the row next to `card`'s, for the keyboard.
#[must_use]
pub fn neighbour(runs: &[Vec<ObjectId>], card: ObjectId, later: bool) -> Option<ObjectId> {
    let at = row_of(runs, card)?;
    let to = if later { at + 1 } else { at.checked_sub(1)? };
    runs.get(to)?.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{CardIndex, ObjectId};

    const fn id(n: u32) -> ObjectId {
        ObjectId::queued_trigger(n)
    }

    fn row(handle: u32, twin: u32, ability: Option<AbilityRef>) -> OrderedTrigger {
        OrderedTrigger {
            handle: id(handle),
            item: StackItem::Ability {
                source: ObjectId::new(1, 0),
                ability,
                text: None,
                rules: None,
                token: None,
            },
            name: "A card".into(),
            twin,
            about_object: None,
            about_name: None,
            about_player: None,
            amount: None,
            asks_targets: false,
            modal: false,
        }
    }

    fn a_sheet() -> Sheet {
        Sheet {
            offered: Vec::new(),
            suggested: Vec::new(),
            reason: Reason::NoOpinion,
            remembered: Vec::new(),
            keys: Vec::new(),
            unasked: false,
            split: Vec::new(),
        }
    }

    /// Two keywords of one card share one handle and a rule's own trigger
    /// has none: neither may be remembered as if it were one ability.
    #[test]
    fn what_cannot_be_told_apart_or_named_is_filed_under_nothing() {
        let card = CardIndex::new(5);
        let printed = AbilityRef::new(card, 1);
        assert_eq!(
            key_of(&row(0, 0, Some(printed))),
            Some(TriggerKey::Card(printed))
        );
        let keyword = AbilityRef::new(card, AbilityRef::SYNTHETIC);
        assert_eq!(key_of(&row(0, 0, Some(keyword))), None);
        assert_eq!(key_of(&row(0, 0, None)), None);
    }

    #[test]
    fn twins_side_by_side_are_one_row_until_the_player_splits_them() {
        let rows = [
            row(0, 1, None),
            row(1, 1, None),
            row(2, 2, None),
            row(3, 1, None),
        ];
        let pile = [id(0), id(1), id(2), id(3)];
        let mut sheet = a_sheet();
        assert_eq!(
            sheet.runs(&pile, &rows),
            vec![vec![id(0), id(1)], vec![id(2)], vec![id(3)]],
            "twins apart stay apart"
        );
        sheet.toggle_split(1);
        assert_eq!(sheet.runs(&pile, &rows).len(), 4);
        sheet.toggle_split(1);
        assert_eq!(sheet.runs(&pile, &rows).len(), 3);
    }

    #[test]
    fn a_row_moves_whole_past_its_neighbour_and_stops_at_the_ends() {
        let runs = vec![vec![id(0), id(1)], vec![id(2)], vec![id(3), id(4)]];
        assert_eq!(
            stepped(&runs, id(1), true),
            Some(vec![id(2), id(0), id(1), id(3), id(4)])
        );
        assert_eq!(stepped(&runs, id(0), false), None);
        assert_eq!(stepped(&runs, id(4), true), None);
    }

    #[test]
    fn a_row_dropped_takes_the_place_of_the_row_under_it() {
        let runs = vec![vec![id(0), id(1)], vec![id(2)], vec![id(3)]];
        assert_eq!(
            dropped(&runs, id(0), id(3)),
            Some(vec![id(2), id(3), id(0), id(1)])
        );
        assert_eq!(
            dropped(&runs, id(3), id(1)),
            Some(vec![id(3), id(0), id(1), id(2)])
        );
        assert_eq!(dropped(&runs, id(0), id(1)), None, "one row");
    }

    #[test]
    fn the_keyboard_walks_rows_and_stops_at_the_ends() {
        let runs = vec![vec![id(0), id(1)], vec![id(2)]];
        assert_eq!(neighbour(&runs, id(1), true), Some(id(2)));
        assert_eq!(neighbour(&runs, id(2), false), Some(id(0)));
        assert_eq!(neighbour(&runs, id(0), false), None);
        assert_eq!(neighbour(&runs, id(2), true), None);
    }
}
