//! Oracle scenarios for Pianna, Nomad Captain.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn pianna_nomad_captain() -> CardIndex {
    card_index("9cd8f7f6-0b69-466a-8f4c-d864ff682457")
}

#[test]
fn pianna_nomad_captain_pumps_all_attacking_creatures_when_she_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[
                index::PLAINS,
                index::PLAINS,
                index::PLAINS,
                index::GRIZZLY_BEARS,
                index::GRAY_OGRE,
            ],
        )
        .hand(0, &[pianna_nomad_captain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, pianna_nomad_captain());
    pass_until(&mut e, stack_is_empty);

    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));

    let pianna = on_battlefield(&e, p0, pianna_nomad_captain()).unwrap();
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&e, p0, index::GRAY_OGRE).unwrap();

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (pianna, Defender::Player(p1)),
                (bears, Defender::Player(p1)),
            ],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, pianna), (3, 3));
    assert_eq!(pt(&e, bears), (3, 3));
    assert_eq!(pt(&e, ogre), (2, 2), "non-attacking creature is not pumped");
}
