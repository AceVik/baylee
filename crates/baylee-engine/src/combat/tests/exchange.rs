//! Reassigning blockers does not declare them again and fails atomically.
use super::*;
fn attacks(state: &mut GameState, first: ObjectId, second: ObjectId) {
    state
        .combat
        .declare_attackers([first, second].map(|creature| AttackerInfo {
            creature,
            defending: Defender::Player(P1),
            blocked: false,
            band: None,
        }));
}
#[test]
fn exchange_moves_all_blockers_and_keeps_creature_blocking_both() {
    let mut state = empty_state();
    let first = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let second = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let one = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let two = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let both = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attacks(&mut state, first, second);
    for (blocker, attacker) in [(one, first), (two, first), (both, first), (both, second)] {
        state.declare_block(blocker, attacker);
    }
    let before = state.journal.last_seq();
    assert!(exchange_blockers(&mut state, first, second));
    assert_eq!(state.combat.blockers_of(first), vec![both]);
    assert_eq!(state.combat.blockers_of(second), vec![one, two, both]);
    assert_eq!(state.journal.last_seq(), before);
}
#[test]
fn exchange_rejects_one_illegal_pair_without_changing_any_pair() {
    let mut state = empty_state();
    let first = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let second = creature(&mut state, P0, 2, 2, KeywordSet::FLYING);
    let ground = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let air = creature(&mut state, P1, 2, 2, KeywordSet::FLYING);
    attacks(&mut state, first, second);
    state.declare_block(ground, first);
    state.declare_block(air, second);
    assert!(!exchange_blockers(&mut state, first, second));
    assert_eq!(state.combat.blockers_of(first), vec![ground]);
    assert_eq!(state.combat.blockers_of(second), vec![air]);
}
/// A creature that "can't block unless you control more creatures than
/// attacking player" (CR 509.1b) is not moved onto another attacker once it
/// no longer does, though it was a legal blocker when it was declared. The
/// exchange declares nothing, so what it asks is the block it would leave.
#[test]
fn exchange_is_held_to_a_restriction_that_compares_creature_counts() {
    let mut state = empty_state();
    let first = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let second = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let toady = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    let other = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attacks(&mut state, first, second);
    state.declare_block(toady, first);
    state.declare_block(other, second);
    let timestamp = state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(&state, toady);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P1,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Text,
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter,
        modifier: baylee_cards_dsl::Modifier::CantBlockUnlessYouControlMoreCreaturesThanAttacker,
    });

    assert!(
        !exchange_blockers(&mut state, first, second),
        "two creatures are not more than two"
    );
    assert_eq!(state.combat.blockers_of(first), vec![toady]);
    assert_eq!(state.combat.blockers_of(second), vec![other]);

    creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    assert!(exchange_blockers(&mut state, first, second));
    assert_eq!(state.combat.blockers_of(first), vec![other]);
    assert_eq!(state.combat.blockers_of(second), vec![toady]);
}
