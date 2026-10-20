//! `cards/creatures/mv_1/whippoorwill.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

fn balm_of_restoration() -> CardIndex {
    card_index("45a40b4c-1d37-4d87-adf1-abf0ea4c86b4")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
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

fn in_exile(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    e.state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Whippoorwill taps for `target`.
fn whip(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    fill_pools(e);
    let bird = on_battlefield(e, P0, card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64"))
        .expect("the Whippoorwill is out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: bird,
            ability_index: 0,
        },
    )
    .expect("{G}{G}, {T} is paid");
    let menu = aim_at(e, P0, target);
    assert!(menu.contains(&target), "any creature may be aimed at");
    pass_until(e, stack_is_empty);
}

/// Player 0 casts Lightning Bolt at `target`.
fn bolt(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    fill_pools(e);
    cast_with_floating(e, P0, lightning_bolt());
    let Pending::ChooseTargets { player, .. } = e.pending().clone() else {
        panic!("expected a target, got {:?}", e.pending())
    };
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .expect("the target is legal");
    pass_until(e, stack_is_empty);
}

/// Whippoorwill (84050a10-e1f1-413e-aa21-5c1f47bb2a64) — "{G}{G}, {T}: Target
/// creature can't be regenerated this turn. Damage that would be dealt to
/// that creature this turn can't be prevented or dealt instead to another
/// permanent or player. When the creature dies this turn, exile the
/// creature."
#[test]
fn whippoorwill_exiles_the_creature_when_it_dies_this_turn() {
    let mut e = ready(
        &[lightning_bolt()],
        &[card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64")],
        &[grizzly_bears()],
    );
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    whip(&mut e, bears);
    assert!(
        e.state()
            .object(
                on_battlefield(&e, P0, card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64"))
                    .expect("it is out")
            )
            .expect("it is out")
            .status
            .contains(Status::TAPPED),
        "the bird paid {{T}}"
    );
    bolt(&mut e, bears);
    assert!(on_battlefield(&e, P1, grizzly_bears()).is_none(), "it died");
    assert!(in_exile(&e, P1, grizzly_bears()), "and was exiled");
    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_none(),
        "not left in the graveyard"
    );
}

#[test]
fn whippoorwill_a_regeneration_shield_cannot_save_the_creature() {
    let mut e = ready(
        &[lightning_bolt()],
        &[
            card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64"),
            drudge_skeletons(),
        ],
        &[],
    );
    let skeleton = on_battlefield(&e, P0, drudge_skeletons()).expect("the Skeletons are out");
    fill_pools(&mut e);
    raise_a_shield(&mut e, P0, skeleton, 0);

    whip(&mut e, skeleton);
    bolt(&mut e, skeleton);

    assert!(
        on_battlefield(&e, P0, drudge_skeletons()).is_none(),
        "the shield did not regenerate it"
    );
    assert!(in_exile(&e, P0, drudge_skeletons()), "it was exiled");
}

#[test]
fn whippoorwill_without_the_ability_the_same_shield_does_save_the_creature() {
    let mut e = ready(
        &[lightning_bolt()],
        &[
            card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64"),
            drudge_skeletons(),
        ],
        &[],
    );
    let skeleton = on_battlefield(&e, P0, drudge_skeletons()).expect("the Skeletons are out");
    fill_pools(&mut e);
    raise_a_shield(&mut e, P0, skeleton, 0);
    bolt(&mut e, skeleton);
    assert!(
        on_battlefield(&e, P0, drudge_skeletons()).is_some(),
        "it regenerated"
    );
}

/// Player 0 sacrifices the Balm to prevent the next 2 damage to `target`.
fn balm_shield(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    fill_pools(e);
    let balm = on_battlefield(e, P0, balm_of_restoration()).expect("the Balm is out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: balm,
            ability_index: 0,
        },
    )
    .expect("{1}, {T}, sacrifice is paid");
    e.apply(P0, PlayerAction::ChooseMode(1))
        .expect("the prevention mode");
    let p = e.pending().asked().expect("the target question");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .expect("the target is legal");
    pass_until(e, stack_is_empty);
}

#[test]
fn whippoorwill_without_the_ability_prevention_saves_a_bolted_creature() {
    let mut e = ready(
        &[lightning_bolt()],
        &[
            card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64"),
            balm_of_restoration(),
        ],
        &[grizzly_bears()],
    );
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    balm_shield(&mut e, bears);
    bolt(&mut e, bears);
    let bears = e.state().object(bears).expect("the Bears survive");
    assert_eq!(bears.zone, Zone::Battlefield);
    assert_eq!(bears.damage, 1, "3 damage less the 2 prevented");
}

#[test]
fn whippoorwill_damage_to_the_creature_cannot_be_prevented() {
    let mut e = ready(
        &[lightning_bolt()],
        &[
            card_index("84050a10-e1f1-413e-aa21-5c1f47bb2a64"),
            balm_of_restoration(),
        ],
        &[grizzly_bears()],
    );
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    balm_shield(&mut e, bears);
    whip(&mut e, bears);
    bolt(&mut e, bears);
    assert!(
        on_battlefield(&e, P1, grizzly_bears()).is_none(),
        "the full 3 damage was dealt"
    );
    assert!(in_exile(&e, P1, grizzly_bears()), "and it was exiled");
}
