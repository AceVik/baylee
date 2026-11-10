//! `cards/sorceries/mv_1/detonate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Detonate — {X}{R} — "Destroy target artifact with mana value X. It can't
/// be regenerated. Detonate deals X damage to that artifact's controller."
///
/// X is four, and the board holds one artifact of mana value four — their
/// Living Wall — beside two that are not (a one-drop Sol Ring and a two-drop
/// Pendant), so the menu is `Filter::CmcExactlyX` read whole: the wrong mana
/// value is not merely unaffordable, it is not a target. The Wall shields
/// itself in response and still dies (CR 701.19c), and the four damage goes
/// to *its* controller rather than the spell's — the two life totals are what
/// tell those apart (CR 608.2h, last known controller).
#[test]
fn detonate_kills_a_mana_value_x_artifact_through_a_shield_and_burns_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_artifact(),
                darksteel_pendant(),
            ],
        )
        .battlefield(1, &[living_wall(), forest()])
        .hand(0, &[detonate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, living_wall()).expect("their Wall is out");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let pendant = on_battlefield(&engine, p0, darksteel_pendant()).expect("my Pendant is out");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: their_forest,
            },
        )
        .unwrap();
    activate(&mut engine, p1, living_wall(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine
            .state()
            .object(wall)
            .expect("the Wall is still there")
            .regeneration_shields,
        1,
        "one shield, standing over the Wall"
    );

    cast_from_hand(&mut engine, p0, detonate());
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("\"{{X}}{{R}}\" asks for X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster announces X");
    assert!(
        min <= 4 && 4 <= max,
        "X = 4 is one of the offers: {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(4))
        .expect("the value the question itself enumerated");

    let menu = aim_at(&mut engine, p0, wall);
    assert_eq!(
        menu,
        vec![wall],
        "\"target artifact with mana value X\": only the four is on the menu, \
         not the one or the two: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, living_wall()).is_none(),
        "\"It can't be regenerated\": the shield did not save the Wall"
    );
    assert!(in_graveyard(&engine, p1, living_wall()).is_some());
    assert!(
        engine.state().object(ring).map(|o| o.zone) == Some(Zone::Battlefield),
        "the mana value one artifact was never a target"
    );
    assert!(
        engine.state().object(pendant).map(|o| o.zone) == Some(Zone::Battlefield),
        "nor the mana value two one"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "\"X damage to that artifact's controller\": four to p1"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the caster takes none of its own Detonate"
    );
}
