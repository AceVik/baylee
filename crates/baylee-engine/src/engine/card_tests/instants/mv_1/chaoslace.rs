//! `cards/instants/mv_1/chaoslace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Chaoslace: "Target spell or permanent becomes red."
#[test]
fn chaoslace_turns_its_target_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), mountain()])
        .hand(0, &[chaoslace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, chaoslace());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("its own Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().colors,
        ColorSet::of(Color::Red),
        "\"becomes red\""
    );
}

/// The five laces ("Target spell or permanent becomes …") and Sink into
/// Stupor ("Return target spell or nonland permanent an opponent controls
/// to its owner's hand"), each cast over an opponent's trigger: the
/// permanent that put it there is a target, the trigger is not. Abilities
/// on the stack aren't spells (CR 113.9), and the menu used to offer them.
#[test]
fn a_spell_or_permanent_instant_never_offers_an_ability_on_the_stack() {
    let p0 = PlayerId::new(0);
    for (card, lands) in [
        (chaoslace(), vec![mountain()]),
        (deathlace(), vec![swamp()]),
        (lifelace(), vec![forest()]),
        (purelace(), vec![plains()]),
        (thoughtlace(), vec![island()]),
        (sink_into_stupor(), vec![island(); 3]),
    ] {
        let name = baylee_cards::by_index(card).map_or("?", baylee_cards_dsl::CardDef::name);
        let (mut engine, trigger, bowmasters) = an_opponents_trigger_on_the_stack(&lands, &[card]);
        cast_from_hand(&mut engine, p0, card);
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!(
                "{name}: expected its target prompt, got {:?}",
                engine.pending()
            )
        };
        assert!(
            options.contains(&bowmasters),
            "{name}: the Bowmasters are a permanent: {options:?}"
        );
        assert!(
            !options.contains(&trigger),
            "{name}: their trigger is no spell and no permanent: {options:?}"
        );
    }
}

/// Chaoslace: "Target spell or permanent becomes red." Aimed at a
/// creature spell on the stack, which the opponent answers it with: the
/// spell turns red while it waits, keeps its mana cost, and the creature
/// it resolves into is still red.
#[test]
fn chaoslace_turns_a_spell_on_the_stack_red_and_the_permanent_stays_red() {
    a_lace_recolours_a_spell(
        chaoslace(),
        mountain(),
        llanowar_elves(),
        forest(),
        Color::Green,
        Color::Red,
    );
}
