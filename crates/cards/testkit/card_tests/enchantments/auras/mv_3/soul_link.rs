//! Oracle scenarios for Soul Link.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn soul_link() -> CardIndex {
    card_index("bb81aa50-49db-4743-85fc-6b4d9ed29bc6")
}

#[test]
fn soul_link_gains_life_when_enchanted_creature_deals_and_is_dealt_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[
                index::PLAINS,
                index::PLAINS,
                index::SWAMP,
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[soul_link()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let giant = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut e, p0, soul_link());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(giant, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bears, giant)],
        },
    )
    .unwrap();

    reach_their_main_phase(&mut e, p1);
    // Hill Giant dealt 3 damage to Grizzly Bears (+3 life) and was dealt 2 damage by Grizzly Bears (+2 life).
    assert_eq!(e.state().players[0].life, 25);
}
