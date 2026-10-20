//! `cards/creatures/artifacts/mv_5/scarecrow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

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

/// Player 1 attacks player 0 with a flier (Serra Angel, 4) and a walker
/// (Grizzly Bears, 2); player 0 may use the Scarecrow once the attack is
/// declared. Returns how much life player 0 lost, and the Scarecrow.
fn flier_and_walker_attack(use_scarecrow: bool) -> (i32, Engine<RegistryLookup>) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("58616e15-531f-4680-8294-35ab7e962c96")])
        .battlefield(1, &[serra_angel(), grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let before = e.state().players[0].life;

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::DeclareAttackers
    });
    let angel = on_battlefield(&e, P1, serra_angel()).expect("the Angel is out");
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(angel, Defender::Player(P0)), (bears, Defender::Player(P0))],
        },
    )
    .expect("both attack");

    if use_scarecrow {
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
        );
        fill_pools(&mut e);
        let scarecrow = on_battlefield(&e, P0, card_index("58616e15-531f-4680-8294-35ab7e962c96"))
            .expect("Scarecrow is out");
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: scarecrow,
                ability_index: 0,
            },
        )
        .expect("{6}, {T} is paid");
        pass_until(&mut e, stack_is_empty);
        assert!(
            e.state()
                .object(scarecrow)
                .expect("Scarecrow is still out")
                .status
                .contains(Status::TAPPED),
            "the cost tapped it"
        );
    }
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    (before - e.state().players[0].life, e)
}

/// Scarecrow (58616e15-531f-4680-8294-35ab7e962c96) — "{6}, {T}: Prevent all
/// damage that would be dealt to you this turn by creatures with flying."
#[test]
fn scarecrow_prevents_damage_from_fliers_only() {
    let (lost, _) = flier_and_walker_attack(false);
    assert_eq!(lost, 6, "without the ability: Angel 4 + Bears 2");
    let (lost, e) = flier_and_walker_attack(true);
    assert_eq!(lost, 2, "only the Bears' damage gets through");
    let angel = on_battlefield(&e, P1, serra_angel()).expect("the Angel is still out");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
}
