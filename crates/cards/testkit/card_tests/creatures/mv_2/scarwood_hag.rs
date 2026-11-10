//! `cards/creatures/mv_2/scarwood_hag.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn scarwood_hag() -> CardIndex {
    card_index("2b483041-4dea-468a-9857-50dae2e8a57d")
}

/// Willow Dryad: {G} 1/1 with forestwalk printed.
fn willow_dryad() -> CardIndex {
    card_index("19d5eb29-2d35-44e7-afa1-3ba8d812ed41")
}

fn float(engine: &mut Engine<RegistryLookup>, color: ManaColor, n: u32) {
    engine.state.players[0].mana_pool.add(color, n);
    engine.refresh_offer();
}

fn has_forestwalk(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    keywords(engine, id).contains(KeywordSet::FORESTWALK)
}

/// Scarwood Hag (2b483041-4dea-468a-9857-50dae2e8a57d) — "{G}{G}{G}{G}, {T}:
/// Target creature gains forestwalk until end of turn."
#[test]
fn scarwood_hag_grants_forestwalk_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scarwood_hag(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let hag = on_battlefield(&engine, P0, scarwood_hag()).expect("the Hag is out");
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("the Elf is out");
    assert!(!has_forestwalk(&engine, elf), "no forestwalk to begin with");

    float(&mut engine, ManaColor::Green, 4);
    activate(&mut engine, P0, scarwood_hag(), 0);
    let menu = aim_at(&mut engine, P0, elf);
    assert!(menu.contains(&elf) && menu.contains(&hag), "any creature");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, hag), "{{T}} was paid");
    assert!(has_forestwalk(&engine, elf), "the Elf gained forestwalk");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{G}}{{G}}{{G}} was spent"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !has_forestwalk(&engine, elf),
        "it wore off at the end of the turn"
    );
}

/// Scarwood Hag — "{G}{G}{G}{G}, {T}: ..." needs all four green.
#[test]
fn scarwood_hag_forestwalk_ability_is_not_offered_without_its_mana() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scarwood_hag(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let hag = on_battlefield(&engine, P0, scarwood_hag()).expect("the Hag is out");
    float(&mut engine, ManaColor::Green, 3);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: hag,
                    ability_index: 0
                }
            )
            .is_err(),
        "three green is one short"
    );
}

/// Scarwood Hag — "{T}: Target creature loses forestwalk until end of turn."
#[test]
fn scarwood_hag_takes_forestwalk_away_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scarwood_hag()])
        .battlefield(1, &[willow_dryad()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let hag = on_battlefield(&engine, P0, scarwood_hag()).expect("the Hag is out");
    let dryad = on_battlefield(&engine, P1, willow_dryad()).expect("the Dryad is out");
    assert!(has_forestwalk(&engine, dryad), "printed forestwalk");

    activate(&mut engine, P0, scarwood_hag(), 1);
    aim_at(&mut engine, P0, dryad);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, hag), "{{T}} was paid");
    assert!(!has_forestwalk(&engine, dryad), "the Dryad lost forestwalk");

    reach_their_main_phase(&mut engine, P1);
    assert!(
        has_forestwalk(&engine, dryad),
        "it is back after the end of the turn"
    );
}

/// Two Hags: whichever effect came later wins (timestamp order). Strip, then
/// grant: the creature has forestwalk. Grant, then strip: it does not.
#[test]
fn scarwood_hags_later_effect_wins() {
    // strip first, grant second
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scarwood_hag(), scarwood_hag()])
        .battlefield(1, &[willow_dryad()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let dryad = on_battlefield(&engine, P1, willow_dryad()).expect("the Dryad is out");
    activate(&mut engine, P0, scarwood_hag(), 1);
    aim_at(&mut engine, P0, dryad);
    pass_until(&mut engine, stack_is_empty);
    assert!(!has_forestwalk(&engine, dryad), "stripped");
    float(&mut engine, ManaColor::Green, 4);
    activate(&mut engine, P0, scarwood_hag(), 0);
    aim_at(&mut engine, P0, dryad);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        has_forestwalk(&engine, dryad),
        "the later grant puts forestwalk back"
    );

    // grant first, strip second
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scarwood_hag(), scarwood_hag(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("the Elf is out");
    float(&mut engine, ManaColor::Green, 4);
    activate(&mut engine, P0, scarwood_hag(), 0);
    aim_at(&mut engine, P0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert!(has_forestwalk(&engine, elf), "granted");
    activate(&mut engine, P0, scarwood_hag(), 1);
    aim_at(&mut engine, P0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !has_forestwalk(&engine, elf),
        "the later strip takes it away"
    );
}
