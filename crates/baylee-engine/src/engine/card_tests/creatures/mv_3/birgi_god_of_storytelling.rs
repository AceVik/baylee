//! `cards/creatures/mv_3/birgi_god_of_storytelling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Birgi, God of Storytelling` prints `Whenever you cast a spell, add {{R}}. Until end of turn, you don't lose this mana as steps and phases end.`, `Creatures you control can boast twice during each of your turns rather than once.`, and `Discard a card: Exile the top two cards of your library. You may play those cards this turn.`
///
/// Marked `Coverage::Partial`, casting a spell triggers `Trigger::SpellCast` under `Filter::ControlledByYou`, producing one `ManaColor::Red` via `Effect::mana`.
/// Harnfel's discard-to-exile activated ability is omitted from `LegalActions::abilities`.
#[test]
fn birgi_god_of_storytelling_adds_red_mana_on_spell_cast_and_omits_harnfel() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[birgi_god_of_storytelling(), forest(), forest()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let birgi =
        on_battlefield(&engine, p0, birgi_god_of_storytelling()).expect("birgi on battlefield");
    assert_eq!(pt(&engine, birgi), (3, 3));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0
    );

    let my_forests = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: my_forests[0],
            },
        )
        .unwrap();

    cast_with_floating(&mut engine, p0, young_wolf());

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_some(),
        "young wolf resolved"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Birgi added {{R}} on spell cast"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == birgi),
        "under `Coverage::Partial` Harnfel's activated ability is omitted"
    );
}
