//! `cards/creatures/mv_5/martyrs_of_korlis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Martyrs ruling: "If you have multiple Martyrs of Korlis, you can
/// decide which one receives the redirected damage each time artifact damage
/// would be dealt to you." Two untapped copies make two applicable
/// replacement effects, so the affected player chooses among them (CR 616.1)
/// and only the named one is dealt the point.
#[test]
fn martyrs_of_korlis_lets_its_controller_choose_between_two_copies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[martyrs_of_korlis(), martyrs_of_korlis()])
        .battlefield(1, &[copper_tablet()])
        .start();
    keep_mulligans(&mut engine);
    let copies = all_on_battlefield(&engine, p0, martyrs_of_korlis());
    assert_eq!(copies.len(), 2, "both copies are out");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    let Pending::ChooseDamageEffect {
        player,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(
        player, p0,
        "the player the damage would be dealt to chooses"
    );
    let destinations: Vec<ObjectId> = options
        .iter()
        .filter_map(|option| match option.kind {
            crate::choice::DamageEffectKind::Redirect {
                to: crate::event::DamageTarget::Object(id),
            } => Some(id),
            _ => None,
        })
        .collect();
    for copy in &copies {
        assert!(
            destinations.contains(copy),
            "both copies offer themselves: {destinations:?}"
        );
    }
    let chosen = copies[1];
    let effect = options
        .iter()
        .find(|option| {
            matches!(
                option.kind,
                crate::choice::DamageEffectKind::Redirect {
                    to: crate::event::DamageTarget::Object(id),
                } if id == chosen
            )
        })
        .expect("the option that names the chosen copy")
        .id;
    engine
        .apply(p0, PlayerAction::ChooseDamageEffect { choice, effect })
        .expect("the offer's own option answers it");
    pass_until(&mut engine, |e| {
        at_rest(e, p0) && e.state().turn.step == Step::Upkeep
    });

    assert_eq!(engine.state().players[0].life, 20, "the player took none");
    assert_eq!(
        engine.state().object(chosen).map(|o| o.damage),
        Some(1),
        "the copy the controller named took the point"
    );
    assert_eq!(
        engine.state().object(copies[0]).map(|o| o.damage),
        Some(0),
        "and the other copy took nothing"
    );
}
