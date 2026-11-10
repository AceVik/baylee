//! `cards/creatures/mv_1/dega_disciple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dega Disciple is a 1/1 for {W} printing two abilities that differ in
/// exactly three things — the colour of the mana, the sign and the size:
/// "{B}, {T}: Target creature gets -2/-0 until end of turn" and "{R}, {T}:
/// Target creature gets +2/+0 until end of turn."
///
/// Both are played in one main phase, which is the only way to tell them
/// apart: two Disciples stand on the board so each pays its own `{T}`, a
/// Swamp and a Mountain put exactly one `{B}` and one `{R}` in the pool, and
/// the two are aimed at different creatures — the big body across the table
/// for the black one and my own 1/1 for the red one. A single creature
/// pumped by both would net back out to its printed body, and a swapped sign
/// or a swapped colour would be invisible in the card file and unmissable
/// here.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn dega_disciple_shrinks_with_black_and_grows_with_red_each_aimed_at_its_own_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                dega_disciple(),
                dega_disciple(),
                swamp(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm =
        on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is across the table");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let disciples = all_on_battlefield(&engine, p0, dega_disciple());
    assert_eq!(disciples.len(), 2, "one Disciple per printed ability");
    let (black, red) = (disciples[0], disciples[1]);
    let (wurm_power, wurm_toughness) = pt(&engine, wurm);
    let (elf_power, elf_toughness) = pt(&engine, elves);
    assert!(
        wurm_power > 2,
        "a body big enough that -2/-0 reads as -2 and not as a clamp at zero"
    );

    // Both prices are mana plus the source's own tap (CR 601.2h), and a
    // Disciple prints no mana ability of its own — so the pool is filled
    // first and both Disciples stay standing, which is what makes the offer
    // below mean anything.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in &disciples {
        assert!(
            legal.abilities.contains(&(*source, 0)),
            "a Swamp in the pool pays the black line: {:?}",
            legal.abilities
        );
        assert!(
            legal.abilities.contains(&(*source, 1)),
            "and a Mountain pays the red one: {:?}",
            legal.abilities
        );
    }

    // {B}, {T} — the target is announced first (CR 601.2c) ...
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: black,
                ability_index: 0,
            },
        )
        .expect("the black line was offered");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&wurm) && options.contains(&elves),
        "any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    // ... and the price only afterwards, which is where the tap and the {B} go.
    assert!(
        is_tapped(&engine, black),
        "the Disciple that activated paid its own {{T}}"
    );
    assert!(!is_tapped(&engine, red), "and the other one did not");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "the {{B}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wurm),
        (wurm_power - 2, wurm_toughness),
        "{{B}}, {{T}}: -2/-0 — two power off and no toughness at all"
    );
    assert_eq!(
        pt(&engine, elves),
        (elf_power, elf_toughness),
        "and the black line reached the creature it named and no other"
    );

    // {R}, {T} — the other Disciple, the other colour, the other creature.
    assert!(
        matches!(engine.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(red, 1))),
        "the red line is still offered on the Disciple that is still standing"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: red,
                ability_index: 1,
            },
        )
        .expect("the red line was offered");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves) && options.contains(&wurm),
        "the same menu, read before the {{R}} is spent: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "the {{R}} is what the red line charges"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elves),
        (elf_power + 2, elf_toughness),
        "{{R}}, {{T}}: +2/+0 — two power on and no toughness"
    );
    assert_eq!(
        pt(&engine, wurm),
        (wurm_power - 2, wurm_toughness),
        "the Wurm is still only as small as the black line made it: the red \
         one never pointed at it"
    );
    assert!(
        is_tapped(&engine, red),
        "and the second Disciple has now spent its tap too"
    );
}
