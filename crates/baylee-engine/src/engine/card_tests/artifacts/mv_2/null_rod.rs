//! `cards/artifacts/mv_2/null_rod.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn null_rod() -> CardIndex {
    card_index("2f83ca86-e23d-40f7-8085-6928d8cfef9b")
}

fn mox_sapphire() -> CardIndex {
    card_index("d5ed1233-df87-4b90-8918-13922ec95249")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Null Rod (2f83ca86-e23d-40f7-8085-6928d8cfef9b) — "Activated abilities of
/// artifacts can't be activated." Before the Rod the Mox taps for mana; once
/// it is cast the Mox's mana ability is gone from the offer (mana abilities are
/// activated abilities), while the Elves, which are no artifact, still tap.
#[test]
fn null_rod_stops_an_artifact_mana_ability_but_not_a_creature_s() {
    let mut e = lk::game(
        SEED,
        &[mox_sapphire(), llanowar_elves(), island()],
        &[],
        &[null_rod()],
        &[],
    );
    assert!(
        lk::offered(&e, mox_sapphire(), P0, 0),
        "the Mox taps before the Rod"
    );
    assert!(lk::offered(&e, llanowar_elves(), P0, 0));

    lk::float(&mut e, P0, &[(baylee_core::mana::ManaColor::Colorless, 2)]);
    cast_with_floating(&mut e, P0, null_rod());
    pass_until(&mut e, stack_is_empty);

    assert!(
        !lk::offered(&e, mox_sapphire(), P0, 0),
        "an artifact's activated ability, mana ability included, is not offered"
    );
    assert!(
        lk::offered(&e, llanowar_elves(), P0, 0),
        "a creature that is not an artifact still taps for mana"
    );
}

/// Null Rod — "artifacts", not "your artifacts": the opponent's Mox is shut off
/// too. On player 1's own turn, with the Rod on player 0's side, the Mox is not
/// offered to its controller, while their Elves are.
#[test]
fn null_rod_also_stops_the_opponents_artifacts() {
    let mut e = lk::game(
        SEED,
        &[null_rod()],
        &[mox_sapphire(), llanowar_elves()],
        &[],
        &[],
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });

    assert!(
        !lk::offered(&e, mox_sapphire(), P1, 0),
        "the opponent's Mox is an artifact too"
    );
    assert!(
        lk::offered(&e, llanowar_elves(), P1, 0),
        "their creature is not"
    );
}
