//! `cards/instants/mv_1/enrage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Enrage — `{X}{R}` instant: "Target creature gets +X/+0 until end of turn."
///
/// One casting asks the two questions CR 601.2 puts before the payment, and the
/// board is built so each answer is worth reading: X stops at what the pool
/// pays (three Mountains, so 2 — the Elf beside them is named as the source kept
/// back, because a mana creature counted into the pool would move that number),
/// and the target menu holds the Elf *across* the table as well as the one this
/// side, which is "target creature" and not "target creature you control",
/// while the Sol Ring over there is no creature at all. `(3, 1)` on a printed
/// 1/1 is the only body that reads both halves of the pump, and the same Elf
/// reading `(1, 1)` again on the opponent's turn is what makes "until end of
/// turn" a duration rather than a counter.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn enrage_pumps_the_target_by_x_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), quiet_artifact()])
        .hand(0, &[enrage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // The Elf is named as the source kept back: its whole price is its own
    // `{T}`, so `tap_all_mana` would have drunk it too (#159) and the three
    // red X is measured against would have been four.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains tapped and the Elf still standing"
    );

    cast_with_floating(&mut engine, p0, enrage());

    // CR 601.2b then CR 601.2c, before a single mana is spent (CR 601.2h). The
    // two questions are answered in whichever order they arrive rather than in
    // the order they are expected: a spell that asked only one of them would
    // otherwise look the same as one that asked the other first.
    let mut named_x = false;
    let mut aimed = false;
    for _ in 0..20 {
        if named_x && aimed && at_rest(&engine, p0) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert_eq!(player, p0, "the caster names X");
                // The resource bound can overestimate the X that remains
                // after fixed costs and later choices. The legal two must
                // be offered; the actual payment is checked below.
                assert_eq!(min, 0, "nothing is a legal X");
                assert!(max >= 2, "the two the Mountains pay is offered: {max}");
                engine
                    .apply(p0, PlayerAction::ChooseNumber(2))
                    .expect("the value the question itself offered");
                named_x = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the caster aims it");
                assert_eq!((min, max), (1, 1), "exactly one creature");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target creature\" is any creature, on either side of the \
                     table: {options:?}"
                );
                assert!(
                    !options.contains(&rock),
                    "the Sol Ring is an artifact and no creature: {options:?}"
                );
                assert_eq!(options.len(), 2, "and those two are the whole menu");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .expect("the Elf the question offered was chosen");
                aimed = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Enrage is being cast: {other:?}"),
        }
    }
    assert!(
        named_x && aimed,
        "the cast asked for both its value and its target"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{R}} with X two is the three Mountains, and they are spent"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+X/+0: the two the caster named, and the toughness the card never pumps"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );

    // The duration. The opponent's main phase lies past this turn's cleanup
    // step, which is where "until end of turn" ends — an Elf still reading
    // `(3, 1)` there would be a counter wearing a one-shot's name.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"until end of turn\": the +2 is gone with the turn that paid for it"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the creature nobody aimed at was never anything but a 1/1"
    );
}
