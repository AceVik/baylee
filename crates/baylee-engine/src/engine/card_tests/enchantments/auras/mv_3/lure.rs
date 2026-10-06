//! `cards/enchantments/auras/mv_3/lure.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lure — "Enchant creature" (CR 702.5a, 303.4a): offered only a creature,
/// never Sol Ring, and ends attached to the Elves; the forced-block
/// sentence is played in
/// `lure_forces_every_able_creature_to_block_it_but_spares_a_second_attacker`
/// and its siblings below.
#[test]
fn lure_attaches_only_to_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), llanowar_elves(), sol_ring()],
        )
        .hand(0, &[lure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    let rock = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    attaches_only_to(&mut engine, p0, lure(), elf, rock);
}

/// Lure's second sentence, on a board with a second attacker beside the
/// enchanted one: both of the defending player's creatures are able to
/// block the Lured Bear and neither is offered a choice about it — a
/// declaration leaving either out is refused, and the Ogre attacking
/// alongside the Bear, with no Lure of its own, is never required to be
/// blocked at all.
#[test]
fn lure_forces_every_able_creature_to_block_it_but_spares_a_second_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), grizzly_bears(), gray_ogre()],
        )
        .battlefield(1, &[hurloon_minotaur(), pearled_unicorn()])
        .hand(0, &[lure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bear is seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre is seated");
    let minotaur = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");
    let unicorn = on_battlefield(&engine, p1, pearled_unicorn()).expect("seated");
    let a_forest = on_battlefield(&engine, p0, forest()).expect("a Forest is seated");

    attaches_only_to(&mut engine, p0, lure(), bear, a_forest);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .expect("both attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let question = engine.pending().clone();
    let Pending::ChooseBlockers { obeying, .. } = question.clone() else {
        unreachable!("pass_until stopped on exactly this")
    };
    // The question says so itself (`answer_fault`), so a client can hold
    // its declaration to it before sending it.
    for short in [
        vec![(minotaur, bear)],
        vec![(minotaur, ogre), (unicorn, bear)],
    ] {
        assert_eq!(
            question.answer_fault(&PlayerAction::DeclareBlockers { blockers: short }),
            Some(crate::choice::AnswerFault::MustBlock)
        );
    }
    assert_eq!(
        question.answer_fault(&PlayerAction::DeclareBlockers {
            blockers: obeying.clone()
        }),
        None
    );
    let mut obeying_sorted = obeying.clone();
    obeying_sorted.sort_unstable();
    let mut expected = vec![(minotaur, bear), (unicorn, bear)];
    expected.sort_unstable();
    assert_eq!(
        obeying_sorted, expected,
        "both defenders are able to block the Lured Bear and must; the Ogre is named nowhere"
    );

    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(minotaur, bear)],
            },
        ),
        MUST_BLOCK,
    );
    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(minotaur, ogre), (unicorn, bear)],
            },
        ),
        MUST_BLOCK,
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: obeying })
        .expect("the question's own declaration is legal");

    let mut blockers_of_bear = engine.state().combat.blockers_of(bear);
    blockers_of_bear.sort_unstable();
    let mut expected = vec![minotaur, unicorn];
    expected.sort_unstable();
    assert_eq!(
        blockers_of_bear, expected,
        "the Bear ends up double-blocked by both of the defenders Lure named \
         — the Ogre's freedom from any block is already shown above, by \
         `obeying` naming it nowhere and the refusal that tried to swap a \
         blocker off the Bear and onto it"
    );
}

/// Lure's requirement only reaches a creature able to block the enchanted
/// one: a ground creature facing a Lured flier, and a flier that is tapped,
/// are both refused outright as blockers of it — evasion and a tap are not
/// waived by the Aura — and the one flier standing is the only creature the
/// question ever names.
#[test]
fn lure_does_not_require_a_creature_that_cannot_block_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), fire_sprites()])
        .battlefield(1, &[grizzly_bears(), angelic_wall(), angelic_wall()])
        .hand(0, &[lure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sprite = on_battlefield(&engine, p0, fire_sprites()).expect("the Sprites are seated");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let walls = all_on_battlefield(&engine, p1, angelic_wall());
    assert_eq!(walls.len(), 2, "two Angelic Walls are seated");
    let (untapped_wall, tapped_wall) = (walls[0], walls[1]);
    engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up")
        .set_tapped(tapped_wall, true);
    engine.refresh_offer();
    let a_forest = on_battlefield(&engine, p0, forest()).expect("a Forest is seated");

    attaches_only_to(&mut engine, p0, lure(), sprite, a_forest);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sprite, Defender::Player(p1))],
            },
        )
        .expect("the Sprites attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { obeying, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert_eq!(
        obeying,
        vec![(untapped_wall, sprite)],
        "only the standing flier is named; the Bear and the tapped Wall are not"
    );

    refused(
        engine.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] }),
        MUST_BLOCK,
    );
    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, sprite)],
            },
        ),
        "not among the options",
    );
    refused(
        engine.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(tapped_wall, sprite)],
            },
        ),
        "not among the options",
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(untapped_wall, sprite)],
            },
        )
        .expect("the one able flier blocks");
    assert_eq!(
        engine.state().combat.blockers_of(sprite),
        vec![untapped_wall],
        "the Sprites is blocked by exactly the one able flier"
    );
}
