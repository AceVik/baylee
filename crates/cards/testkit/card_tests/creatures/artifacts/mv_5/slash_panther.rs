//! `cards/creatures/artifacts/mv_5/slash_panther.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn slash_panther() -> CardIndex {
    card_index("76558381-c4ae-4bc3-8c7b-fa0b717f0282")
}

/// Slash Panther — {4}{R/P}, haste. With a Mountain the Phyrexian symbol can
/// be paid in red mana, and the Panther, hasty, attacks the turn it enters.
#[test]
fn slash_panther_has_haste_and_attacks_the_turn_it_comes_down() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[slash_panther()])
        .battlefield(0, &[mountain(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, slash_panther());
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine
            .apply(p0, PlayerAction::YesNo(false))
            .expect("pay the mana, not life");
    }
    pass_until(&mut engine, stack_is_empty);
    let panther = on_battlefield(&engine, p0, slash_panther()).expect("resolved");
    assert_eq!(pt(&engine, panther), (4, 2));
    assert!(is_artifact_creature(&engine, panther));
    assert!(keywords(&engine, panther).contains(KeywordSet::HASTE));
    assert_eq!(life_of(&engine, 0), 20, "mana paid, no life");

    attack_alone(&mut engine, p0, panther, p1);
    through_combat(&mut engine);
    assert_eq!(
        life_of(&engine, 1),
        16,
        "it hit for four on the turn it entered"
    );
}

/// "{R/P} can be paid with either {R} or 2 life": with no red source at all
/// the symbol is paid with 2 life (CR 107.4f).
#[test]
fn slash_panther_can_be_paid_with_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[slash_panther()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, slash_panther());
    while matches!(engine.pending(), Pending::YesNo { .. }) {
        engine
            .apply(p0, PlayerAction::YesNo(true))
            .expect("pay life");
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, slash_panther()).is_some());
    assert_eq!(life_of(&engine, 0), 18, "2 life paid the {{R/P}}");
}
