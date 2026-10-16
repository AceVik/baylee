//! `cards/creatures/mv_7/guardian_of_cloverdell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn guardian_of_cloverdell() -> CardIndex {
    card_index("27d6b1fb-ce53-45d6-a601-9f5b9ee7ab2f")
}

/// Guardian of Cloverdell — a 4/5 for {5}{G}{G} that makes three 1/1 white
/// Kithkin Soldier tokens when it enters.
#[test]
fn guardian_of_cloverdell_makes_three_kithkin() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 7), &[guardian_of_cloverdell()], &[]);
    let guardian = cast_and_resolve(&mut engine, guardian_of_cloverdell());
    assert_eq!(pt(&engine, guardian), (4, 5));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 3);
    for token in tokens {
        assert_eq!(pt(&engine, token), (1, 1));
    }
}

/// "{G}, Sacrifice a Kithkin: You gain 1 life." Cast with an eighth Forest to
/// spare: the Guardian is not a Kithkin, so only the tokens are on offer, and
/// sacrificing one gains 1.
#[test]
fn guardian_of_cloverdell_sacrifices_a_kithkin_for_life() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 8), &[guardian_of_cloverdell()], &[]);
    let forests = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| forests[..7].contains(&id));
    cast_with_floating(&mut engine, p0, guardian_of_cloverdell());
    pass_until(&mut engine, stack_is_empty);
    let guardian = on_battlefield(&engine, p0, guardian_of_cloverdell()).expect("out");
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 3);

    tap_mana_where(&mut engine, p0, |id| id == forests[7]);
    activate(&mut engine, p0, guardian_of_cloverdell(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the sacrifice is a choice, got {:?}", engine.pending())
    };
    assert!(!options.contains(&guardian), "the Guardian is no Kithkin");
    assert!(
        tokens.iter().all(|t| options.contains(t)),
        "the Soldiers are"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tokens[0]],
            },
        )
        .expect("sacrificing a Kithkin pays");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), 21, "gained 1");
    assert_eq!(tokens_of(&engine, p0).len(), 2, "one Kithkin gone");
}
