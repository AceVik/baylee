//! `cards/creatures/mv_2/orcish_bowmasters.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Orcish Bowmasters: "deals 1 damage to any target. Then amass Orcs 1."
///
/// The card was written with the damage pointed at `PlayerRel::Opponent` and
/// no target requirement on the ability at all, so the arrow was never aimed
/// — the engine reads the *ability's* requirement and the effect's own
/// `target` field is not what it asks about. At a duel that is invisible
/// (there is one opponent, and they were hit either way); at a four-player
/// table it picked one, and it could never hit a creature.
#[test]
fn the_bowmasters_aim_where_their_controller_points() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[orcish_bowmasters()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    let before = engine.state().players[1].life;
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("two Swamps pay {1}{B}");

    assert!(
        settle_aiming_at(&mut engine, p1),
        "the enters trigger asks where its arrow goes"
    );
    assert_eq!(
        engine.state().players[1].life,
        before - 1,
        "and the chosen face takes the damage"
    );
}

/// The same ability's other trigger. It is one printed ability with two
/// triggers and the engine has no variant for that, so the card writes it
/// twice — and the second copy was written without the damage, so an
/// opponent's extra draw amassed an Orc and fired no arrow.
///
/// Mikokoro makes both players draw on *this* turn, which is outside the
/// opponent's draw step, so it is never their excepted first draw.
#[test]
fn an_opponents_extra_draw_fires_an_arrow_as_well_as_an_orc() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(12, island())
        .battlefield(0, &[orcish_bowmasters(), mikokoro(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, mikokoro()).expect("Mikokoro");
    tap_mana_except(&mut engine, p0, well);
    let before = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: well,
                ability_index: 1,
            },
        )
        .expect("two Swamps pay the {2}");

    assert!(
        settle_aiming_at(&mut engine, p1),
        "the draw trigger asks where its arrow goes"
    );
    assert_eq!(
        engine.state().players[1].life,
        before - 1,
        "an opponent's extra draw costs them a life, not only a token"
    );
}
