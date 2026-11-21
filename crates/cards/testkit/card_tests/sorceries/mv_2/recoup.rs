//! `cards/sorceries/mv_2/recoup.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn recoup() -> CardIndex {
    card_index("f80d151c-7489-4189-8c5b-6d2a730d238e")
}

fn salvage() -> CardIndex {
    card_index("c01a090a-11e4-469e-aded-74203c06fef4")
}

/// Recoup (f80d151c-7489-4189-8c5b-6d2a730d238e) — {1}{R} Sorcery:
/// "Target sorcery card in your graveyard gains flashback until end of turn.
/// The flashback cost is equal to its mana cost. Flashback {3}{R}."
#[test]
fn recoup_grants_flashback_only_to_own_sorcery_in_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), forest()],
        &[],
        &[recoup(), salvage(), counterspell(), llanowar_elves()],
        &[salvage()],
    );
    let gy_sorcery = hand_to_graveyard(&mut engine, P0, salvage());
    let gy_instant = hand_to_graveyard(&mut engine, P0, counterspell());
    let gy_creature = hand_to_graveyard(&mut engine, P0, llanowar_elves());
    let opp_gy_sorcery = hand_to_graveyard(&mut engine, P1, salvage());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, recoup());
    let menu = aim_at(&mut engine, P0, gy_sorcery);
    assert_eq!(menu, vec![gy_sorcery], "only sorcery cards in your gy");
    assert!(
        !menu.contains(&gy_instant)
            && !menu.contains(&gy_creature)
            && !menu.contains(&opp_gy_sorcery)
    );
    lk::settle(&mut engine);

    // Salvage ({G}) now has flashback {G} until end of turn; cast it targeting gy_creature!
    engine
        .apply(P0, PlayerAction::CastSpell { card: gy_sorcery })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    aim_at(&mut engine, P0, gy_creature);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, gy_sorcery), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, gy_creature), Zone::Library);
}

/// Recoup — Flashback {3}{R} exiles Recoup and granted flashback expires EOT.
#[test]
fn recoup_flashback_exiles_recoup_and_granted_flashback_expires_at_eot() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain(), forest()],
        &[],
        &[recoup(), salvage(), llanowar_elves()],
        &[],
    );
    let recoup_gy = hand_to_graveyard(&mut engine, P0, recoup());
    let gy_sorcery = hand_to_graveyard(&mut engine, P0, salvage());
    let _gy_elf = hand_to_graveyard(&mut engine, P0, llanowar_elves());

    tap_all_mana(&mut engine, P0);
    engine
        .apply(P0, PlayerAction::CastSpell { card: recoup_gy })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    aim_at(&mut engine, P0, gy_sorcery);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, recoup_gy), Zone::Exile);

    // Let the turn end without casting Salvage -> granted flashback expires on Turn 3!
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    tap_all_mana(&mut engine, P0);
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: gy_sorcery })
            .is_err(),
        "granted flashback expired at end of turn"
    );
}
