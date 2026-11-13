//! `cards/creatures/mv_3/vraska_the_cutting_glare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn vraska() -> CardIndex {
    card_index("580de511-863c-4b7a-9d2e-fcdf6e12d54b")
}

fn treasures(e: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    tokens_of(e, seat)
        .into_iter()
        .filter(|id| {
            e.state().object(*id).is_some_and(|object| {
                object
                    .characteristics()
                    .subtypes
                    .contains(subtypes::artifact::TREASURE)
            })
        })
        .collect()
}

#[test]
fn vraska_the_cutting_glare_plays_deathtouch_in_combat() {
    let mut e = lk::game(SEED, &[vraska()], &[rootbreaker_wurm()], &[], &[]);
    let attacker = lk::obj(&e, P0, vraska());
    let blocker = lk::obj(&e, P1, rootbreaker_wurm());
    assert!(keywords(&e, attacker).contains(KeywordSet::DEATHTOUCH));
    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);
    lk::through_combat(&mut e);
    assert_eq!(
        lk::zone_of(&e, blocker),
        Zone::Graveyard,
        "four damage is lethal with deathtouch"
    );
}

#[test]
fn vraska_the_cutting_glare_requires_six_lands_and_gives_its_victims_controller_treasure() {
    for land_count in [5, 6] {
        let lands = vec![forest(); land_count];
        let mut e = lk::game(SEED, &lands, &[index::JAYEMDAE_TOME], &[vraska()], &[]);
        let land = lk::obj(&e, P0, forest());
        let tome = lk::obj(&e, P1, index::JAYEMDAE_TOME);
        lk::float(&mut e, P0, &[(ManaColor::Black, 2), (ManaColor::Green, 1)]);
        cast_with_floating(&mut e, P0, vraska());

        if land_count == 5 {
            lk::settle(&mut e);
            assert_eq!(lk::zone_of(&e, tome), Zone::Battlefield);
            assert!(treasures(&e, P1).is_empty(), "five lands do not trigger");
            continue;
        }

        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
            unreachable!()
        };
        let vraska = lk::obj(&e, P0, vraska());
        assert_eq!(options, vec![tome], "only an opponent's permanent is legal");
        assert!(!options.contains(&land));
        assert!(!options.contains(&vraska));
        lk::aim(&mut e, &[tome], &[]);
        lk::settle(&mut e);

        assert_eq!(lk::zone_of(&e, tome), Zone::Graveyard);
        assert!(treasures(&e, P0).is_empty());
        let treasures = treasures(&e, P1);
        assert_eq!(
            treasures.len(),
            1,
            "the destroyed permanent's controller creates one"
        );
        let treasure = e.state().object(treasures[0]).unwrap();
        let characteristics = treasure.characteristics();
        assert!(characteristics.types.contains(TypeSet::ARTIFACT));
        assert!(
            characteristics
                .subtypes
                .contains(subtypes::artifact::TREASURE)
        );
    }
}
