//! `cards/enchantments/mv_2/freyalise_s_charm.rs`, played.
//!
//! Oracle:
//! - "Whenever an opponent casts a black spell, you may pay {G}{G}. If you do, you draw a card."
//! - "{G}{G}: Return this enchantment to its owner's hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn freyalises_charm() -> CardIndex {
    card_index("e0d638d2-2c5b-415c-95e7-74bf8d88794f")
}

/// Passes until the stack is empty, answering every "pay {G}{G}?" with
/// `pay`, and returns how many times the Charm's controller was asked.
#[track_caller]
fn resolve_answering(engine: &mut Engine<RegistryLookup>, pay: bool) -> usize {
    let mut asked = 0;
    for _ in 0..30 {
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            return asked;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayMana { .. },
                ..
            } => {
                assert_eq!(player, P0, "\"you may pay\": the Charm's controller");
                asked += 1;
                engine.apply(player, PlayerAction::YesNo(pay)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    panic!("the stack never emptied");
}

/// P0 floats four green and passes; P1 casts Dark Ritual off its own black.
fn opponent_casts_dark_ritual() -> Engine<RegistryLookup> {
    let mut engine = lk::game(SEED, &[freyalises_charm()], &[], &[], &[index::DARK_RITUAL]);
    lk::float(&mut engine, P0, &[(ManaColor::Green, 4)]);
    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    engine
}

#[test]
fn freyalises_charm_draws_on_opponent_black_spell_for_gg_and_bounces_self() {
    let mut engine = opponent_casts_dark_ritual();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    assert_eq!(
        resolve_answering(&mut engine, true),
        1,
        "one black spell, one question"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1,
        "paying {{G}}{{G}} when opponent casts a black spell draws a card"
    );
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Green),
        2,
        "and the {{G}}{{G}} was paid for it"
    );

    activate(&mut engine, P0, freyalises_charm(), 1);
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P0, freyalises_charm()).is_some());
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Green),
        0,
        "the return cost {{G}}{{G}} too"
    );
}

/// "If you do": declined, nothing is paid and nothing is drawn.
#[test]
fn freyalises_charm_declined_costs_nothing_and_draws_nothing() {
    let mut engine = opponent_casts_dark_ritual();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    assert_eq!(resolve_answering(&mut engine, false), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before
    );
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Green), 4);
}

/// "An opponent casts a **black** spell": the Charm's controller's own Dark
/// Ritual asks nothing, and neither does the opponent's red Lightning Bolt.
#[test]
fn freyalises_charm_ignores_its_controller_s_black_spell_and_an_opponent_s_red_one() {
    let mut engine = lk::game(
        SEED,
        &[freyalises_charm()],
        &[],
        &[index::DARK_RITUAL],
        &[index::LIGHTNING_BOLT],
    );
    let library_before = library_size(&engine, P0);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    assert_eq!(
        resolve_answering(&mut engine, true),
        0,
        "its controller's own black spell is not an opponent's"
    );

    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P0]);
    assert_eq!(
        resolve_answering(&mut engine, true),
        0,
        "an opponent's red spell is not a black one"
    );
    assert_eq!(lk::life(&engine, P0), 17, "the Bolt resolved");
    assert_eq!(
        library_size(&engine, P0),
        library_before,
        "and nothing was drawn"
    );
}
