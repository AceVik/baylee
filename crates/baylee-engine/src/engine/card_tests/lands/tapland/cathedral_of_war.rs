//! `cards/lands/tapland/cathedral_of_war.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cathedral of War: "This land enters tapped." / "Exalted" / "{T}: Add {C}."
/// The land enters tapped when played, and after untapping taps for colorless mana — its
/// second ability, after exalted, in the order the card prints them.
#[test]
fn cathedral_of_war_enters_tapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .hand(0, &[cathedral_of_war()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, cathedral_of_war());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, cathedral_of_war(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Cathedral of War's exalted (CR 702.83a): "Whenever a creature you control attacks alone,
/// that creature gets +1/+1 until end of turn." A lone Llanowar Elves is 2/2 once the trigger
/// resolves and 1/1 again on the next turn; the same Elf attacking beside a Young Wolf is not
/// attacking alone (CR 702.83b, CR 506.5) and nothing triggers.
#[test]
fn cathedral_of_war_exalts_a_lone_attacker_and_no_one_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[cathedral_of_war(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(!stack_is_empty(&engine), "a lone attacker triggers exalted");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (2, 2),
        "the Elf that attacked alone gets +1/+1"
    );
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(pt(&engine, elf), (1, 1), "until end of turn");

    let mut engine = Duel::new(114, forest())
        .battlefield(0, &[cathedral_of_war(), llanowar_elves(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("Young Wolf");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1)), (wolf, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(stack_is_empty(&engine), "two attackers trigger nothing");
    assert_eq!(pt(&engine, elf), (1, 1));
    assert_eq!(pt(&engine, wolf), (1, 1));
}
