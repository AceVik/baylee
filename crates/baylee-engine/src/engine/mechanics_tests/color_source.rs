//! A source-color instruction must not select its spell's unrelated target.

use super::*;
use baylee_cards_dsl::{ColorSubject, Duration, Filter, TargetSpec};
use baylee_core::color::{Color, ColorSet};

static COLOR: &[AbilityDef] = &[free(
    &[Effect::ChooseColors {
        subject: ColorSubject::Source,
        max: 5,
        duration: Duration::Indefinitely,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

#[test]
fn choosing_source_colors_changes_the_source_even_with_a_different_target() {
    let mut engine = bench(
        7953,
        vec![
            card(
                7953,
                creature_face("Source", "{0}", 1, 2),
                KeywordSet::EMPTY,
                COLOR,
            ),
            card(
                7954,
                creature_face("Target", "{0}", 1, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[7953, 7954]), Seat::default()],
    );
    to_main(&mut engine, me());
    let source = the(&engine, ZoneLocation::Battlefield, 7953);
    let target = the(&engine, ZoneLocation::Battlefield, 7954);
    activate(&mut engine, me(), source, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        unreachable!()
    };
    let color = ColorSet::of(Color::Blue).union(ColorSet::of(Color::Red));
    let index = options
        .iter()
        .find(|option| option.kind == crate::choice::CastModeKind::Colors(color))
        .unwrap()
        .index;
    engine
        .apply(
            me(),
            PlayerAction::ChooseMode(usize::try_from(index).unwrap()),
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(source)
            .unwrap()
            .characteristics()
            .colors,
        color
    );
    assert_eq!(
        engine
            .state()
            .object(target)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );
}
