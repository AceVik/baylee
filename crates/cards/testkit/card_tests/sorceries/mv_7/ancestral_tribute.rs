//! `cards/sorceries/mv_7/ancestral_tribute.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ancestral_tribute() -> CardIndex {
    card_index("380ea3b6-8eb4-466a-8680-ce5cfed9b8c4")
}

/// Ancestral Tribute (380ea3b6-8eb4-466a-8680-ce5cfed9b8c4) — {5}{W}{W} Sorcery:
/// "You gain 2 life for each card in your graveyard. Flashback {9}{W}{W}{W}."
#[test]
fn ancestral_tribute_gains_two_life_per_card_in_your_graveyard_and_flashes_back() {
    let mut engine = lk::game(SEED, &[plains(); 12], &[], &[ancestral_tribute()], &[]);
    seed_graveyard(&mut engine, P0, 3);
    seed_graveyard(&mut engine, P1, 5);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ancestral_tribute());
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 26, "3 cards in P0's gy -> +6 life");
    assert_eq!(lk::life(&engine, P1), 20);

    // Now P0's graveyard has 4 cards (the 3 seeded + Ancestral Tribute itself),
    // and while Ancestral Tribute is on the stack via Flashback {9}{W}{W}{W},
    // 3 cards remain in P0's graveyard -> +6 more life (26 -> 32) and exiled!
    let gy_tribute = in_graveyard(&engine, P0, ancestral_tribute()).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::White, 7)]);
    assert_eq!(lk::pool_total(&engine, P0), 12);
    engine
        .apply(P0, PlayerAction::CastSpell { card: gy_tribute })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "spent all {{9}}{{W}}{{W}}{{W}}"
    );
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 32);
    assert_eq!(lk::zone_of(&engine, gy_tribute), Zone::Exile);
}
