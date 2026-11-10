//! `cards/enchantments/auras/mv_2/psychic_venom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Psychic Venom: "Enchant land" / "Whenever enchanted land becomes tapped,
/// this Aura deals 2 damage to that land's controller." Tapping the
/// enchanted land for mana costs its controller 2 life.
#[test]
fn psychic_venom_deals_two_damage_when_the_enchanted_land_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let venom = psychic_venom();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[forest()])
        .hand(0, &[venom])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p1, forest()).expect("their land");

    cast_from_hand(&mut engine, p0, venom);
    aim_at(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "2 damage when the enchanted land became tapped"
    );
    assert_eq!(engine.state().players[0].life, 20);
}
