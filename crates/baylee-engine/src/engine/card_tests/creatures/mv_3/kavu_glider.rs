//! `cards/creatures/mv_3/kavu_glider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kavu Glider is a `{2}{R}` 2/1 Kavu with two one-colour activated
/// abilities that each reach only itself: "{W}: This creature gets +0/+1
/// until end of turn" and "{U}: This creature gains flying until end of
/// turn". They are played on separate turns because each price is a single
/// coloured mana and the pool on the turn one is taken holds exactly one
/// mana of that colour — so "the price was paid" is an exact reading rather
/// than a claim about which of several floating sources got spent. The Elf
/// kept standing beside the Glider is the control: `Filter::This` is not "a
/// creature you control", and turning the page between the two activations
/// is what reads "until end of turn", because the toughness is back to the
/// printed 1 by the time the flying is granted.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn kavu_glider_pays_white_for_toughness_and_blue_for_flying_on_itself_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8241, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[kavu_glider(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Mountains pay `{2}{R}` and nothing else, and the Elf is named as
    // the printing kept back: it is this test's control creature, and a
    // source tapped for mana has already changed status for a reason of its
    // own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains tapped and the Elf left standing"
    );
    cast_with_floating(&mut engine, p0, kavu_glider());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{R}} is the whole of the printed cost"
    );

    let glider = on_battlefield(&engine, p0, kavu_glider()).expect("the Glider resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, glider), (2, 1), "the body the card prints");
    assert!(
        !keywords(&engine, glider).contains(KeywordSet::FLYING),
        "and neither ability has been paid for yet"
    );

    // Turn two: the Plains, whose single white is the only white in the pool
    // when the first ability is taken.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, plains());
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the Plains tapped and one white is floating"
    );

    // Ability 0 is "{W}: This creature gets +0/+1 until end of turn."
    activate(&mut engine, p0, kavu_glider(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it uses the stack (CR 605.1)"
    );
    assert_eq!(
        pt(&engine, glider),
        (2, 1),
        "and nothing has happened while it is still on the stack (CR 608.2)"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, glider),
        (2, 2),
        "+0/+1 on itself: the toughness moved and the power did not, which a \
         (3, 2) would deny"
    );
    assert!(
        !keywords(&engine, glider).contains(KeywordSet::FLYING),
        "the white ability grants no keyword"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "the {{W}} was the whole price and it is gone"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "the Elf was not pumped");

    // Turn three: the Island, and with it the other half of "until end of
    // turn" — last turn's +0/+1 did not survive its own cleanup step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, glider),
        (2, 1),
        "\"until end of turn\": the pump lasted the turn it was paid for and \
         no longer"
    );
    play_land(&mut engine, p0, island());
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the Island tapped and one blue is floating"
    );

    // Ability 1 is "{U}: This creature gains flying until end of turn."
    activate(&mut engine, p0, kavu_glider(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, glider).contains(KeywordSet::FLYING),
        "{{U}}: the Glider gains flying"
    );
    assert_eq!(
        pt(&engine, glider),
        (2, 1),
        "and the blue ability moves no numbers at all"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0,
        "the {{U}} was the whole price of the second activation"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "the Elf stays on the ground: `Filter::This` names one creature"
    );
}
