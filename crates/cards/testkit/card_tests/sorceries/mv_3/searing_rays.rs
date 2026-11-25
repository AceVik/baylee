#[allow(clippy::wildcard_imports)]
use super::*;

fn searing_rays() -> CardIndex {
    card_index("fd6730e2-3904-43cf-90c6-e3d9fe210bc5")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn cast_and_choose_color(engine: &mut Engine<RegistryLookup>, color: ManaColor) {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, searing_rays());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 5, "all five colors are offered");
    engine.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
    pass_until(engine, stack_is_empty);
}

#[test]
fn searing_rays_counts_the_chosen_color_for_each_player_including_multicolor_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                gray_ogre(),
                lightning_angel(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[searing_rays()])
        .battlefield(1, &[gray_ogre(), gray_ogre(), gray_ogre(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_choose_color(&mut engine, ManaColor::Red);

    assert_eq!(
        engine.state().players[0].life,
        18,
        "Gray Ogre and the red-white-blue Lightning Angel are two red creatures"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "three Gray Ogres are three red creatures; Grizzly Bears is not red"
    );
    assert!(in_graveyard(&engine, p0, searing_rays()).is_some());
}

#[test]
fn searing_rays_deals_zero_damage_to_a_player_with_no_creatures_of_that_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), lightning_angel()])
        .hand(0, &[searing_rays()])
        .battlefield(1, &[gray_ogre(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_choose_color(&mut engine, ManaColor::Blue);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "Lightning Angel is blue as well as white and red"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "neither the red Ogre nor the green Bears is blue"
    );
}
