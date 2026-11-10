//! `cards/enchantments/mv_5/righteous_cause.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Righteous Cause` is an enchantment costing `{3}{W}{W}` under `Coverage::Implemented`.
/// It prints "Whenever a creature attacks, you gain 1 life."
/// When an opponent's creature attacks in their combat phase, the trigger fires and resolves,
/// increasing `Righteous Cause`'s controller's life total by 1.
#[test]
fn righteous_cause_gains_life_whenever_a_creature_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[righteous_cause()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls an Elf");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1, "opponent declares attackers");
    assert!(attackers.contains(&elf), "opponent's Elf can attack");

    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("opponent declaring attack is legal");

    // The attack triggers Righteous Cause. Pass until stack is empty.
    pass_until(&mut engine, |e| e.state().players[0].life > 20);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "Righteous Cause granted 1 life to its controller when a creature attacked"
    );
}
