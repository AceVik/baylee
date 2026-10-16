//! `cards/creatures/mv_7/ashen_monstrosity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ashen_monstrosity() -> CardIndex {
    card_index("01906aae-fc53-40b0-a386-dae43e4f437f")
}

/// Ashen Monstrosity — a 7/4 with haste for {5}{R}{R} that "attacks each
/// combat if able". Cast in the main phase, it is offered as an attacker the
/// same turn (haste), the offer names it as required, and a declaration
/// without it is refused; attacking with it puts 7 on the player.
#[test]
fn ashen_monstrosity_has_haste_and_must_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(mountain(), 7), &[ashen_monstrosity()], &[]);
    let monster = cast_and_resolve(&mut engine, ashen_monstrosity());
    assert_body(&engine, monster, (7, 4), KeywordSet::HASTE);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass stopped here")
    };
    assert!(attackers.contains(&monster), "haste: it may attack at once");
    assert!(required.contains(&monster), "and it must");
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "attacking with nothing disobeys the printed requirement"
    );

    fight(&mut engine, &[monster], &[]);
    assert_eq!(life_of(&engine, 1), 13, "7 damage from a 7/4");
}
