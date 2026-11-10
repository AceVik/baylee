//! `cards/instants/mv_2/gerrard_s_command.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gerrard's Command — {G}{W} instant: "Untap target creature. It gets +3/+3
/// until end of turn."
///
/// One target carries both printed sentences, so the board is built to show
/// they land on the creature the question named and on nothing else: two
/// Llanowar Elves under p0 are tapped for the mana that pays for the spell
/// (so the one it untaps is genuinely tapped rather than untapped anyway), and
/// a third Elf stands across the table because "target creature" (CR 115.1)
/// reaches all three. Only the Elf that was answered for comes back up and
/// reads 4/4 — an untap without the pump would leave it a 1/1, and a pump
/// without the untap would leave it lying down.
#[test]
fn gerrards_command_untaps_and_pumps_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[gerrards_command()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays out of it");
    let (named, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // Every mana source on the board, counted rather than assumed: the Forest
    // and the Plains, plus each Elf's own printed `{T}: Add {G}` — which is
    // what leaves the creature the spell is about to untap lying down.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "one green and one white off the lands, and two more green off the Elves"
    );
    assert!(
        is_tapped(&engine, named) && is_tapped(&engine, bystander),
        "both Elves paid for the spell with their own tap"
    );
    assert_eq!(pt(&engine, named), (1, 1), "a printed 1/1 while tapped");

    cast_with_floating(&mut engine, p0, gerrards_command());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it is the seat that aims it");
    assert!(
        options.contains(&named) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "\"target creature\" names no player (CR 115.1): {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![named],
            },
        )
        .expect("the creature the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, named),
        "\"untap target creature\" — the Elf that had just paid for the spell"
    );
    assert_eq!(
        pt(&engine, named),
        (4, 4),
        "+3/+3 on the creature the spell named, and nothing else"
    );
    assert!(
        is_tapped(&engine, bystander),
        "the Elf nobody named stays exactly where it tapped"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and unpumped — the effect does not read \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor does it reach across the table, however wide the target filter is"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}}{{W}} came out of the pool, and the two green left are all that remains"
    );
}
