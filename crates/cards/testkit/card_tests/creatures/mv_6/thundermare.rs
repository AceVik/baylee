//! `cards/creatures/mv_6/thundermare.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thundermare() -> CardIndex {
    card_index("58fa33fd-02a4-4ec9-9226-82594cfec363")
}

/// When Thundermare enters, it taps every other creature on both sides while
/// staying untapped itself (and leaving noncreatures untapped), then attacks
/// immediately with haste past the tapped defenders for 5.
#[test]
fn thundermare_taps_all_other_creatures_on_etb_and_attacks_immediately_with_haste() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
                sol_ring(),
            ],
        )
        .battlefield(1, &[craw_wurm(), gray_ogre()])
        .hand(0, &[thundermare()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let own_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let own_ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let opp_wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();
    let opp_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    let lands = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| lands.contains(&id));
    let spell = in_hand(&engine, p0, thundermare()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let horse = on_battlefield(&engine, p0, thundermare()).expect("resolved");
    assert_eq!(pt(&engine, horse), (5, 5));
    assert!(keywords(&engine, horse).contains(KeywordSet::HASTE));
    assert!(!is_tapped(&engine, horse), "Thundermare stays untapped");
    assert!(
        is_tapped(&engine, own_bears),
        "your other creatures tap too"
    );
    assert!(is_tapped(&engine, opp_wurm), "opposing creatures tap");
    assert!(is_tapped(&engine, opp_ogre), "all opposing creatures tap");
    assert!(
        !is_tapped(&engine, own_ring),
        "noncreature artifacts stay untapped"
    );

    // Haste lets Thundermare attack this turn, and tapped blockers cannot block.
    let blocks = attack_and_collect_blocks(&mut engine, horse, p1);
    assert!(blocks.is_empty(), "tapped creatures cannot block");
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 15);
}
