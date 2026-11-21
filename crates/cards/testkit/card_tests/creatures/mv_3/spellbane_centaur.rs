//! `cards/creatures/mv_3/spellbane_centaur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn spellbane_centaur() -> CardIndex {
    card_index("81de0cb1-6665-41eb-8dfc-bb30192e1359")
}

/// Spellbane Centaur ({2}{G} 3/2 Centaur):
/// "Creatures you control can't be the targets of blue spells or abilities from
/// blue sources."
#[test]
fn spellbane_centaur_protects_your_creatures_from_blue_spells_and_blue_abilities_only() {
    let mut e = lk::game(
        SEED,
        &[spellbane_centaur(), grizzly_bears()],
        &[pearled_unicorn(), prodigal_sorcerer()],
        &[unsummon(), lightning_bolt()],
        &[],
    );
    let centaur = lk::obj(&e, lk::P0, spellbane_centaur());
    let my_bear = lk::obj(&e, lk::P0, grizzly_bears());
    let opp_unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    // Even your own blue Unsummon cannot target your creatures, only opponent's creature!
    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut e, lk::P0, unsummon());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected Unsummon target choice, got {:?}", e.pending());
    };
    assert!(!options.contains(&centaur) && !options.contains(&my_bear));
    assert!(options.contains(&opp_unicorn));
    lk::aim(&mut e, &[opp_unicorn], &[]);
    lk::settle(&mut e);

    // Non-blue spell (Lightning Bolt) CAN target your creatures.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    let Pending::ChooseTargets {
        options: bolt_opts, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert!(bolt_opts.contains(&centaur) && bolt_opts.contains(&my_bear));
    lk::aim(&mut e, &[], &[lk::P1]);
    lk::settle(&mut e);

    // Opponent's blue Prodigal Sorcerer ability cannot target your creatures either.
    reach_their_main_phase(&mut e, lk::P1);
    activate(&mut e, lk::P1, prodigal_sorcerer(), 0);
    let Pending::ChooseTargets {
        options: ping_opts, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert!(!ping_opts.contains(&centaur) && !ping_opts.contains(&my_bear));
    lk::aim(&mut e, &[], &[lk::P0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, lk::P0), 19);
}
