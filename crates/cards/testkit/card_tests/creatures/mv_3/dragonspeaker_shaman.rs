//! `cards/creatures/mv_3/dragonspeaker_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn dragonspeaker_shaman_reduces_dragon_spell_costs_by_two() {
    let shaman = card_index("6a29f36b-7911-4d84-a3ae-546b131d5159");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[shaman, mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[index::SHIVAN_DRAGON, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dragon_hand = in_hand(&engine, p0, index::SHIVAN_DRAGON).unwrap();
    let giant_hand = in_hand(&engine, p0, index::HILL_GIANT).unwrap();

    // Tap 3 Mountains: {R}{R}{R} is 1 short of discounted Shivan Dragon ({2}{R}{R}) and 1 short of Hill Giant ({3}{R}).
    let mountains = lands_of(&engine, p0);
    for &m in &mountains[..3] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(!legal.castable.contains(&dragon_hand));
    assert!(!legal.castable.contains(&giant_hand));

    // Tap the 4th Mountain: 4 red mana pays {2}{R}{R} for Shivan Dragon!
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: mountains[3],
            },
        )
        .unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        legal.castable.contains(&dragon_hand),
        "Shivan Dragon costs 2 less with Dragonspeaker Shaman"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: dragon_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::SHIVAN_DRAGON).is_some());
}
