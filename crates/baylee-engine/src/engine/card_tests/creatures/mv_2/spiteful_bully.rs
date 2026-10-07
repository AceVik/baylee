//! `cards/creatures/mv_2/spiteful_bully.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spiteful Bully — {1}{B} 3/3 — "At the beginning of your upkeep, this
/// creature deals 3 damage to target creature you control."
///
/// The trigger is the whole card, so the Bully is cast and a full turn cycle
/// is walked to its controller's next upkeep, where the three damage has to
/// land on the creature the question offered. "You control" is read on both
/// sides of the table — the Elf across it is a creature and must stay off the
/// menu — and "creature you control" is not "another creature", so the Bully
/// itself is on that menu too. The Elf under the same seat dying while the
/// Bully stands is the repeat the card is played for (CR 704.5g).
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn spiteful_bully_shoots_a_creature_you_control_at_your_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[spiteful_bully()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{B} off the Forest and the Swamp. The Elf is named as the printing
    // kept back because it is the creature the upkeep trigger will aim at,
    // and it is a mana creature `tap_all_mana` would otherwise have spent.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the Forest and the Swamp, and the Elf left standing"
    );
    cast_with_floating(&mut engine, p0, spiteful_bully());
    pass_until(&mut engine, stack_is_empty);

    let bully = on_battlefield(&engine, p0, spiteful_bully()).expect("the Bully resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, bully), (3, 3), "the body the card prints");

    // The trigger belongs to its controller's own upkeep, so a whole turn
    // cycle has to pass — and the combat declarations are answered on the way,
    // because the question arrives before any priority round of that turn.
    let mut offered: Vec<ObjectId> = Vec::new();
    for _ in 0..400 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the Bully's controller is asked");
                offered = options;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand.into_iter().take(usize::from(count)).collect(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected on the way to the next upkeep: {other:?}"),
        }
    }

    assert!(
        offered.contains(&mine),
        "the Elf under my own control is on the menu: {offered:?}"
    );
    assert!(
        offered.contains(&bully),
        "\"target creature you control\" is not \"another creature\": the \
         Bully's own body is a legal target for its trigger: {offered:?}"
    );
    assert!(
        !offered.contains(&theirs),
        "the Elf across the table is a creature and still no legal target, \
         which is the whole of `Filter::YOUR_CREATURE`: {offered:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "three damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and the creature the trigger named left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, spiteful_bully()).is_some(),
        "the Bully outlives its own upkeep, which is what makes the damage a \
         repeated price and not a one-shot"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the creature the trigger did not name never moved"
    );
}
