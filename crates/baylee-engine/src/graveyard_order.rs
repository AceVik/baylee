//! Owner choices for cards entering one graveyard simultaneously (CR 404.3).

use crate::choice::{ArrangePile, ArrangePlace, ArrangePrompt, Pending};
use crate::event::GameEvent;
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, CardDef, Condition, CostPart};
use baylee_core::ids::{ObjectId, PlayerId};
use std::collections::VecDeque;

#[derive(Clone, Debug, Default, Hash)]
pub(crate) struct Ordering {
    // Fixed when cards are introduced, including cards outside the game.
    // When no participating definition can read order, insertion order is
    // a harmless default and there is no strategic choice to interrupt for.
    pub enabled: bool,
    pub captured_through: u64,
    pub batches: VecDeque<Batch>,
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct Batch {
    pub owner: PlayerId,
    pub cards: Vec<(ObjectId, u32)>,
}

/// The DSL expands "sacrifice N" into consecutive sacrifice parts, including
/// a named source plus another permanent. Discard parts form their own group;
/// another payment action ends either group (independent costs, CR 601.2h).
/// Queued choices stay inside the caller's payment checkpoint until it succeeds.
pub(crate) struct PaymentBatch {
    since: u64,
    kind: Option<u8>,
}

impl PaymentBatch {
    pub(crate) fn new(state: &GameState) -> Self {
        Self {
            since: state.journal.last_seq(),
            kind: None,
        }
    }

    pub(crate) fn before_part(&mut self, state: &mut GameState, part: &CostPart) {
        let kind = match part {
            CostPart::SacrificeSelf | CostPart::Sacrifice(_) => Some(0),
            CostPart::DiscardSelf | CostPart::Discard(_) => Some(1),
            _ => None,
        };
        if kind != self.kind || kind.is_none() {
            capture(state, self.since);
            self.since = state.journal.last_seq();
        }
        self.kind = kind;
    }

    pub(crate) fn finish(self, state: &mut GameState) {
        capture(state, self.since);
    }
}

fn reads_order(condition: &Condition) -> bool {
    match condition {
        Condition::GraveyardCardsAbove(..) => true,
        Condition::All(parts) | Condition::Any(parts) => parts.iter().any(reads_order),
        Condition::Not(part) => reads_order(part),
        _ => false,
    }
}

pub(crate) fn card_reads_order(card: &CardDef) -> bool {
    (0..card.faces.len()).any(|face| {
        card.abilities_for_face(face)
            .iter()
            .any(|ability| match ability {
                AbilityDef::Triggered { condition, .. }
                | AbilityDef::ModalTriggered { condition, .. } => {
                    condition.as_ref().is_some_and(reads_order)
                }
                AbilityDef::ActivatedConditional { condition, .. } => reads_order(condition),
                _ => false,
            })
    })
}

/// Close one simultaneous instruction or SBA pass. Nested instructions have
/// already closed their own events, so the cursor prevents merging them.
pub(crate) fn capture(state: &mut GameState, since: u64) {
    if !state.graveyard_order.enabled {
        return;
    }
    let since = since.max(state.graveyard_order.captured_through);
    state.graveyard_order.captured_through = state.journal.last_seq();
    let mut batches: Vec<Batch> = Vec::new();
    for entry in state.journal.entries().iter().skip(since as usize) {
        let GameEvent::ZoneChanged {
            object,
            from,
            to: Zone::Graveyard,
            ..
        } = entry.event
        else {
            continue;
        };
        let Some(obj) = state.object(object) else {
            continue;
        };
        // Tokens are not cards; replacement effects may also have sent a
        // would-be arrival elsewhere. Same-zone rearrangements are not moves.
        if from == Zone::Graveyard || !obj.is_card() || obj.zone != Zone::Graveyard {
            continue;
        }
        if let Some(batch) = batches.iter_mut().find(|b| b.owner == obj.owner) {
            if !batch.cards.iter().any(|(id, _)| *id == object) {
                batch.cards.push((object, obj.version));
            }
        } else {
            batches.push(Batch {
                owner: obj.owner,
                cards: vec![(object, obj.version)],
            });
        }
    }
    // Each owner chooses in APNAP order (CR 101.4).
    let active = usize::from(state.turn.active.get());
    let seats = state.players.len();
    batches.sort_by_key(|b| (usize::from(b.owner.get()) + seats - active) % seats);
    state
        .graveyard_order
        .batches
        .extend(batches.into_iter().filter(|b| b.cards.len() > 1));
}

pub(crate) fn pending(state: &mut GameState) -> Option<Pending> {
    loop {
        let mut batch = state.graveyard_order.batches.pop_front()?;
        batch.cards.retain(|&(id, version)| {
            state
                .object(id)
                .is_some_and(|o| o.zone == Zone::Graveyard && o.version == version)
        });
        if state.has_left(batch.owner) || batch.cards.len() < 2 {
            continue;
        }
        let cards: Vec<_> = state
            .zones
            .list(ZoneLocation::Graveyard(batch.owner))
            .iter()
            .rev()
            .filter(|id| batch.cards.iter().any(|(card, _)| card == *id))
            .copied()
            .collect();
        let n = u32::try_from(cards.len()).unwrap_or(u32::MAX);
        let player = batch.owner;
        state.graveyard_order.batches.push_front(batch);
        return Some(Pending::Arrange {
            player,
            cards,
            piles: vec![ArrangePile::all_of(ArrangePlace::Graveyard, n)],
            prompt: ArrangePrompt::Order,
        });
    }
}

/// The validated answer is top first. Reorder only these positions: later
/// arrivals and older cards keep their places, and this creates no zone move.
pub(crate) fn answer(state: &mut GameState, cards: &[ObjectId]) {
    let batch = state
        .graveyard_order
        .batches
        .pop_front()
        .expect("graveyard order pending");
    let graveyard = state.zones.list_mut(ZoneLocation::Graveyard(batch.owner));
    let mut ordered = cards.iter().rev();
    for card in graveyard.iter_mut() {
        if batch.cards.iter().any(|(id, _)| id == card) {
            *card = *ordered.next().expect("every offered card was answered");
        }
    }
    state.invalidate_projections();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, land, preset};
    use crate::object::ObjectKind;
    use crate::resolve::{self, Flow, Resolution};
    use crate::zone::ZonePosition;
    use baylee_cards_dsl::{Amount, Effect, Filter, PlayerRel, StepKind, Trigger};
    use baylee_core::ids::SeatSet;
    use smallvec::SmallVec;

    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);
    const MARKER: u32 = 4_000_130;
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[],
        condition = Some(Condition::All(&[Condition::GraveyardCardsAbove(
            &Filter::Any,
            2
        )]))
    )];

    fn table() -> (GameState, Resolution) {
        let lookup = SyntheticLookup::new(vec![land(MARKER, "Order reader", ABILITIES)]);
        let state = GameState::from_preset(&preset(32, &[MARKER]), &lookup).unwrap();
        assert!(state.graveyard_order.enabled);
        let source = state.zones.list(ZoneLocation::Battlefield)[0];
        let res = Resolution {
            source,
            on_stack: source,
            controller: P0,
            effects: vec![],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            event_mana: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            subject: crate::resolve::SubjectContext::default(),
            retarget_left: None,
        };
        (state, res)
    }

    fn offered(flow: Flow, player: PlayerId) -> Vec<ObjectId> {
        let Flow::Wait(Pending::Arrange {
            player: chooser,
            cards,
            piles,
            prompt,
        }) = flow
        else {
            panic!("expected graveyard order, got {flow:?}");
        };
        assert_eq!(chooser, player);
        assert_eq!(prompt, ArrangePrompt::Order);
        assert_eq!(
            piles,
            vec![ArrangePile::all_of(
                ArrangePlace::Graveyard,
                cards.len() as u32
            )]
        );
        cards
    }

    #[test]
    fn graveyard_mill_owners_order_apnap_before_the_following_instruction() {
        let (mut state, mut res) = table();
        state.turn.active = P1;
        res.effects = vec![
            Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::EachPlayer,
            },
            Effect::DrawCards {
                amount: Amount::Fixed(1),
            },
        ];
        let cards1 = offered(resolve::run(&mut state, &mut res), P1);
        assert_eq!(state.zones.list(ZoneLocation::Hand(P0)).len(), 0);
        let pending_hash = state.snapshot_hash();
        let mut without_queue = state.clone();
        without_queue.graveyard_order.batches.clear();
        assert_ne!(pending_hash, without_queue.snapshot_hash());
        let mut replay = state.clone();
        let mut replay_res = res.clone();
        let mut answer1 = cards1;
        answer1.reverse();
        let before_moves = state.journal.len();
        let cards0 = offered(
            resolve::resume_arranged(&mut state, &mut res, &[answer1.clone()]),
            P0,
        );
        let replay_cards0 = offered(
            resolve::resume_arranged(&mut replay, &mut replay_res, &[answer1.clone()]),
            P0,
        );
        assert_eq!(cards0, replay_cards0);
        assert_eq!(state.snapshot_hash(), replay.snapshot_hash());
        assert_eq!(
            state.journal.len(),
            before_moves,
            "ordering is no second zone move"
        );
        assert_eq!(
            state
                .zones
                .list(ZoneLocation::Graveyard(P1))
                .iter()
                .rev()
                .copied()
                .collect::<Vec<_>>(),
            answer1
        );
        assert!(matches!(
            resolve::resume_arranged(&mut state, &mut res, &[cards0]),
            Flow::Complete
        ));
        assert_eq!(state.zones.list(ZoneLocation::Hand(P0)).len(), 1);
    }

    #[test]
    fn graveyard_separate_instructions_do_not_become_a_simultaneous_batch() {
        let (mut state, mut res) = table();
        res.effects = vec![
            Effect::Mill {
                amount: Amount::Fixed(1),
                target: PlayerRel::You
            };
            2
        ];
        let library = state.zones.list(ZoneLocation::Library(P0));
        let first = library[library.len() - 1];
        let second = library[library.len() - 2];
        assert!(matches!(resolve::run(&mut state, &mut res), Flow::Complete));
        assert_eq!(
            state.zones.list(ZoneLocation::Graveyard(P0)),
            &[first, second]
        );
        assert!(state.graveyard_order.batches.is_empty());
    }

    #[test]
    fn graveyard_milling_before_another_choice_preserves_that_choice() {
        let (mut state, mut res) = table();
        res.effects = vec![Effect::MillMayTakeOne {
            amount: 3,
            filter: &Filter::Any,
        }];
        let cards = offered(resolve::run(&mut state, &mut res), P0);
        let Flow::Wait(Pending::ChooseCards { options, .. }) =
            resolve::resume_arranged(&mut state, &mut res, std::slice::from_ref(&cards))
        else {
            panic!("the original take-one choice must follow the order");
        };
        assert_eq!(options.len(), 3);
        assert!(matches!(
            resolve::resume(&mut state, &mut res, &[cards[0]]),
            Flow::Complete
        ));
        assert!(state.zones.list(ZoneLocation::Hand(P0)).contains(&cards[0]));
    }

    #[test]
    fn graveyard_discard_choice_orders_before_its_draw() {
        let (mut state, mut res) = table();
        state.draw_cards(P0, 3);
        let hand = state.zones.list(ZoneLocation::Hand(P0)).clone();
        res.effects = vec![Effect::DiscardUpToThenDraw { count: 3 }];
        assert!(matches!(
            resolve::run(&mut state, &mut res),
            Flow::Wait(Pending::ChooseCards { .. })
        ));
        let cards = offered(resolve::resume(&mut state, &mut res, &hand), P0);
        assert!(state.zones.list(ZoneLocation::Hand(P0)).is_empty());
        assert!(matches!(
            resolve::resume_arranged(&mut state, &mut res, &[cards]),
            Flow::Complete
        ));
        assert_eq!(state.zones.list(ZoneLocation::Hand(P0)).len(), 3);
    }

    #[test]
    fn graveyard_nested_optional_program_resumes_after_each_completed_batch() {
        static BODY: &[Effect] = &[
            Effect::Mill {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            },
            Effect::Mill {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            },
        ];
        let (mut state, mut res) = table();
        res.effects = vec![
            Effect::MayDo { effects: BODY },
            Effect::DrawCards {
                amount: Amount::Fixed(1),
            },
        ];
        assert!(matches!(
            resolve::run(&mut state, &mut res),
            Flow::Wait(Pending::YesNo { .. })
        ));
        let first = offered(resolve::resume_may_do(&mut state, &mut res, true), P0);
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(P0)).len(), 2);
        let second = offered(
            resolve::resume_arranged(&mut state, &mut res, std::slice::from_ref(&first)),
            P0,
        );
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(P0)).len(), 4);
        assert!(second.iter().all(|id| !first.contains(id)));
        assert!(matches!(
            resolve::resume_arranged(&mut state, &mut res, &[second]),
            Flow::Complete
        ));
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(P0)).len(), 4);
        assert_eq!(state.zones.list(ZoneLocation::Hand(P0)).len(), 1);
    }

    #[test]
    fn graveyard_tokens_and_copies_are_not_offered_as_cards() {
        let (mut state, _) = table();
        let name = state.names.intern("Token");
        let token = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let cards = state
            .zones
            .list(ZoneLocation::Library(P0))
            .iter()
            .rev()
            .take(2)
            .copied()
            .collect::<Vec<_>>();
        state
            .object_mut(cards[1])
            .unwrap()
            .riders
            .push(crate::object::Rider::SpellCopy);
        let since = state.journal.last_seq();
        for id in [token, cards[0], cards[1]] {
            state
                .move_object(
                    id,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        capture(&mut state, since);
        assert!(
            pending(&mut state).is_none(),
            "only one actual card arrived"
        );
    }

    #[test]
    fn graveyard_no_order_reader_keeps_the_deterministic_default() {
        let (mut state, mut res) = table();
        state.graveyard_order.enabled = false;
        res.effects = vec![Effect::Mill {
            amount: Amount::Fixed(3),
            target: PlayerRel::You,
        }];
        assert!(matches!(resolve::run(&mut state, &mut res), Flow::Complete));
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(P0)).len(), 3);
    }
}
