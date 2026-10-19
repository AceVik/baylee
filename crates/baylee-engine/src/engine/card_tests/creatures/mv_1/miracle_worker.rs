//! `cards/creatures/mv_1/miracle_worker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
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

/// Casts `aura` from player 0's hand onto `host` and lets it resolve.
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

/// The permanent attached to `host`, which must be exactly one.
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

fn worker_ready(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn worker_offered(e: &Engine<RegistryLookup>) -> bool {
    let src = on_battlefield(e, P0, card_index("86ddbc56-3318-49cf-be95-78772ef87637"))
        .expect("the Worker is out");
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    legal.abilities.contains(&(src, 0))
}

/// Miracle Worker (86ddbc56-3318-49cf-be95-78772ef87637) — "{T}: Destroy
/// target Aura attached to a creature you control."
#[test]
fn miracle_worker_destroys_an_aura_on_your_creature() {
    let mut e = worker_ready(
        &[dead_weight()],
        &[
            card_index("86ddbc56-3318-49cf-be95-78772ef87637"),
            serra_angel(),
        ],
        &[],
    );
    let angel = on_battlefield(&e, P0, serra_angel()).expect("the Angel is out");
    cast_aura(&mut e, dead_weight(), angel);
    let aura = aura_on(&e, angel);
    assert_eq!(pt(&e, angel), (2, 2), "Dead Weight shrinks it");

    let worker = on_battlefield(&e, P0, card_index("86ddbc56-3318-49cf-be95-78772ef87637"))
        .expect("the Worker is out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: worker,
            ability_index: 0,
        },
    )
    .expect("{T} is paid");
    let menu = aim_at(&mut e, P0, aura);
    assert_eq!(menu, vec![aura], "the one Aura on my creature");
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, dead_weight()).is_none(),
        "the Aura left the battlefield"
    );
    assert!(in_graveyard(&e, P0, dead_weight()).is_some());
    assert_eq!(pt(&e, angel), (4, 4), "the Angel is itself again");
    assert!(
        e.state()
            .object(worker)
            .expect("the Worker is out")
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn miracle_worker_cannot_aim_at_auras_on_opposing_creatures_or_on_lands() {
    let mut e = worker_ready(
        &[dead_weight(), dead_weight(), wild_growth()],
        &[
            card_index("86ddbc56-3318-49cf-be95-78772ef87637"),
            serra_angel(),
            forest(),
        ],
        &[hill_giant()],
    );
    let angel = on_battlefield(&e, P0, serra_angel()).expect("the Angel is out");
    let giant = on_battlefield(&e, P1_SEAT, hill_giant()).expect("the Giant is out");
    let land = on_battlefield(&e, P0, forest()).expect("the Forest is out");
    cast_aura(&mut e, dead_weight(), angel);
    cast_aura(&mut e, dead_weight(), giant);
    cast_aura(&mut e, wild_growth(), land);
    let mine = aura_on(&e, angel);
    let theirs = aura_on(&e, giant);
    let on_land = aura_on(&e, land);

    let worker = on_battlefield(&e, P0, card_index("86ddbc56-3318-49cf-be95-78772ef87637"))
        .expect("the Worker is out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: worker,
            ability_index: 0,
        },
    )
    .expect("{T} is paid");
    let menu = aim_at(&mut e, P0, mine);
    assert!(menu.contains(&mine));
    assert!(!menu.contains(&theirs), "the Giant is not my creature");
    assert!(!menu.contains(&on_land), "a land is not a creature");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, wild_growth()).is_some(),
        "still there"
    );
    assert_eq!(pt(&e, giant), (1, 1), "the Giant's Aura stands");
}

#[test]
fn miracle_worker_has_no_target_without_an_aura() {
    let e = worker_ready(
        &[],
        &[
            card_index("86ddbc56-3318-49cf-be95-78772ef87637"),
            serra_angel(),
        ],
        &[],
    );
    assert!(!worker_offered(&e));
}

const P1_SEAT: PlayerId = PlayerId::new(1);
