#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn veiled_crocodile() -> CardIndex {
    card_index("bdc3ff35-f8d8-477d-b042-94a9f8ed7243")
}

#[test]
fn veiled_crocodile_becomes_crocodile_creature_when_hands_are_empty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[veiled_crocodile(), gray_ogre()])
        .battlefield(0, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for seat in [PlayerId::new(1)] {
        let hand: Vec<ObjectId> = engine
            .state()
            .zones
            .list(ZoneLocation::Hand(seat))
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_none_or(|o| o.card.is_none_or(|c| c.index != veiled_crocodile()))
            })
            .collect();
        let state = engine.dev_state_mut(p0).unwrap();
        for id in hand {
            state
                .move_object(
                    id,
                    ZoneLocation::Graveyard(seat),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Setup,
                )
                .unwrap();
        }
    }
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, veiled_crocodile());
    pass_until(&mut engine, stack_is_empty);

    let croc = on_battlefield(&engine, p0, veiled_crocodile()).expect("crocodile on bf");
    assert_eq!(pt(&engine, croc), (4, 4));
    assert!(
        in_hand(&engine, p0, gray_ogre()).is_some(),
        "only one player need have an empty hand"
    );
}

#[test]
fn veiled_crocodile_stays_an_enchantment_when_every_player_has_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[veiled_crocodile(), gray_ogre()],
        &[gray_ogre()],
    );
    lk::float(&mut engine, p0, &[(ManaColor::Blue, 3)]);
    lk::cast_at(&mut engine, p0, veiled_crocodile(), &[], &[]);
    let croc = lk::obj(&engine, p0, veiled_crocodile());
    assert!(
        engine
            .state()
            .object(croc)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(
        !engine
            .state()
            .object(croc)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}
