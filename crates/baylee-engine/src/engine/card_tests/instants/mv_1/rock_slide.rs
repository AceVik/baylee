//! `cards/instants/mv_1/rock_slide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn rock_slide() -> CardIndex {
    card_index("0b2e7c1a-dc07-4138-a560-dc80fa723860")
}

/// `Rock Slide` ({X}{R}, Instant, `Coverage::Implemented`):
/// `Rock Slide deals X damage divided as you choose among any number of target
/// attacking or blocking creatures without flying.`
#[test]
fn rock_slide_deals_x_damage_divided_among_attacking_creatures_without_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[GRIZZLY_BEARS, GRIZZLY_BEARS])
        .hand(0, &[rock_slide()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bears: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).unwrap().controller == p1
                && engine.state().object(id).unwrap().card.map(|c| c.index) == Some(GRIZZLY_BEARS)
        })
        .collect();
    assert_eq!(bears.len(), 2);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears[0], Defender::Player(p0)),
                    (bears[1], Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, rock_slide());
    engine.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: bears.clone(),
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bears[0]).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(bears[1]).unwrap().zone,
        Zone::Graveyard
    );
}
