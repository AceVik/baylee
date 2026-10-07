//! `cards/lands/utility/tomb_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tomb Fortress` prints `This land enters tapped.`, `{{T}}: Add {{B}}.`, and `{{2}}{{B}}{{B}}{{B}}, {{T}}, Exile this land: Mill four cards, then return a creature card from your graveyard to the battlefield. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, `EnterModifier::Tapped` and the `{{T}}: Add {{B}}` mana ability are built, while the reanimation activation is omitted because returning an untargeted card from the graveyard has no DSL effect.
/// Playing `Tomb Fortress` from hand puts it onto the battlefield tapped; after advancing to the next turn to untap, floating `{{2}}{{B}}{{B}}{{B}}` shows that ability 0 is offered while ability 1 is withheld under `Coverage::Partial`, and tapping it adds `{{B}}`.
#[test]
fn tomb_fortress_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[tomb_fortress()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fortress = play_land(&mut engine, p0, tomb_fortress());
    assert!(
        entered_tapped(&engine, fortress),
        "tomb fortress enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, fortress));

    // Float {{2}}{{B}}{{B}}{{B}} (5 black mana) from basic swamps while keeping Tomb Fortress untapped.
    tap_mana_except(&mut engine, p0, fortress);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        5
    );
    assert!(!is_tapped(&engine, fortress));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(fortress, 0)),
        "ability 0 ({{T}}: Add {{B}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(fortress, 1)),
        "reanimation ability is omitted under `Coverage::Partial` despite floating {{2}}{{B}}{{B}}{{B}}"
    );

    activate(&mut engine, p0, tomb_fortress(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 6);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, fortress));
}
