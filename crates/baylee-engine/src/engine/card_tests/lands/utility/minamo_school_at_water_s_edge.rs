//! `cards/lands/utility/minamo_school_at_water_s_edge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "17784f90-89a1-47a5-83ef-ae60dfc30bd1"

/// Minamo, School at Water's Edge is a legendary land printing two lines:
/// "{T}: Add {U}" and "{U}, {T}: Untap target legendary permanent."
///
/// The second line's price is paid by the permanent the effect is about, so
/// aiming it at the School itself is the one reading that tells a real untap
/// from an ability whose cost was never collected: the card is tapped the
/// moment the target is answered (CR 601.2h) and standing again once the
/// ability resolves. The menu is the filter's other half — the Elf and the two
/// Islands are permanents a bare `Filter::Any` would have offered and none of
/// them is legendary, while the Ring across the table is offered, because the
/// printed sentence says "target legendary permanent" and not "you control".
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn minamo_untaps_a_legendary_permanent_for_one_blue_and_its_own_tap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[the_one_ring()])
        .hand(0, &[minamo_school_at_water_s_edge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A land has no summoning sickness (CR 302.6), so both printed lines are
    // live the turn the School arrives.
    let school = play_land(&mut engine, p0, minamo_school_at_water_s_edge());
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let islands = all_on_battlefield(&engine, p0, island());
    let ring = on_battlefield(&engine, p1, the_one_ring()).expect("the Ring is out");
    assert_eq!(islands.len(), 2, "two Islands stand under the School");
    assert!(!is_tapped(&engine, school), "and it enters untapped");

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // rather than the untapped lands: on an empty pool the {U} is unpayable
    // and the untap is absent from the offer altogether.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(school, 0)),
        "{{T}}: Add {{U}} costs its own tap and nothing else: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(school, 1)),
        "{{U}} is not one, so the untap is unpayable and absent from the \
         offer: {:?}",
        legal.abilities
    );

    // Mana into the pool first, with the School named as the source kept back:
    // its printed `{T}: Add {U}` is a mana route whose whole price is its own
    // tap (#159), and `tap_all_mana` would have spent the very activation
    // this test is about.
    tap_all_mana_but(&mut engine, p0, Some(minamo_school_at_water_s_edge()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "the two Islands"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the Elf's own {{G}}, because a mana creature is a route too"
    );
    assert!(
        !is_tapped(&engine, school),
        "the School is the one source kept standing"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(school, 1)),
        "with the {{U}} in the pool the untap is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, minamo_school_at_water_s_edge(), 1);
    let menu = aim_at(&mut engine, p0, school);

    // CR 601.2c names the target and CR 601.2h pays afterwards, so the tap and
    // the blue are read here rather than before the answer.
    assert!(
        is_tapped(&engine, school),
        "{{T}} is half the price, paid by the permanent the target names"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "and the other half is the {{U}} the pool was carrying"
    );
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so the ability is on the stack"
    );
    assert!(
        menu.contains(&school),
        "the School is itself a legendary permanent: {menu:?}"
    );
    assert!(
        menu.contains(&ring),
        "\"target legendary permanent\" prints no \"you control\", so the Ring \
         across the table is offered: {menu:?}"
    );
    assert!(
        !menu.contains(&elf),
        "the Elf is a creature and no legendary one: {menu:?}"
    );
    assert!(
        !islands.iter().any(|id| menu.contains(id)),
        "and the Islands are lands and no legendary ones: {menu:?}"
    );
    assert_eq!(
        menu.len(),
        2,
        "the two legends on the table are the whole menu: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, school),
        "\"Untap target legendary permanent\": the very permanent that paid \
         the {{T}} is standing again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one blue and one green are left: the {{U}} is spent and the Elf's \
         {{G}} was never part of the price"
    );

    // And the standing School prints blue the way its first line says, which
    // is what the untap bought.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(school, 0)),
        "a School standing untapped again has its {{T}} back: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, minamo_school_at_water_s_edge(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, school), "{{T}}: Add {{U}}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "the tap the untap restored, spent on the printed mana line"
    );
}
