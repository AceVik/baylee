//! `cards/creatures/mv_1/icatian_javelineers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn choose_targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().expect("a question is pending");
    if matches!(e.pending(), Pending::ChoosePlayer { .. }) {
        e.apply(p, PlayerAction::ChoosePlayer(players[0]))
            .expect("player chosen");
    } else {
        e.apply(p, PlayerAction::ChooseTargets { objects, players })
            .expect("targets chosen");
    }
}

fn javelins(e: &Engine<RegistryLookup>, id: ObjectId) -> u32 {
    e.state()
        .object(id)
        .expect("the Javelineers are out")
        .counters
        .get(baylee_cards_dsl::counters::JAVELIN)
        .into()
}

/// Casts the Javelineers from hand and lets them resolve; they are made
/// ready to tap (they would otherwise be summoning sick).
fn cast_and_ready(theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[card_index("26112ba7-2859-4af3-93c3-e62c31177216")])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    fill_pools(&mut e);
    cast_with_floating(
        &mut e,
        P0,
        card_index("26112ba7-2859-4af3-93c3-e62c31177216"),
    );
    pass_until(&mut e, stack_is_empty);
    let jav = on_battlefield(&e, P0, card_index("26112ba7-2859-4af3-93c3-e62c31177216"))
        .expect("the Javelineers resolved");
    e.state
        .object_mut(jav)
        .expect("they are out")
        .controlled_since = 0;
    (e, jav)
}

fn shoot(e: &mut Engine<RegistryLookup>, jav: ObjectId) {
    fill_pools(e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: jav,
            ability_index: 0,
        },
    )
    .expect("{T} and the counter are paid");
}

/// Icatian Javelineers (26112ba7-2859-4af3-93c3-e62c31177216) — "This
/// creature enters with a javelin counter on it. {T}, Remove a javelin
/// counter from this creature: It deals 1 damage to any target."
#[test]
fn icatian_javelineers_enter_with_one_javelin_counter() {
    let (e, jav) = cast_and_ready(&[]);
    assert_eq!(javelins(&e, jav), 1);
    assert_eq!(pt(&e, jav), (1, 1));
}

#[test]
fn icatian_javelineers_shoot_a_player_for_one_and_spend_the_counter() {
    let (mut e, jav) = cast_and_ready(&[]);
    shoot(&mut e, jav);
    choose_targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[1].life, 19);
    assert_eq!(javelins(&e, jav), 0, "the counter was the cost");
    assert!(
        e.state()
            .object(jav)
            .expect("they are still out")
            .status
            .contains(Status::TAPPED),
        "so was the tap"
    );
}

#[test]
fn icatian_javelineers_shoot_a_creature_for_one() {
    let (mut e, jav) = cast_and_ready(&[grizzly_bears()]);
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    shoot(&mut e, jav);
    choose_targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    let bears = e.state().object(bears).expect("the Bears are out");
    assert_eq!(bears.zone, Zone::Battlefield);
    assert_eq!(bears.damage, 1);
    assert_eq!(e.state().players[1].life, 20, "the player was not hit");
}

#[test]
fn icatian_javelineers_without_a_counter_cannot_shoot_even_untapped() {
    let (mut e, jav) = cast_and_ready(&[]);
    shoot(&mut e, jav);
    choose_targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    e.state.set_tapped(jav, false);
    fill_pools(&mut e);

    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    assert!(
        !legal.abilities.contains(&(jav, 0)),
        "the counter is gone, so the cost cannot be paid"
    );
}
