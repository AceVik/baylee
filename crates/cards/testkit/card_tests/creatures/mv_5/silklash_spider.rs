//! `cards/creatures/mv_5/silklash_spider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Silklash Spider (`2d0c73da-ed8c-44fc-abd9-7c344b1f890d`) — {3}{G}{G} Creature — Spider 2/7:
/// "Reach
/// {X}{G}{G}: This creature deals X damage to each creature with flying."
#[test]
fn silklash_spider_deals_x_damage_to_each_creature_with_flying() {
    let spider_card = card_index("2d0c73da-ed8c-44fc-abd9-7c344b1f890d");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                spider_card,
                index::GRIZZLY_BEARS,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[index::PHANTOM_MONSTER, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spider = on_battlefield(&engine, p0, spider_card).unwrap();
    assert!(keywords(&engine, spider).contains(KeywordSet::REACH));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, spider_card, 0);
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Phantom Monster (3/3 flying) takes 3 damage and dies; non-flyers take 0 damage.
    assert!(in_graveyard(&engine, p1, index::PHANTOM_MONSTER).is_some());
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_some());
}
