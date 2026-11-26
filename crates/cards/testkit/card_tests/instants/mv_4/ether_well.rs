//! `cards/instants/mv_4/ether_well.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ether_well() -> CardIndex {
    card_index("a2b6faca-3321-45d2-a027-ff3a8ae61f10")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn gray_ogre() -> CardIndex {
    card_index("83c8a3a6-2e1a-4e26-8847-6d066f42d906")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lib_cards(e: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<CardIndex> {
    e.state()
        .zones
        .list(ZoneLocation::Library(seat))
        .iter()
        .filter_map(|id| e.state().object(*id).and_then(|o| o.card.map(|c| c.index)))
        .collect()
}

#[test]
fn ether_well_puts_nonred_on_top_and_offers_bottom_for_red_creature() {
    let mut e = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[ether_well(), ether_well()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    // Non-red Bears goes straight to top with no YesNo prompt.
    let bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut e, P0, ether_well());
    kit::aim(&mut e, &[bears], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lib_cards(&e, P1).last().copied(), Some(grizzly_bears()));

    // Red Ogre offers bottom option; declining puts it on top.
    let ogre = on_battlefield(&e, P1, gray_ogre()).unwrap();
    cast_from_hand(&mut e, P0, ether_well());
    kit::aim(&mut e, &[ogre], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lib_cards(&e, P1).last().copied(), Some(gray_ogre()));
}

#[test]
fn ether_well_puts_red_creature_on_bottom_when_accepted() {
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[ether_well()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    // Accepting bottom option puts red Ogre on bottom (index 0).
    let ogre = on_battlefield(&e, P1, gray_ogre()).unwrap();
    cast_from_hand(&mut e, P0, ether_well());
    kit::aim(&mut e, &[ogre], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lib_cards(&e, P1).first().copied(), Some(gray_ogre()));
}
