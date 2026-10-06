//! `cards/artifacts/vehicles/mv_2/smuggler_s_copter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smuggler's Copter — {2}, a 3/3 Vehicle with flying and an attack trigger:
/// "Whenever this Vehicle attacks or blocks, you may draw a card. If you do,
/// discard a card." Crew 1 is not expressible and is written nowhere on the
/// card, so nothing on it can ever animate it; what is left to play is the
/// cast, the body and the keyword, and then the consequence of the missing
/// cost — the 3/3 flier the combat step never offers as an attacker, against
/// the clean control of an untapped Elf beside it that the same offer does
/// name.
#[test]
fn smugglers_copter_lands_as_a_flying_three_three_the_combat_step_never_offers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1313, forest())
        .battlefield(0, &[forest(), forest(), quiet_creature()])
        .hand(0, &[smugglers_copter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2} off two Forests, with the Elf named as the thing kept back: it is
    // this test's control in the attack declaration below, and a creature
    // tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, smugglers_copter());
    pass_until(&mut engine, stack_is_empty);
    let copter = on_battlefield(&engine, p0, smugglers_copter()).expect("the Copter resolved");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is on the table");

    let kinds = types(&engine, copter);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && !kinds.contains(TypeSet::CREATURE),
        "CR 301.7: a Vehicle is an artifact and nothing else until a crew \
         payment animates it, and Crew 1 has no spelling in this engine: {kinds:?}"
    );
    assert_eq!(pt(&engine, copter), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, copter).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );

    // The gap, and its control. The trigger is `Trigger::Attacks(Filter::This)`,
    // so it wants this permanent in the attack declaration — which wants a
    // creature, which wants the crew cost the card file says cannot be written.
    // The Elf is the control: an untapped creature under the same seat is
    // offered, so an offer without the Copter is the missing crew and not a
    // combat step that never came.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&copter),
        "the 3/3 flier may not: its numbers and its flying are printed, but \
         the only thing that turns a Vehicle into a creature is the crew \
         payment this card cannot carry: {attackers:?}"
    );
}
