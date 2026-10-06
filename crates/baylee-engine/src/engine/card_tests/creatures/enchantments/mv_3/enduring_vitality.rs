//! `cards/creatures/enchantments/mv_3/enduring_vitality.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Enduring Vitality` prints `Vigilance`, `Creatures you control have "{{T}}: Add one mana of any color."`, and an enduring return-on-death trigger.
///
/// Marked `Coverage::Partial`, it carries `KeywordSet::VIGILANCE` and its static ability grants a mana ability to `Filter::YOUR_CREATURE` via `Modifier::GrantActivated`.
/// This offers `PlayerAction::ActivateManaAbility` in `legal.mana_abilities` on both `Enduring Vitality` and a controlled `young_wolf()`, prompting via `Pending::ChooseColor` to produce `ManaColor::Blue`, while an opponent's creature is excluded.
#[test]
fn enduring_vitality_has_vigilance_and_grants_mana_ability_to_controlled_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[enduring_vitality(), young_wolf()])
        .battlefield(1, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vitality =
        on_battlefield(&engine, p0, enduring_vitality()).expect("vitality on battlefield");
    let my_wolf = on_battlefield(&engine, p0, young_wolf()).expect("my wolf on battlefield");
    let their_wolf = on_battlefield(&engine, p1, young_wolf()).expect("their wolf on battlefield");

    assert_eq!(pt(&engine, vitality), (3, 3));
    assert!(keywords(&engine, vitality).contains(KeywordSet::VIGILANCE));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.mana_abilities.contains(&my_wolf),
        "controlled wolf receives granted mana ability in `legal.mana_abilities`"
    );
    assert!(
        legal.mana_abilities.contains(&vitality),
        "vitality also receives its own granted mana ability"
    );
    assert!(
        !legal.mana_abilities.contains(&their_wolf),
        "opponent's creature does not receive granted mana ability"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: my_wolf })
        .unwrap();

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "all five colors can be chosen");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue mana produced by granted mana ability"
    );
    assert!(is_tapped(&engine, my_wolf));
}

/// Enduring Vitality: "When Enduring Vitality dies, if it was a creature,
/// return it to the battlefield under its owner's control. It's an
/// enchantment. (It's not a creature.)"
///
/// The opponent's Lightning Bolt kills it and it comes back as an
/// enchantment only: Soul Warden's "whenever another creature enters" does
/// not see a creature arrive, and the Warden keeps the granted mana ability.
/// The opponent's Maelstrom Pulse then destroys the enchantment, which was
/// not a creature as it died, so it stays in the graveyard.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn enduring_vitality_returns_as_an_enchantment_and_only_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[enduring_vitality(), soul_warden()])
        .battlefield(1, &[mountain(), swamp(), forest(), forest()])
        .hand(1, &[lightning_bolt(), maelstrom_pulse()])
        .start();
    keep_mulligans(&mut engine);
    let vitality = on_battlefield(&engine, p0, enduring_vitality()).expect("Vitality is seated");
    let warden = on_battlefield(&engine, p0, soul_warden()).expect("so is the Warden");

    reach_their_main_phase(&mut engine, p1);
    let life = engine.state().players[0].life;
    let mountain_id = on_battlefield(&engine, p1, mountain()).expect("p1's Mountain");
    tap_mana_where(&mut engine, p1, |id| id == mountain_id);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![vitality],
                players: vec![],
            },
        )
        .expect("the Vitality is a creature to Bolt");
    pass_until(&mut engine, stack_is_empty);

    let back = on_battlefield(&engine, p0, enduring_vitality()).expect("it came back");
    let now = types(&engine, back);
    assert!(
        now.contains(TypeSet::ENCHANTMENT) && !now.contains(TypeSet::CREATURE),
        "an enchantment and not a creature: {now:?}"
    );
    assert_eq!(
        engine.state().players[0].life,
        life,
        "Soul Warden saw no creature enter"
    );
    assert!(walk_to_own_main(&mut engine, p0), "p0's turn comes");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.mana_abilities.contains(&warden),
        "the enchantment still grants the Warden its mana ability"
    );
    assert!(
        !legal.mana_abilities.contains(&back),
        "and no longer itself, which is no creature"
    );

    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, maelstrom_pulse());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![back],
                players: vec![],
            },
        )
        .expect("a nonland permanent");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, enduring_vitality()).is_none(),
        "it died as an enchantment, so it does not return"
    );
    assert!(in_graveyard(&engine, p0, enduring_vitality()).is_some());
}
