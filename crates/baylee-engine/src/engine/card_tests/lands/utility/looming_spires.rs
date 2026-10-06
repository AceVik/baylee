//! `cards/lands/utility/looming_spires.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Looming Spires is a land that enters tapped, and its only other printed
/// text is a targeted enters-trigger: "target creature gets +1/+1 and gains
/// first strike until end of turn". Two Elves of mine and one across the
/// table make the word *target* mean something — the filter has no side on it,
/// so all three are on the menu, and the two that are not named stay printed
/// 1/1s with no first strike. The land itself is no creature and may not be
/// named by its own trigger; and because it entered tapped its `{T}: Add {R}`
/// is read a turn later, where the untap step has stood it up, the Elves are
/// kept out of the tapping, and one red with nothing beside it can only have
/// come off the land.
#[test]
#[allow(clippy::too_many_lines)]
fn looming_spires_enters_tapped_and_pumps_the_one_creature_its_trigger_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[looming_spires()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves of mine, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the land");

    let land = play_land(&mut engine, p0, looming_spires());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is a real entry and not a placement"
    );

    // The trigger names its target as it is put on the stack, and "target
    // creature" is a filter with no side of the table in it.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, p0, "the land's controller names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the trigger asks once"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "the land is no creature and no target for its own trigger: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the trigger named"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike reaches it through the layers"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is untouched: the trigger targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "and gains no first strike for standing beside the one that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the effect reaches one creature and never the board across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor across the table"
    );

    // The other half of "enters tapped": its `{T}` is spent, so the land
    // offers nothing at all while the turn it arrived in is still running.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat that played the land");
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land)
            && !legal.mana_abilities.contains(&land),
        "a land that entered tapped has no untapped {{T}} to offer: {:?} / {:?}",
        legal.abilities,
        legal.mana_abilities
    );

    // A land that entered tapped gives nothing this turn, so the mana line is
    // read on the next one — and the pump's printed duration is read with it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land back up"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "\"until end of turn\": the pump lasted the turn it was made in and no longer"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and the first strike left with it, with the creature still standing"
    );

    // Both Elves are named as the printing kept back: each prints a `{T}: Add
    // {G}` of its own, and the one land on this board is what this half is
    // about — so one red in the pool has no other source it could come from.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "{{T}}: Add {{R}} — one red, and nothing on this board but the land makes it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        !is_tapped(&engine, host) && !is_tapped(&engine, bystander),
        "and both Elves were kept out of the tapping, so neither paid for it"
    );
}
