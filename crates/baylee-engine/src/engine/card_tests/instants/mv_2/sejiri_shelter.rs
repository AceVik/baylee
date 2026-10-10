//! `cards/instants/mv_2/sejiri_shelter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn sejiri_shelter_chooses_protection_during_resolution_and_stops_a_red_spell() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        950,
        &[llanowar_elves()],
        &[llanowar_elves()],
        &[sejiri_shelter(), lightning_bolt()],
        &[],
    );
    let elf = kit::obj(&engine, kit::P0, llanowar_elves());
    let other = kit::obj(&engine, kit::P1, llanowar_elves());
    kit::float(
        &mut engine,
        kit::P0,
        &[(ManaColor::White, 2), (ManaColor::Red, 1)],
    );
    let bolt = in_hand(&engine, kit::P0, lightning_bolt()).unwrap();
    cast_with_floating(&mut engine, kit::P0, lightning_bolt());
    kit::aim(&mut engine, &[elf], &[]);
    cast_with_floating(&mut engine, kit::P0, sejiri_shelter());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("target")
    };
    assert!(options.contains(&elf));
    assert!(!options.contains(&other));
    kit::aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(engine.state().object(elf).unwrap().zone, Zone::Battlefield);
    assert_eq!(kit::damage_on(&engine, elf), 0);
    assert!(crate::eval::protected_from(engine.state(), elf, bolt));
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(!crate::eval::protected_from(engine.state(), elf, bolt));
}

#[test]
fn sejiri_glacier_enters_tapped_and_makes_white() {
    let seat = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(sejiri_shelter(), 1).expect("the back face is a land and may be played");

    assert!(
        is_tapped(&engine, land),
        "Sejiri Glacier prints \"this land enters tapped\""
    );

    // A land untaps in its controller's own untap step (CR 502.1), which is
    // the next turn but one — and then it makes white.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && e.state().turn.active == seat && !is_tapped(e, land)
    });
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("a printed mana ability is offered on the land it is printed on");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "`{{T}}: Add {{W}}` puts one white mana in the pool"
    );
    assert!(is_tapped(&engine, land), "and the land is tapped for it");
}
