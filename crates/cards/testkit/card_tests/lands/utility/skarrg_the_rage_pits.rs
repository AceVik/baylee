//! `cards/lands/utility/skarrg_the_rage_pits.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "92bac34e-2045-4331-842f-185711c1ac56"

/// Skarrg, the Rage Pits prints two lines: "{T}: Add {C}" and "{R}{G}, {T}:
/// Target creature gets +1/+1 and gains trample until end of turn."
/// The pump is the half a board can argue about, so a second Elf of mine and
/// an Elf across the table stand beside the one it names — "target creature"
/// reaches any creature on either side and modifies only the one that was
/// chosen. The {R}{G} is a real payment out of a pool the Forest, the Mountain
/// and the two Elves filled, and reading the price while the target question
/// is still open (CR 601.2c before CR 601.2h) says the tap and the two mana are
/// both unpaid there. The turn is then walked to an end because "until end of
/// turn" is printed, and the land's other line is played on the turn its {T}
/// is payable again, so both printed abilities are in the game rather than in
/// the card file.
#[test]
#[allow(clippy::too_many_lines)]
fn skarrg_the_rage_pits_pumps_the_creature_it_names_and_only_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                skarrg_the_rage_pits(),
                forest(),
                mountain(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skarrg = on_battlefield(&engine, p0, skarrg_the_rage_pits())
        .expect("Skarrg, the Rage Pits is on the battlefield");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves of mine, one of which stays the control"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the pump");
    assert!(
        !is_tapped(&engine, skarrg),
        "and nothing has tapped the land"
    );

    // The land's own {T} is half the pump's price, so the mana helper is told
    // to leave it standing: everything else on the board is a source whose
    // whole price is its own tap — the Forest and the Mountain by CR 305.6,
    // the two Elves by the printed ability they carry (#159).
    tap_all_mana_but(&mut engine, p0, Some(skarrg_the_rage_pits()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "one Forest, one Mountain and two Elves tapped for mana"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain's {{R}}, the only red source on this board"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "the Forest and both Elves"
    );

    // Ability 0 is the printed "{T}: Add {C}"; ability 1 is the pump.
    activate(&mut engine, p0, skarrg_the_rage_pits(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&skarrg),
        "the land is a land and no creature, and cannot aim its own ability at itself: {options:?}"
    );

    // CR 601.2c before CR 601.2h: while the question stands the whole price is
    // still unpaid, and both halves of it are read here.
    assert!(
        !is_tapped(&engine, skarrg),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{R}}{{G}} is still floating for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    assert!(
        is_tapped(&engine, skarrg),
        "{{T}} is half the price and is paid with the answer"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "and the {{R}}{{G}} came out of the pool: the Mountain's red is gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "four floating less the two the ability charges"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the ability named"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "and the printed trample reaches it through the layers"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the pump targets, it does not sweep the board"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it never reaches across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "\"target creature\" is not \"creatures\", on either side of the table"
    );
    assert!(
        !keywords(&engine, skarrg).contains(KeywordSet::TRAMPLE),
        "and the land that granted the keyword keeps none of it"
    );

    // "until end of turn" is printed, so the turn is walked to an end: a grant
    // that never expired would satisfy every assertion above it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert_eq!(pt(&engine, host), (1, 1), "and the +1/+1 went with it");
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature is still standing, so the keyword left rather than the creature"
    );

    // The land's other printed line, on the turn its {T} is payable again.
    assert!(
        !is_tapped(&engine, skarrg),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );
    activate(&mut engine, p0, skarrg_the_rage_pits(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — the half of the card that asks for no colour and no mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana, off one tap"
    );
    assert!(
        is_tapped(&engine, skarrg),
        "and the land paid its own {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
