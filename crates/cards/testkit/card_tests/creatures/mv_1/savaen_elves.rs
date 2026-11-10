//! `cards/creatures/mv_1/savaen_elves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn dead_weight() -> CardIndex {
    card_index("b1804304-fac1-4b19-a48d-6ade9407972a")
}

fn wild_growth() -> CardIndex {
    card_index("706ae742-1807-44b7-a4fa-f2e26f61519a")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn cast_aura(e: &mut Engine<RegistryLookup>, aura: CardIndex, host: ObjectId) {
    fill_pools(e);
    cast_with_floating(e, P0, aura);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected the host question, got {:?}", e.pending())
    };
    assert!(options.contains(&host), "the host is a legal one");
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![host],
            players: vec![],
        },
    )
    .expect("the host is named");
    pass_until(e, stack_is_empty);
}

fn aura_on(e: &Engine<RegistryLookup>, host: ObjectId) -> ObjectId {
    let found: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| e.state().object(*id).and_then(|o| o.attached_to) == Some(host))
        .collect();
    assert_eq!(found.len(), 1, "one Aura on the host: {found:?}");
    found[0]
}

fn ready(hand: &[CardIndex], mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn activate_elves(e: &mut Engine<RegistryLookup>) -> ObjectId {
    fill_pools(e);
    let elves = on_battlefield(e, P0, card_index("330e109c-1885-4946-9edc-895408519b3d"))
        .expect("the Elves are out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: elves,
            ability_index: 0,
        },
    )
    .expect("{G}{G}, {T} is paid");
    elves
}

/// Savaen Elves (330e109c-1885-4946-9edc-895408519b3d) — "{G}{G}, {T}: Destroy
/// target Aura attached to a land."
#[test]
fn savaen_elves_destroy_an_aura_on_your_land_and_not_one_on_a_creature() {
    let mut e = ready(
        &[wild_growth(), dead_weight()],
        &[
            card_index("330e109c-1885-4946-9edc-895408519b3d"),
            forest(),
            serra_angel(),
        ],
        &[],
    );
    let land = on_battlefield(&e, P0, forest()).expect("the Forest is out");
    let angel = on_battlefield(&e, P0, serra_angel()).expect("the Angel is out");
    cast_aura(&mut e, wild_growth(), land);
    cast_aura(&mut e, dead_weight(), angel);
    let on_land = aura_on(&e, land);
    let on_creature = aura_on(&e, angel);

    let elves = activate_elves(&mut e);
    let menu = aim_at(&mut e, P0, on_land);
    assert_eq!(menu, vec![on_land], "the creature's Aura is not on offer");
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, wild_growth()).is_none(), "destroyed");
    assert!(in_graveyard(&e, P0, wild_growth()).is_some());
    assert!(
        on_battlefield(&e, P0, dead_weight()).is_some(),
        "left alone"
    );
    assert_eq!(pt(&e, angel), (2, 2));
    assert_eq!(
        e.state().object(on_creature).map(|o| o.zone),
        Some(Zone::Battlefield),
        "Dead Weight is still attached"
    );
    assert!(
        e.state()
            .object(elves)
            .expect("the Elves are out")
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn savaen_elves_destroy_an_aura_on_an_opponents_land() {
    let mut e = ready(
        &[wild_growth()],
        &[card_index("330e109c-1885-4946-9edc-895408519b3d")],
        &[forest()],
    );
    let land = on_battlefield(&e, P1, forest()).expect("their Forest is out");
    cast_aura(&mut e, wild_growth(), land);
    let aura = aura_on(&e, land);

    activate_elves(&mut e);
    let menu = aim_at(&mut e, P0, aura);
    assert_eq!(menu, vec![aura]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, wild_growth()).is_none());
    assert!(
        in_graveyard(&e, P0, wild_growth()).is_some(),
        "it is P0's card"
    );
}

#[test]
fn savaen_elves_have_no_target_without_an_aura_on_a_land() {
    let mut e = ready(
        &[],
        &[card_index("330e109c-1885-4946-9edc-895408519b3d"), forest()],
        &[],
    );
    fill_pools(&mut e);
    let elves = on_battlefield(&e, P0, card_index("330e109c-1885-4946-9edc-895408519b3d"))
        .expect("the Elves are out");
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority")
    };
    assert!(!legal.abilities.contains(&(elves, 0)));
}
