//! `cards/creatures/mv_2/merfolk_assassin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// River Bear: a plain 3/3 with islandwalk.
fn river_bear() -> CardIndex {
    card_index("568c4d02-501e-44ce-871c-adf9e4b9d8d6")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn ready(mine: &[CardIndex], theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut all = vec![card_index("7edd536c-c043-4189-aa57-83d22eed3103")];
    all.extend_from_slice(mine);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &all)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let assassin = on_battlefield(&e, P0, card_index("7edd536c-c043-4189-aa57-83d22eed3103"))
        .expect("the Assassin is out");
    (e, assassin)
}

/// Merfolk Assassin (7edd536c-c043-4189-aa57-83d22eed3103) — "{T}: Destroy
/// target creature with islandwalk."
#[test]
fn merfolk_assassin_destroys_a_creature_with_islandwalk_and_only_that_one() {
    let (mut e, assassin) = ready(&[], &[river_bear(), grizzly_bears()]);
    let bear = on_battlefield(&e, P1, river_bear()).expect("the River Bear is out");
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Grizzly Bears are out");
    assert!(keywords(&e, bear).contains(KeywordSet::ISLANDWALK));

    fill_pools(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: assassin,
            ability_index: 0,
        },
    )
    .expect("{T} is paid");
    let menu = aim_at(&mut e, P0, bear);
    assert_eq!(menu, vec![bear], "only the islandwalker, not the Bears");
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P1, river_bear()).is_none(), "destroyed");
    assert!(in_graveyard(&e, P1, river_bear()).is_some());
    assert_eq!(
        e.state().object(bears).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the Bears are untouched"
    );
    assert!(
        e.state()
            .object(assassin)
            .expect("the Assassin is out")
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn merfolk_assassin_may_destroy_its_own_sides_islandwalker_too() {
    let (mut e, assassin) = ready(&[river_bear()], &[]);
    let bear = on_battlefield(&e, P0, river_bear()).expect("the River Bear is out");
    fill_pools(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: assassin,
            ability_index: 0,
        },
    )
    .expect("{T} is paid");
    let menu = aim_at(&mut e, P0, bear);
    assert!(menu.contains(&bear), "any creature with islandwalk");
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, river_bear()).is_some());
}

#[test]
fn merfolk_assassin_has_no_target_without_an_islandwalker() {
    let (mut e, assassin) = ready(&[], &[grizzly_bears()]);
    fill_pools(&mut e);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    assert!(!legal.abilities.contains(&(assassin, 0)));
}
