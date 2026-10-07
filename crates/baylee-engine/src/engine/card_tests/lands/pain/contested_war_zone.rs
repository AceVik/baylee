//! `cards/lands/pain/contested_war_zone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Contested War Zone is `Coverage::Partial` and this plays the half that
/// exists: it is a land, it taps for `{C}`, and `{1}, {T}` gives attacking
/// creatures +1/+0 until end of turn. The missing clause — a creature that
/// deals combat damage to you taking the land — is not written anywhere, so
/// nothing here pretends to move control of anything.
///
/// The reading that costs a scenario is the word *attacking*: a 1/1 that
/// attacked is a 2/1, while an Elf under the same controller that stayed home
/// and an Elf across the table are both still the 1/1s they were printed as.
/// The `{1}` is a real payment — the pool is filled before the ability is
/// claimed and one mana is gone afterwards — and the land is kept standing
/// because its own `{T}` is half of what the ability charges.
#[test]
fn contested_war_zone_pumps_the_creatures_that_attacked_and_only_those() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[contested_war_zone()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A land arrives by the land drop and not by being seeded: seeded
    // permanents are placed rather than played, and everything this card does
    // hangs off the permanent that arrived.
    play_land(&mut engine, p0, contested_war_zone());
    let zone = on_battlefield(&engine, p0, contested_war_zone()).expect("the land is out");
    assert_eq!(
        engine.state().object(zone).map(|o| o.controller),
        Some(p0),
        "and it belongs to the seat that played it"
    );
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays home");
    let (attacker, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // "Attacking creature" is a state the combat step hands out, so the
    // scenario starts at the declaration that puts a creature in it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&attacker),
        "an untapped 1/1 with no summoning sickness may attack: {attackers:?}"
    );
    let defender = defenders
        .into_iter()
        .next()
        .expect("the one opponent is attackable");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, defender)],
            },
        )
        .expect("the Elf was on the offer of who may attack");

    // Mana into the pool before the claim, and the War Zone named as the one
    // source kept standing: its `{T}` is what the ability charges, so tapping
    // it for `{C}` would remove the very half under test. CR 500.5 leaves the
    // pool through this step.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, Some(contested_war_zone()));
    let pool = engine.state().players[0].mana_pool.total();
    assert!(pool >= 1, "the Forests and the Elves paid into the pool");

    // Ability 0 is the printed `{T}: Add {C}`; ability 1 is the pump.
    activate(&mut engine, p0, contested_war_zone(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, attacker),
        (2, 1),
        "\"Attacking creatures get +1/+0\" — the creature that attacked"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf under the same controller that stayed home is not attacking"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and neither is the creature across the table that never attacked"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool - 1,
        "the {{1}} came out of the pool the lands and Elves filled"
    );
    assert!(
        on_battlefield(&engine, p0, contested_war_zone()).is_some(),
        "the land is still on the table: the pump is the ability and not its source"
    );
}
