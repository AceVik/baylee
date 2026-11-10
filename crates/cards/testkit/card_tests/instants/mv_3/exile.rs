//! `cards/instants/mv_3/exile.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn exile() -> CardIndex {
    card_index("8c91d50b-bf58-4d06-b3b7-4b6db867bfce")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn elvish_bard() -> CardIndex {
    card_index("094b778f-95a0-436f-a9d0-20a46a674486")
}

/// `Exile` ({2}{W}, Instant) exiles target nonwhite attacking creature and you
/// gain life equal to its toughness.
#[test]
fn exile_removes_nonwhite_attacking_creature_and_gains_its_toughness_in_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Hill Giant 3/3 and Elvish Bard 2/4 are nonwhite attackers, Serra Angel is a white one, and
    // the Bears stay home.
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 3])
        .battlefield(
            1,
            &[hill_giant(), serra_angel(), elvish_bard(), grizzly_bears()],
        )
        .hand(0, &[exile()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::DeclareAttackers
    });
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let angel = on_battlefield(&engine, p1, serra_angel()).unwrap();
    let bard = on_battlefield(&engine, p1, elvish_bard()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (giant, Defender::Player(p0)),
                    (angel, Defender::Player(p0)),
                    (bard, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, exile());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Exile, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&giant) && options.contains(&bard),
        "nonwhite attacking creatures are legal targets"
    );
    assert!(
        !options.contains(&angel),
        "white attacking Serra Angel cannot be targeted by Exile"
    );
    assert!(
        !options.contains(&bears),
        "a nonwhite creature that is not attacking cannot be targeted either"
    );

    let life = engine.state().players[0].life;
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bard],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bard).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0].life,
        life + 4,
        "its toughness 4, not its power 2"
    );
    for survivor in [giant, angel, bears] {
        assert_eq!(
            engine.state().object(survivor).unwrap().zone,
            Zone::Battlefield,
            "only the target is exiled"
        );
    }
}
