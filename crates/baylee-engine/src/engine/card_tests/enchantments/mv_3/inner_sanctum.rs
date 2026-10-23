//! `cards/enchantments/mv_3/inner_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn inner_sanctum() -> CardIndex {
    card_index("2ae36d31-08b7-481a-bfa1-a5c65921dee0")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// Bolts `target` with exactly its {R}.
#[track_caller]
fn bolt(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    lk::float(e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(e, P0, lightning_bolt());
    aim_at(e, P0, target);
    lk::settle(e);
}

/// The life the upkeep on `turn` asks P0 to pay.
fn upkeep_price(e: &mut Engine<RegistryLookup>, turn: u32) -> u32 {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayLife { amount },
        ..
    } = e.pending().clone()
    else {
        panic!("the upkeep asks to pay life: {:?}", e.pending())
    };
    assert_eq!(player, P0);
    amount
}

/// Inner Sanctum (2ae36d31-08b7-481a-bfa1-a5c65921dee0) — "Prevent all damage
/// that would be dealt to creatures you control." A Bolt on the caster's Bears
/// marks nothing; the same Bolt on the opponent's Bears kills them; a Bolt on
/// the player is not prevented.
#[test]
fn inner_sanctum_prevents_damage_to_your_creatures_only() {
    let mut e = lk::game_with(
        SEED,
        &[inner_sanctum(), grizzly_bears()],
        &[grizzly_bears()],
        &[lightning_bolt(), lightning_bolt(), lightning_bolt()],
        &[],
        |e| lk::yes_no(e, true),
    );
    assert_eq!(
        lk::life(&e, P0),
        18,
        "the first cumulative upkeep cost 2 life"
    );
    let mine = lk::obj(&e, P0, grizzly_bears());
    let theirs = lk::obj(&e, P1, grizzly_bears());

    bolt(&mut e, mine);
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "the Bolt was prevented"
    );
    assert_eq!(lk::damage_on(&e, mine), 0);

    bolt(&mut e, theirs);
    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "their Bears are not protected"
    );

    // The caster at 18 takes the third Bolt (players are not creatures).
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P0],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P0), 15, "damage to a player is not prevented");
}

/// Inner Sanctum — "Cumulative upkeep—Pay 2 life": the first upkeep asks 2,
/// the next 4 (two counters); declined, the Sanctum is sacrificed and the
/// prevention goes with it, so the next Bolt kills the Bears.
#[test]
fn inner_sanctum_costs_two_more_life_each_upkeep_and_its_protection_ends_with_it() {
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[inner_sanctum(), grizzly_bears(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);

    assert_eq!(upkeep_price(&mut e, 1), 2, "one age counter, 2 life each");
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lk::life(&e, P0), 18);

    reach_their_main_phase(&mut e, P1);
    assert_eq!(lk::life(&e, P0), 18, "P1's upkeep is not yours");

    assert_eq!(upkeep_price(&mut e, 3), 4, "two age counters, 2 life each");
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(lk::life(&e, P0), 18, "nothing was paid");
    assert!(
        in_graveyard(&e, P0, inner_sanctum()).is_some(),
        "sacrificed"
    );

    reach_their_main_phase(&mut e, P0);
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("the Bears are still out");
    bolt(&mut e, bears);
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "no Sanctum, no prevention"
    );
}
