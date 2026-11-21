//! `cards/enchantments/mv_3/wave_of_terror.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn wave_of_terror() -> CardIndex {
    card_index("2346be19-7344-4f11-be2c-a22750cb7e1d")
}

fn pay_upkeep_mana(e: &mut Engine<RegistryLookup>, p0: PlayerId, colorless: u32) {
    pass_until(e, |e| {
        e.state().turn.active == p0
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { .. },
                    ..
                }
            )
    });
    lk::float(e, p0, &[(ManaColor::Colorless, colorless)]);
    lk::yes_no(e, true);
}

/// Cumulative upkeep `{1}` + Draw-step destruction by age counter MV (ignoring
/// regeneration, sparing other MVs) and sacrificing when cumulative upkeep is declined.
#[test]
fn wave_of_terror_destroys_matching_mv_without_regen_and_respects_cumulative_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[lotleth_troll()],
        &[
            llanowar_elves(),
            legends_ids::crimson_kobolds(),
            grizzly_bears(),
        ],
        &[wave_of_terror()],
        &[],
    );
    let troll = lk::obj(&e, p0, lotleth_troll());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());
    let opp_kobolds = lk::obj(&e, p1, legends_ids::crimson_kobolds());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut e, p0, wave_of_terror(), &[], &[]);
    let wave = lk::obj(&e, p0, wave_of_terror());

    pay_upkeep_mana(&mut e, p0, 1);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::FirstMain);
    assert_eq!(lk::zone_of(&e, opp_elves), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, opp_kobolds), Zone::Battlefield);
    assert_eq!(lk::zone_of(&e, troll), Zone::Battlefield);

    // Age 2: regenerate Lotleth Troll before Draw step; Wave ignores regeneration.
    pay_upkeep_mana(&mut e, p0, 2);
    pass_until(&mut e, |e| {
        e.state().turn.step == baylee_engine::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    lk::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    activate(&mut e, p0, lotleth_troll(), 1);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::FirstMain);
    assert_eq!(lk::zone_of(&e, troll), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, opp_bear), Zone::Graveyard);

    // Age 3: decline cumulative upkeep -> sacrificed before Draw step.
    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { .. },
                    ..
                }
            )
    });
    lk::yes_no(&mut e, false);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::FirstMain);
    assert_eq!(lk::zone_of(&e, wave), Zone::Graveyard);
}
