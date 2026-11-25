//! `cards/instants/mv_2/winnow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn winnow() -> CardIndex {
    card_index("1b2ad355-3f1e-4c8b-93ee-848934646b23")
}

fn hand_len(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(player)).len()
}

fn library_len(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(player))
        .len()
}

fn game_with(winnows: u32, theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut hand = vec![winnow()];
    hand.extend((1..winnows).map(|_| winnow()));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, theirs)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

fn assert_draws_top_card(
    engine: &Engine<RegistryLookup>,
    player: PlayerId,
    top: ObjectId,
    hand_before: usize,
    library_before: usize,
) {
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(player))
            .contains(&top)
    );
    assert_eq!(hand_len(engine, player), hand_before, "one out, one in");
    assert_eq!(library_len(engine, player), library_before - 1);
}

#[test]
fn winnow_destroys_only_one_of_two_same_named_permanents_and_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = game_with(1, &[grizzly_bears(), grizzly_bears()]);
    let bears = all_on_battlefield(&engine, p1, grizzly_bears());
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library.last().unwrap();
    let (hand_before, library_before) = (hand_len(&engine, p0), library.len());

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, winnow());
    aim_at(&mut engine, p0, bears[0]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears[0])
            .is_none_or(|o| o.zone != Zone::Battlefield)
    );
    assert!(
        engine
            .state()
            .object(bears[1])
            .is_some_and(|o| o.zone == Zone::Battlefield)
    );
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_draws_top_card(&engine, p0, top, hand_before, library_before);
}

#[test]
fn winnow_leaves_a_permanent_without_a_name_twin_but_still_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = game_with(1, &[grizzly_bears()]);
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library.last().unwrap();
    let (hand_before, library_before) = (hand_len(&engine, p0), library.len());

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, winnow());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert_draws_top_card(&engine, p0, top, hand_before, library_before);
}

#[test]
fn winnow_never_offers_a_land_even_when_two_share_a_name() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = game_with(1, &[forest(), forest(), gray_ogre()]);
    let forests = all_on_battlefield(&engine, p1, forest());
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, winnow());
    let menu = aim_at(&mut engine, p0, ogre);
    assert!(menu.contains(&ogre));
    for forest in forests {
        assert!(
            !menu.contains(&forest),
            "a land is never a target: {menu:?}"
        );
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}
