//! Feroz's Ban — `{6}` Artifact (Homelands)
//!
//! Creature spells cost `{2}` more to cast.

#[allow(clippy::wildcard_imports)]
use super::*;

fn feroz_s_ban() -> CardIndex {
    card_index("445dd350-5165-4516-8669-9233f5558605")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn feroz_s_ban_increases_creature_spell_cost_by_two_and_leaves_noncreature_spells_unchanged() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[bears(), lightning_bolt()])
        .battlefield(
            0,
            &[feroz_s_ban(), forest(), forest(), forest(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears_hand = in_hand(&engine, p0, bears()).unwrap();
    let bolt_hand = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let forests = all_on_battlefield(&engine, p0, forest());
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();

    // Tap only two Forests ({G}{G}): Grizzly Bears ({1}{G} + {2} = {3}{G}) cannot be cast with 2 mana.
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    assert!(
        !priority_offer(&engine).castable.contains(&bears_hand),
        "Grizzly Bears costs {{3}}{{G}} under Feroz's Ban and cannot be cast with {{G}}{{G}}"
    );

    // Tap third Forest + Mountain ({G}{G}{G}{R}, 4 mana total): Grizzly Bears is now castable and consumes all 4 mana!
    tap_mana_where(&mut engine, p0, |id| id == forests[2] || id == mtn);
    assert!(
        priority_offer(&engine).castable.contains(&bears_hand),
        "Grizzly Bears is castable with 4 mana"
    );
    assert!(
        priority_offer(&engine).castable.contains(&bolt_hand),
        "noncreature spell Lightning Bolt still costs {{R}}"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: bears_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, bears()).is_some());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "Grizzly Bears spent all 4 mana ({{1}}{{G}} + {{2}} tax)"
    );
}

/// "Creature spells cost {2} more to cast" taxes every player, not only the
/// one who controls Feroz's Ban: the opponent's Grizzly Bears costs `{3}{G}`
/// on their own turn.
#[test]
fn feroz_s_ban_taxes_the_opponents_creature_spells_too() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[feroz_s_ban()])
        .hand(1, &[bears()])
        .battlefield(1, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bears_hand = in_hand(&engine, p1, bears()).unwrap();
    let forests = all_on_battlefield(&engine, p1, forest());

    // Three Forests would pay the printed {1}{G} twice over, but not {3}{G}.
    tap_mana_where(&mut engine, p1, |id| forests[..3].contains(&id));
    assert!(
        !priority_offer(&engine).castable.contains(&bears_hand),
        "under Feroz's Ban the opponent's Grizzly Bears costs {{3}}{{G}} and three mana is short"
    );

    // The fourth Forest pays the tax.
    tap_mana_where(&mut engine, p1, |id| id == forests[3]);
    assert!(
        priority_offer(&engine).castable.contains(&bears_hand),
        "four mana pays {{3}}{{G}}"
    );
}
