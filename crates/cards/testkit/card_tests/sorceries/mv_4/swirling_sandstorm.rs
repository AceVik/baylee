//! `cards/sorceries/mv_4/swirling_sandstorm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn swirling_sandstorm_deals_five_damage_to_each_creature_without_flying_at_threshold() {
    let sandstorm = card_index("dc5cb229-82c9-4a87-9a10-490350cbdfc0");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                aven_flock(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[sandstorm])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 7);

    for m in all_on_battlefield(&engine, p0, mountain()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, sandstorm).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(
        on_battlefield(&engine, p0, aven_flock()).is_some(),
        "creature with flying is unharmed"
    );
}
