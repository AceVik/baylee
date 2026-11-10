//! `cards/lands/manlands/restless_bivouac.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Bivouac enters tapped and taps for {R} or {W}; for {1}{R}{W} it
/// becomes a 2/2 red and white Ox that is *still a land*, which is what lets
/// a land be declared as an attacker and fire the printed "whenever this land
/// attacks, put a +1/+1 counter on target creature you control".
///
/// The three turns are the card's own and not padding. Turn one is a real
/// `PlayLand`, which is the only way "enters tapped" can be read — a
/// permanent seeded onto the battlefield is placed, not entered. Turn two
/// presses the mana ability alone, because tapping the Bivouac for its own
/// mana is exactly what would leave it unable to attack. Turn three pays
/// {1}{R}{W} out of the lands beside it, so the Bivouac is still untapped when
/// the combat step asks, and the Elf across the table is the control for "you
/// control" in the trigger's target.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn restless_bivouac_animates_into_a_two_two_ox_that_attacks_and_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), plains(), llanowar_elves()])
        .hand(0, &[restless_bivouac()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );

    // Turn one: the land arrives the way a land arrives, and the printed
    // entry has it enter tapped — so its {T} is not on offer at all yet.
    let land = play_land(&mut engine, p0, restless_bivouac());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

    // Turn two: the untap step stands it back up, and the mana ability is the
    // only thing pressed this turn. Tapping the Bivouac here is what would
    // stop it attacking on the turn the animation happens.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "back to p0's own main");
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    activate(&mut engine, p0, restless_bivouac(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "both printed colours are on offer: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else is: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana off the land's own tap, and nothing else on this board was tapped"
    );

    // Turn three: the animation, paid for off the Mountain, the Plains and the
    // Elf beside it, so the Bivouac itself is still untapped for combat.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "back to p0's own main");
    tap_all_mana_but(&mut engine, p0, Some(restless_bivouac()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the Mountain, the Plains and the Elf: exactly what {{1}}{{R}}{{W}} needs"
    );
    activate(&mut engine, p0, restless_bivouac(), 1);
    assert!(
        !stack_is_empty(&engine),
        "becoming a creature is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, land);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::CREATURE),
        "\"becomes a 2/2 red and white Ox creature ... It's still a land\": {kinds:?}"
    );
    assert_eq!(pt(&engine, land), (2, 2), "the 2/2 body the animation sets");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&land),
        "a land the animation turned into a creature is a creature the combat \
         step may declare: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(land, Defender::Player(p1))],
            },
        )
        .expect("the Ox was offered as an attacker");

    // The attack trigger asks for its target on the way to the stack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the target question")
    };
    assert_eq!(player, p0, "the attacking seat aims its own trigger");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are still out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        options.contains(&elves) && options.contains(&land),
        "both creatures you control are on the menu — the Elf and the land \
         the animation turned into one: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elf was one of the options it offered");
    pass_until(&mut engine, |e| {
        counters_on(e, elves, CounterKind::P1P1) == 1
    });
    assert_eq!(
        counters_on(&engine, elves, CounterKind::P1P1),
        1,
        "\"whenever this land attacks, put a +1/+1 counter on target creature \
         you control\" — and it is the land's own attack that put it there"
    );
}
