//! Replacement rules that multiply what an effect does (CR 614): Doubling
//! Season, Elspeth Storm-Slayer and the cards written like them.
//!
//! They live in one module because the bug that collected them here was
//! one reader taking its own rule differently from the rest: a filter over
//! the *affected controller* was being asked of the resolving effect's own
//! controller, which is a question that answers yes for everybody, so my
//! Doubling Season doubled an opponent's tokens.
//!
//! There are exactly three readers, and they are **doors** for the same
//! reason [`crate::sba::destroy`] is: [`put_counters`] here, and
//! `resolve::tokens::create_tokens` and
//! `resolve::tokens::create_token_copies` beside the token
//! factory. A counter or a token produced beside one of them is invisible
//! to the rule, and nothing says so until someone plays the pair — which is
//! exactly how "put a +1/+1 counter on each other Ally you control", the
//! shape most of the pool writes, sat outside while the single-target shape
//! next to it was inside, and how a token *copy* was outside while the
//! plain token was in.

use crate::eval;
use crate::event::GameEvent;
use crate::state::GameState;
use baylee_core::ids::{ObjectId, PlayerId};

/// How many times over an effect creating tokens under `recipient`'s
/// control actually creates them (CR 614.1).
///
/// `controller_filter` is a filter over the **affected controller**, not
/// over the effect doing the creating: "if one or more tokens would be
/// created under *your* control" is a statement about who ends up with them
/// and says nothing about whose spell put them there. So it is read against
/// the replacement's own source with the recipient as "you", which makes
/// `ControlledByYou` mean "the enchantment that recipient controls".
#[must_use]
pub fn token_multiplier(state: &GameState, recipient: PlayerId) -> u32 {
    let mut count = 1u32;
    for entry in &state.replacement_rules {
        if let baylee_cards_dsl::ReplacementRule::DoubleTokenCreation { controller_filter } =
            entry.rule
            && let Some(source_obj) = state.object(entry.source)
            && eval::matches(
                controller_filter,
                state,
                source_obj,
                recipient,
                entry.source,
            )
        {
            count *= 2;
        }
    }
    count
}

/// How many times over counters put on `target` are actually put on it
/// (CR 614.16).
///
/// The mirror of [`token_multiplier`] and read the other way round, because
/// this rule's filter is over the **object receiving them**: "a permanent
/// you control" is about the permanent, whoever's effect is placing them.
/// So the filter is matched against `target` with the *replacement's* own
/// controller as "you" — an opponent's spell putting a counter on my
/// creature is doubled by my Doubling Season, and mine putting one on
/// theirs is not.
#[must_use]
pub fn counter_multiplier(state: &GameState, target: ObjectId) -> u16 {
    let Some(target_obj) = state.object(target) else {
        return 1;
    };
    let mut count = 1u16;
    for entry in &state.replacement_rules {
        if let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement { object_filter } =
            entry.rule
            && eval::matches(
                object_filter,
                state,
                target_obj,
                entry.controller,
                entry.source,
            )
        {
            count = count.saturating_mul(2);
        }
    }
    count
}

/// Puts `n` counters of `kind` on `id`, after the replacements that
/// multiply them, and records the change.
///
/// Every effect that puts counters on a permanent goes through here. The
/// journal entry and the projection invalidation are part of the door and
/// not of the caller: a counter is a characteristic input (CR 613.4c), so
/// one placed without invalidating leaves the creature drawn at its old
/// size until something else asks for a pass.
pub fn put_counters(
    state: &mut GameState,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    let n = n.saturating_mul(counter_multiplier(state, id));
    record_counters(state, id, kind, n);
}

/// The same door with the multiplying replacements left out: `n` counters
/// land, recorded and with the projections invalidated, and nothing gets to
/// say otherwise.
///
/// There is one caller, and CR 614.16 is why it is separate. A
/// counter-doubling replacement applies to what "the effect of a resolving
/// spell or ability" places and to what another replacement effect places —
/// and a **turn-based action** is neither of those. The lore counter a Saga
/// takes as its controller's precombat main phase begins is exactly that
/// (CR 505.4, CR 714.3b, both of which say it does not use the stack), so it
/// comes through here. The counter it takes as it *enters* is a replacement
/// effect (CR 614.1c) and goes through [`put_counters`] like everything else.
pub fn record_counters(
    state: &mut GameState,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) {
    if let Some(obj) = state.object_mut(id) {
        let old = obj.counters.get(kind);
        let new = obj.counters.add(kind, n);
        state.journal.record(GameEvent::CounterChanged {
            object: id,
            kind,
            old,
            new,
        });
    }
    state.invalidate_projections();
}

/// Takes `n` counters of `kind` off `id`, records the change, and answers how
/// many actually came off.
///
/// The sibling of [`record_counters`] and in this module for the reason that
/// one is: the journal entry and the projection invalidation are part of the
/// door. A +1/+1 counter spent on Walking Ballista's ping makes the creature
/// smaller, and a removal that skipped the invalidation would leave it drawn
/// at its old size and, worse, leave the state-based action that kills it at
/// nought toughness reading a cached projection.
///
/// It takes no multiplier, and that is the asymmetry rather than an omission:
/// CR 614.16 and the counter-doubling replacements are about counters being
/// **put** on a permanent, and Magic prints nothing that multiplies a
/// removal. A cost of one counter is a cost of one counter under any number
/// of Doubling Seasons.
///
/// It also saturates rather than refusing. Every caller has already checked
/// that enough are there — `can_afford` for a cost, the rules for everything
/// else — and a door that could half-fail would make the two answers
/// disagree at the one moment it matters.
pub fn remove_counters(
    state: &mut GameState,
    id: ObjectId,
    kind: baylee_cards_dsl::CounterKind,
    n: u16,
) -> u16 {
    let mut taken = 0;
    if let Some(obj) = state.object_mut(id) {
        let old = obj.counters.get(kind);
        taken = old.min(n);
        let new = old - taken;
        obj.counters.set(kind, new);
        state.journal.record(GameEvent::CounterChanged {
            object: id,
            kind,
            old,
            new,
        });
    }
    state.invalidate_projections();
    taken
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::{CardLookup, ReplacementEntry};
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{CounterKind, Filter, ReplacementRule};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    /// Two seats, empty boards, nothing on the stack. Every rule here is
    /// pushed by hand: `replacement_rules` is rebuilt only by the engine's
    /// static-ability pass (`engine::progress`), which no test in this
    /// module runs, so an entry put here stays put.
    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: forest,
                print: baylee_core::ids::PrintRef::new(0),
            })
            .collect();
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 9,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    /// A card-less permanent on `owner`'s battlefield. These two rules ask
    /// one question of an object — who controls it — so a bare one is the
    /// whole population a filter over `ControlledByYou` can see.
    fn permanent(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    }

    fn doubles_tokens(state: &mut GameState, controller: PlayerId) {
        let source = permanent(state, controller, "Doubling Season");
        state.replacement_rules.push(ReplacementEntry {
            source,
            controller,
            rule: ReplacementRule::DoubleTokenCreation {
                controller_filter: &Filter::ControlledByYou,
            },
        });
    }

    fn doubles_counters(state: &mut GameState, controller: PlayerId) {
        let source = permanent(state, controller, "Doubling Season");
        state.replacement_rules.push(ReplacementEntry {
            source,
            controller,
            rule: ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::ControlledByYou,
            },
        });
    }

    /// The bug this module was collected around, from the side it was seen
    /// on: "if one or more tokens would be created under **your** control"
    /// is a statement about who ends up with them, so it is read against the
    /// recipient. Asked instead of the resolving effect's own controller it
    /// answers yes for everybody, and my Doubling Season doubles an
    /// opponent's tokens.
    #[test]
    fn a_token_doubler_reads_its_filter_against_the_recipient() {
        let mut state = state();
        doubles_tokens(&mut state, me());
        assert_eq!(token_multiplier(&state, me()), 2, "my tokens are doubled");
        assert_eq!(
            token_multiplier(&state, them()),
            1,
            "and an opponent's are not — the filter is over the affected \
             controller, not over whose effect is creating them"
        );
    }

    /// The mirror, and read the other way round on purpose: this filter is
    /// over the permanent *receiving* the counters. Neither
    /// [`counter_multiplier`] nor [`put_counters`] takes a controller
    /// argument at all, which is the same sentence said in the signature —
    /// an opponent's spell putting a counter on my creature is doubled by my
    /// Doubling Season, because the door cannot know whose spell it was.
    #[test]
    fn a_counter_doubler_reads_its_filter_against_the_permanent() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");
        let theirs = permanent(&mut state, them(), "Theirs");

        assert_eq!(counter_multiplier(&state, mine), 2);
        assert_eq!(counter_multiplier(&state, theirs), 1);

        put_counters(&mut state, mine, CounterKind::P1P1, 1);
        put_counters(&mut state, theirs, CounterKind::P1P1, 1);
        assert_eq!(
            state
                .object(mine)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            2
        );
        assert_eq!(
            state
                .object(theirs)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            1
        );
    }

    /// An object that is gone is not a permanent under anybody's
    /// replacement, and the answer is one rather than a panic: the door is
    /// called from resolution, where a target can have left since the
    /// effect was put on the stack.
    #[test]
    fn a_counter_multiplier_for_an_object_that_is_gone_is_one() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");
        state.arena.remove(mine);
        assert_eq!(counter_multiplier(&state, mine), 1);
    }

    /// Two of them multiply rather than either one winning: CR 614.1 applies
    /// each replacement once, so the second one sees four where the first
    /// made two. Both loops are written this way and neither had a test.
    #[test]
    fn two_doublers_multiply() {
        let mut state = state();
        doubles_tokens(&mut state, me());
        doubles_tokens(&mut state, me());
        doubles_counters(&mut state, me());
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");

        assert_eq!(token_multiplier(&state, me()), 4);
        assert_eq!(counter_multiplier(&state, mine), 4);

        put_counters(&mut state, mine, CounterKind::P1P1, 3);
        assert_eq!(
            state
                .object(mine)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            12,
            "the multiplier applies to the whole placement, not to one counter"
        );
    }

    /// The reason [`record_counters`] exists beside [`put_counters`].
    /// CR 614.16 doubles what a resolving spell or ability places and what
    /// another replacement places — and a **turn-based action** is neither,
    /// so the lore counter a Saga takes as its controller's main phase
    /// begins (CR 714.3b) lands as one counter under any number of Doubling
    /// Seasons.
    #[test]
    fn a_turn_based_counter_is_not_doubled() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let saga = permanent(&mut state, me(), "Saga");

        record_counters(&mut state, saga, CounterKind::Lore, 1);
        assert_eq!(
            state
                .object(saga)
                .expect("still there")
                .counters
                .get(CounterKind::Lore),
            1,
            "the turn-based action puts one"
        );

        put_counters(&mut state, saga, CounterKind::Lore, 1);
        assert_eq!(
            state
                .object(saga)
                .expect("still there")
                .counters
                .get(CounterKind::Lore),
            3,
            "and the same counter placed by an effect is doubled, which is \
             what makes the two doors different rather than redundant"
        );
    }

    /// Magic prints nothing that multiplies a removal, which is why
    /// [`remove_counters`] takes no multiplier. It saturates rather than
    /// refusing, and answers how many actually came off — the number a cost
    /// or an effect paid with.
    #[test]
    fn nothing_multiplies_a_removal() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");
        record_counters(&mut state, mine, CounterKind::P1P1, 3);

        assert_eq!(
            remove_counters(&mut state, mine, CounterKind::P1P1, 1),
            1,
            "one comes off under two doublings of its placement"
        );
        assert_eq!(
            remove_counters(&mut state, mine, CounterKind::P1P1, 5),
            2,
            "and asking for more than is there takes what is there"
        );
        assert_eq!(
            state
                .object(mine)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            0
        );
        assert_eq!(
            remove_counters(&mut state, mine, CounterKind::P1P1, 1),
            0,
            "a removal from nothing removes nothing"
        );
    }

    /// The journal entry and the projection invalidation are part of the
    /// door and not of the caller: a counter is a characteristic input
    /// (CR 613.4c), so one placed without invalidating leaves the creature
    /// drawn — and read by the state-based action that kills it — at its old
    /// size. All three doors owe both.
    #[test]
    fn every_counter_door_records_the_change_and_invalidates_the_projection() {
        let mut state = state();
        let mine = permanent(&mut state, me(), "Mine");

        // The three doors run on one object in turn and this state carries
        // no doubling rule, so `n` is simply what the store reads after
        // each: 2 placed, 3 after one more, 0 once three come off.
        for (label, n) in [("put", 2u16), ("record", 3), ("remove", 0)] {
            let before = state.journal.len();
            // `from_preset` leaves this at the invalid sentinel, so a fresh
            // state would pass the assertion below having done nothing.
            state.characteristics_generation = 0;
            match label {
                "put" => put_counters(&mut state, mine, CounterKind::P1P1, 2),
                "record" => record_counters(&mut state, mine, CounterKind::P1P1, 1),
                _ => {
                    remove_counters(&mut state, mine, CounterKind::P1P1, 3);
                }
            }
            assert_eq!(
                state.characteristics_generation,
                u64::MAX,
                "{label} left a cached projection standing"
            );
            assert_eq!(state.journal.len(), before + 1, "{label} recorded nothing");
            let last = state.journal.entries().last().expect("an entry");
            assert!(
                matches!(
                    last.event,
                    GameEvent::CounterChanged {
                        object,
                        kind: CounterKind::P1P1,
                        new,
                        ..
                    } if object == mine && new == n
                ),
                "{label} recorded {:?}",
                last.event
            );
        }
    }
}
