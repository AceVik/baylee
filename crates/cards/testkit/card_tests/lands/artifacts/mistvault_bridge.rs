//! `cards/lands/artifacts/mistvault_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mistvault Bridge prints three lines: it enters tapped, it is
/// indestructible, and it taps for {U} or {B}. The land is *played*, not
/// placed — `starting_battlefield` seeds a permanent without an entry, so a
/// printed `EnterModifier::Tapped` is only visible through a real land drop.
/// The `{T}` line is read a turn later, because a permanent that arrives
/// tapped is not a legal source for its own tap until its controller's next
/// untap step (CR 502.3). And indestructible is only a fact about a game once
/// a destroy effect has named the land and left it standing: the Vindicate
/// sits in its caster's graveyard afterwards, so the survival is the
/// keyword's doing and not a spell that never resolved.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn mistvault_bridge_enters_tapped_taps_for_blue_or_black_and_survives_vindicate() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(11, forest())
        .hand(0, &[mistvault_bridge()])
        .battlefield(1, &[plains(), plains(), swamp(), swamp()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, mistvault_bridge());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\", and a played land is an entry rather \
         than a placement, so the printed modifier is what tapped it"
    );
    let kinds = types(&engine, land);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::ARTIFACT),
        "an artifact land is both: {kinds:?}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land cannot pay a {{T}}, so its own ability is not even \
         offered: {:?}",
        legal.abilities
    );

    // Around the table once: the untap step is where the tap comes off.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "its controller's untap step stands it back up"
    );

    // Ability 0 is the printed "{T}: Add {U} or {B}." Two colours are two
    // answers, so the engine asks; a single producible colour would have been
    // answered silently and this prompt would never appear.
    activate(&mut engine, p0, mistvault_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "both printed halves are on the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else is: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, off the land's own tap"
    );
    assert_eq!(pool.total(), 1, "and nothing else is in the pool");
    assert!(is_tapped(&engine, land), "{{T}} was the price");
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack (CR 605.3b)"
    );

    // Indestructible is only visible against a destroy effect.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"destroy target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&land),
        "the Bridge is a permanent Vindicate may name: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the land the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, mistvault_bridge()).is_some(),
        "the destruction is what indestructible replaces (CR 702.12b), so \
         the Bridge is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, mistvault_bridge()).is_none(),
        "and it is not in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the control: the Vindicate resolved and went to its caster's \
         graveyard, so nothing above is satisfied by a spell that never \
         happened"
    );
}
