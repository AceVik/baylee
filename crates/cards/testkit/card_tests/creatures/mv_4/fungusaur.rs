//! `cards/creatures/mv_4/fungusaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fungusaur — "Whenever this creature is dealt damage, put a +1/+1
/// counter on it." One point of (non-lethal) combat damage grows it.
#[test]
fn fungusaur_gains_a_counter_whenever_it_is_dealt_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fungusaur()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let dino = on_battlefield(&engine, p0, fungusaur()).expect("seated");
    assert_eq!(pt(&engine, dino), (2, 2));

    reach_their_main_phase(&mut engine, p1);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, elf, p0);
    let pairing = blocks
        .iter()
        .find(|b| b.blocker == dino)
        .unwrap_or_else(|| panic!("Fungusaur may block the Elf: {blocks:?}"));
    assert!(pairing.attackers.contains(&elf));
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(dino, elf)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().object(dino).map(|o| (o.zone, o.damage)),
        Some((Zone::Battlefield, 1)),
        "the Elf's 1 damage was not lethal to a 3-toughness Fungusaur"
    );
    assert_eq!(counters_on(&engine, dino, CounterKind::P1P1), 1);
    assert_eq!(
        pt(&engine, dino),
        (3, 3),
        "the printed 2/2 plus the counter"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "2 power killed the 1-toughness Elf"
    );
}
