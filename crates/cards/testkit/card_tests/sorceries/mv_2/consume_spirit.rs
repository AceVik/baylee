//! `cards/sorceries/mv_2/consume_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn consume_spirit() -> CardIndex {
    card_index("861fa80d-99e0-4332-a2b8-5aa959fd41a4")
}

/// Consume Spirit — {X}{1}{B} Sorcery
/// "Spend only black mana on X."
/// "Consume Spirit deals X damage to any target and you gain X life."
#[test]
fn consume_spirit_deals_x_damage_and_gains_x_life_using_only_black_mana_for_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[consume_spirit()])
        .battlefield(0, &[mountain(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap 1 Mountain + 4 Swamps ({R}{B}{B}{B}{B}).
    // Generic {1} uses {R}, {B} uses one {B}, leaving 3 {B} for X=3.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, consume_spirit());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 23);
}
