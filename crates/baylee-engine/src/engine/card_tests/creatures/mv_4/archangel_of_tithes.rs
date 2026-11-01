//! `cards/creatures/mv_4/archangel_of_tithes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn archangel_of_tithes() -> CardIndex {
    card_index("ff5caed4-0276-476d-8fae-edae2536df7f")
}

#[test]
fn archangel_of_tithes_taxes_attacks_only_while_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for tapped in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[archangel_of_tithes()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let angel = on_battlefield(&e, p0, archangel_of_tithes()).expect("the Archangel");
        assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
        if tapped {
            e.state.set_tapped(angel, true);
            e.sync_static_effects();
            e.refresh_offer();
        }
        reach_their_main_phase(&mut e, p1);
        let ogre = on_battlefield(&e, p1, gray_ogre()).expect("attacker control");
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
        );
        let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
            unreachable!("the attack question")
        };
        assert_eq!(attackers.contains(&ogre), tapped, "tapped = {tapped}");
        if !tapped {
            assert!(
                e.apply(
                    p1,
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(ogre, Defender::Player(p0))],
                    },
                )
                .is_err()
            );
        }
    }
}

fn block_offer(with_mana: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId, bool) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut theirs = vec![serra_angel()];
    if with_mana {
        theirs.push(forest());
    }
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[archangel_of_tithes()])
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let angel = on_battlefield(&e, p0, archangel_of_tithes()).expect("attacking Archangel");
    let blocker = on_battlefield(&e, p1, serra_angel()).expect("potential flying blocker");
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(angel, Defender::Player(p1))],
        },
    )
    .expect("the Archangel attacks");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = e.pending().clone() else {
        unreachable!("the block question")
    };
    let offered = blockers.iter().any(|option| option.blocker == blocker);
    (e, angel, blocker, offered)
}

#[test]
fn archangel_of_tithes_taxes_blocks_only_while_attacking() {
    let p1 = PlayerId::new(1);
    let (_e, _angel, _blocker, offered) = block_offer(false);
    assert!(
        !offered,
        "without generic mana the flying blocker cannot stop the Archangel"
    );

    let (mut e, angel, blocker, offered) = block_offer(true);
    assert!(offered, "the Forest can pay the one-mana block tax");
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, angel)],
        },
    )
    .expect("declare the block and enter its payment window");
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected the block-tax payment window")
    };
    for source in legal.mana_abilities {
        e.apply(p1, PlayerAction::ActivateManaAbility { source })
            .expect("tap the Forest for the tax");
    }
    e.apply(p1, PlayerAction::PassPriority).expect("pay {1}");
    assert!(e.state().combat.is_blocking(blocker, angel));
    assert_eq!(e.state().players[1].mana_pool.total(), 0);
}
