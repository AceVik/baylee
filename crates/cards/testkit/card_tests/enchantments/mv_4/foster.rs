//! `cards/enchantments/mv_4/foster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn foster() -> CardIndex {
    card_index("fb431500-152c-4524-b76b-de62922ff57f")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn objs(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Whenever a creature you control dies, paying {1} reveals from library top
/// until a creature card, puts it into hand, and mills the rest; declining does
/// nothing; opponent's creature dying does not trigger.
#[test]
fn foster_pays_one_to_reveal_until_creature_into_hand_and_mills_rest() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[foster(), llanowar_elves(), llanowar_elves()],
        &[savannah_lions()],
        &[grizzly_bears(), lightning_bolt()],
        &[],
    );
    // Stack library top: Forest (top from Duel filler) -> Lightning Bolt -> Grizzly Bears.
    let bear_lib = in_hand(&e, p0, grizzly_bears()).unwrap();
    let bolt_lib = in_hand(&e, p0, lightning_bolt()).unwrap();
    {
        let st = e.dev_state_mut(p0).unwrap();
        st.move_object(
            bear_lib,
            ZoneLocation::Library(p0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Setup,
        )
        .unwrap();
        st.move_object(
            bolt_lib,
            ZoneLocation::Library(p0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Setup,
        )
        .unwrap();
    }
    let elves = objs(&e, p0, llanowar_elves());
    let opp_lions = lk::obj(&e, p1, savannah_lions());

    // Opponent's creature dying does not trigger Foster.
    kill(&mut e, opp_lions);
    assert!(matches!(e.pending(), Pending::Priority { .. }));

    // My creature dies -> pay {1} -> Lightning Bolt goes to graveyard, Grizzly Bears to hand.
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 1)]);
    let gy_before = e.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    baylee_engine::sba::destroy(e.dev_state_mut(p0).unwrap(), elves[0]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, bear_lib), Zone::Hand);
    assert_eq!(lk::zone_of(&e, bolt_lib), Zone::Graveyard);
    assert!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len() >= gy_before + 2);

    // Second creature dies -> decline {1} -> hand does not gain another card.
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    baylee_engine::sba::destroy(e.dev_state_mut(p0).unwrap(), elves[1]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
