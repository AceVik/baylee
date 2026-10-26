//! `cards/creatures/mv_3/disciple_of_kangee.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

/// Disciple of Kangee — {2}{W} Creature — Human Wizard 2/2
/// "{U}, {T}: Target creature gains flying and becomes blue until end of turn."
#[test]
fn disciple_of_kangee_grants_flying_and_sets_target_creature_color_to_blue_until_end_of_turn() {
    let disciple = card_index("778fb830-2d2b-4f81-a193-e6b652981695");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[disciple, index::GRIZZLY_BEARS, island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let disciple_id = on_battlefield(&engine, p0, disciple).expect("Disciple of Kangee on board");
    let bears_id = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: disciple_id,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(bears_id)
        .unwrap()
        .characteristics()
        .clone();
    assert!(
        chars.keywords.contains(KeywordSet::FLYING),
        "target creature gained flying"
    );
    assert_eq!(
        chars.colors,
        ColorSet::of(Color::Blue),
        "target creature became blue (overwriting green)"
    );

    assert!(walk_to_own_main(&mut engine, p1));
    let chars_after = engine
        .state()
        .object(bears_id)
        .unwrap()
        .characteristics()
        .clone();
    assert!(!chars_after.keywords.contains(KeywordSet::FLYING));
    assert_eq!(chars_after.colors, ColorSet::of(Color::Green));
}
