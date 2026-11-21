use super::*;
use baylee_cards_dsl::{Amount, CopyCleanup, PlayerRel, TargetSpec, Trigger, prelude::triggered};

const SOURCE: u32 = 99_950;

/// A permanent can have a level even without the Class subtype (CR 717.2b).
/// Free activations let two assignments to the same level resolve in sequence.
static ABILITIES: &[AbilityDef] = &[
    free(&[Effect::SetClassLevel(2)], None),
    free(&[Effect::SetClassLevel(3)], None),
    free(
        &[Effect::CreateTokenCopy {
            target: TargetSpec::ThisObject,
            controller: PlayerRel::You,
            amount: Amount::Fixed(1),
            mods: &[],
            tapped: false,
            attacking: false,
            haste_until_eot: false,
            cleanup: CopyCleanup::None,
        }],
        None,
    ),
    free(
        &[Effect::Blink {
            target: TargetSpec::ThisObject,
            owner_control: true,
        }],
        None,
    ),
    triggered!(Trigger::ClassLevelGained(2), &[Effect::gain_life(1)]),
];

fn game() -> (Bench, ObjectId) {
    let source = card(
        SOURCE,
        face("Class level probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ABILITIES,
    );
    let mut engine = bench(42, vec![source], [Seat::with(&[SOURCE]), Seat::with(&[])]);
    to_main(&mut engine, me());
    let source = the(&engine, ZoneLocation::Battlefield, SOURCE);
    (engine, source)
}

#[test]
fn class_level_is_set_once_and_is_not_a_copiable_value() {
    let (mut engine, source) = game();
    let before = life(&engine, me());
    for _ in 0..2 {
        activate(&mut engine, me(), source, 0);
        settle(&mut engine, me());
    }
    assert_eq!(life(&engine, me()), before + 1);
    assert_eq!(engine.state.object(source).unwrap().class_level(), 2);
    activate(&mut engine, me(), source, 2);
    settle(&mut engine, me());
    let copy = engine
        .state
        .battlefield_view()
        .into_iter()
        .find(|&id| !engine.state.object(id).unwrap().is_card())
        .expect("a token copy");
    assert_eq!(engine.state.object(copy).unwrap().class_level(), 1);
    assert_eq!(engine.state.object(source).unwrap().class_level(), 2);
}

#[test]
fn a_pending_level_ability_cannot_level_the_returned_permanent() {
    let (mut engine, source) = game();
    activate(&mut engine, me(), source, 0);
    settle(&mut engine, me());
    let incarnation = engine.state.object(source).unwrap().version;
    activate(&mut engine, me(), source, 1);
    activate(&mut engine, me(), source, 3);
    settle(&mut engine, me());
    let returned = engine.state.object(source).unwrap();
    assert_ne!(returned.version, incarnation);
    assert_eq!(returned.class_level(), 1);
}
