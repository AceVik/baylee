//! `cards/creatures/mv_3/flowstone_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Wall is a `{2}{R}` 0/6 Wall with defender and "`{R}`: This
/// creature gets +1/-1 until end of turn".
///
/// One board reads both printed lines and the price of the second, which
/// reading the card cannot do: five Mountains pay the `{2}{R}` and leave
/// exactly the two red the pump charges, so the pump is a real payment out of
/// the pool rather than a label on a free ability, and the pool is empty once
/// both are spent. The body after each activation — `(1, 5)` then `(2, 4)` —
/// is the only pair that applies +1/-1 twice; a card that read the ability as
/// +1/+1, or as a one-shot, cannot produce both, and the Wall standing
/// untapped after each says the price is mana and not a tap. The Llanowar
/// Elves kept untapped beside it are the control for defender: the combat
/// step is reached at all and offers the Elf, so the Wall's absence is the
/// keyword and not a phase that never came.
#[test]
fn flowstone_wall_trades_toughness_for_power_one_point_of_red_at_a_time() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[flowstone_wall()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Mountains pay the {2}{R} and nothing else is tapped: the Elf is
    // the control the attack declaration below reads, and it has to still be
    // standing untapped when the combat step asks.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains, and the Elf paid nothing into it"
    );
    cast_with_floating(&mut engine, p0, flowstone_wall());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, flowstone_wall()).expect("the Wall resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}}{{R}} is spent and the two red the pump charges are left"
    );
    assert_eq!(pt(&engine, wall), (0, 6), "the printed 0/6 body");
    assert!(
        types(&engine, wall).contains(TypeSet::CREATURE),
        "a Wall is a creature"
    );
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "and it prints defender"
    );

    // Ability 0 is the only line the card prints, and its whole price is one
    // red — not its own tap — so the Wall is still standing to go again.
    let mut pumped = 0;
    for expected in [(1, 5), (2, 4)] {
        activate(&mut engine, p0, flowstone_wall(), 0);
        // `Effect::PumpFilter` over `Filter::This` names its own permanent;
        // answered here if a question arrives rather than assumed either way.
        if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![wall],
                    },
                )
                .expect("the Wall is the creature the pump is about");
        }
        pass_until(&mut engine, stack_is_empty);
        pumped += 1;
        assert_eq!(
            pt(&engine, wall),
            expected,
            "+1/-1 for each red spent, after {pumped} activation(s)"
        );
        assert!(
            !is_tapped(&engine, wall),
            "the price is one red and no tap, so the Wall is still up"
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both red were spent: two activations, two mana"
    );

    // Defender: the combat step is reached, the untapped Elf beside it is
    // offered, and the 2/4 Wall is not.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"Defender (This creature can't attack.)\" keeps the Wall home even \
         as a 2/4: {attackers:?}"
    );
}
