//! `cards/sorceries/mv_2/flash_of_defiance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flash_of_defiance() -> CardIndex {
    card_index("9f66855d-3ae8-4163-a5f1-aa23a9cd06a5")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Flash of Defiance (9f66855d-3ae8-4163-a5f1-aa23a9cd06a5) — {1}{R} Sorcery:
/// "Green creatures and white creatures can't block this turn. Flashback—{1}{R},
/// Pay 3 life."
#[test]
fn flash_of_defiance_stops_green_and_white_blockers_and_flashes_back_for_life() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            fangren_hunter(),
        ],
        &[aurochs(), savannah_lions(), festering_goblin()],
        &[flash_of_defiance()],
        &[],
    );
    let attacker = lk::obj(&engine, P0, fangren_hunter());
    let green_b = lk::obj(&engine, P1, aurochs());
    let white_b = lk::obj(&engine, P1, savannah_lions());
    let black_b = lk::obj(&engine, P1, festering_goblin());

    cast_from_hand(&mut engine, P0, flash_of_defiance());
    lk::settle(&mut engine);

    let offer = lk::attack(&mut engine, &[attacker]);
    assert!(!lk::may_block(&offer, green_b, attacker));
    assert!(!lk::may_block(&offer, white_b, attacker));
    assert!(lk::may_block(&offer, black_b, attacker));
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    // Next turn of P0: effect expired; cast Flash of Defiance via Flashback ({1}{R}, Pay 3 life).
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    let gy_card = in_graveyard(&engine, P0, flash_of_defiance()).unwrap();
    tap_all_mana(&mut engine, P0);
    engine
        .apply(P0, PlayerAction::CastSpell { card: gy_card })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 17, "paid 3 life for flashback");
    assert_eq!(lk::zone_of(&engine, gy_card), Zone::Exile);
    let fb_offer = lk::attack(&mut engine, &[attacker]);
    assert!(!lk::may_block(&fb_offer, green_b, attacker));
    assert!(!lk::may_block(&fb_offer, white_b, attacker));
}
