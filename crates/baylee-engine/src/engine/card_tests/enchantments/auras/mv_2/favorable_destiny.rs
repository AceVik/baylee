//! `cards/enchantments/auras/mv_2/favorable_destiny.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, SERRA_ANGEL};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn favorable_destiny() -> CardIndex {
    card_index("ee04afb0-184f-4a13-8d44-0fbe121b1eff")
}

fn has_shroud(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .characteristics()
        .keywords
        .contains(KeywordSet::SHROUD)
}

/// Favorable Destiny ({1}{W} Aura) cast by P0 on `target`, which `owner`
/// controls, with the named boards beside P0's two Plains. Returns the engine
/// once the Aura has resolved, and the enchanted creature.
fn enchant(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    target: CardIndex,
    owner: PlayerId,
) -> (Engine<RegistryLookup>, ObjectId) {
    let mut field = vec![plains(), plains()];
    field.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &field)
        .battlefield(1, theirs)
        .hand(0, &[favorable_destiny()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let aimed = on_battlefield(&engine, owner, target).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, favorable_destiny());
    aim_at(&mut engine, P0, aimed);
    pass_until(&mut engine, stack_is_empty);
    (engine, aimed)
}

/// "Enchanted creature gets +1/+2 as long as it's white." and "Enchanted
/// creature has shroud as long as its controller controls another creature."
/// The enchanted white Angel (4/4) becomes 5/6, and with Bears beside it, it
/// has shroud.
#[test]
fn favorable_destiny_grants_plus_one_plus_two_and_shroud() {
    let (engine, angel) = enchant(&[SERRA_ANGEL, GRIZZLY_BEARS], &[], SERRA_ANGEL, P0);

    assert_eq!(pt(&engine, angel), (5, 6));
    assert!(has_shroud(&engine, angel));
}

/// The shroud needs "another creature": the Angel alone is enchanted and
/// gets the +1/+2, but no shroud.
#[test]
fn favorable_destiny_gives_no_shroud_without_another_creature() {
    let (engine, angel) = enchant(&[SERRA_ANGEL], &[], SERRA_ANGEL, P0);

    assert_eq!(pt(&engine, angel), (5, 6));
    assert!(!has_shroud(&engine, angel));
}

/// "+1/+2 as long as it's white": a green Grizzly Bears (2/2) gets neither
/// the bonus, and keeps the shroud that depends only on the other creature.
#[test]
fn favorable_destiny_gives_the_bonus_only_to_a_white_creature() {
    let (engine, bears) = enchant(&[GRIZZLY_BEARS, SERRA_ANGEL], &[], GRIZZLY_BEARS, P0);

    assert_eq!(pt(&engine, bears), (2, 2));
    assert!(has_shroud(&engine, bears));
}

/// The Aura on an opponent's creature gives that creature no shroud: the
/// card as written counts only the Aura controller's other creatures, which
/// is not "its controller" for a creature the Aura's controller does not
/// control (the card is Partial for exactly this case).
#[test]
fn favorable_destiny_on_an_opponents_creature_gives_it_no_shroud() {
    let (engine, bears) = enchant(
        &[SERRA_ANGEL, GRIZZLY_BEARS],
        &[GRIZZLY_BEARS, GRIZZLY_BEARS],
        GRIZZLY_BEARS,
        P1,
    );

    assert!(!has_shroud(&engine, bears));
    assert_eq!(pt(&engine, bears), (2, 2));
}
