//! What a card offers at instant speed, read from its rules (the card DSL),
//! never from its name: a counterspell, instant removal, a permanent with
//! flash. `deck_reactivity4` ranks decks by it and `gamelog4 react` counts
//! with it ([`face_counters`]); the tactical layer reads a counterspell with
//! [`counters`], which looks at one effect list and no deeper.
//!
//! A reading for counting and ranking, not a rule: an exile that blinks the
//! seat's own creature reads as removal, as the tactical layer's hostile
//! reading does.

use baylee_cards_dsl::{AbilityDef, CardDef, Effect, KeywordSet, TargetSpec, Trigger};
use baylee_core::types::TypeSet;

/// Whether `effects` themselves counter a spell or an ability, as the
/// tactical layer reads a counterspell: a counter a payment can stop
/// (`PlayerMayPayOr`) sits inside another effect and is not read here.
#[must_use]
pub fn counters(effects: &[Effect]) -> bool {
    effects.iter().any(|effect| {
        matches!(
            effect,
            Effect::CounterTargetSpell
                | Effect::CounterTargetSpellToExile
                | Effect::CounterTargetSpellOrAbility
                | Effect::CounterTargetAbility
        )
    })
}

/// Whether `spec` reaches a permanent on the battlefield.
const fn reaches_permanents(spec: &TargetSpec) -> bool {
    matches!(
        spec,
        TargetSpec::Object(_)
            | TargetSpec::ObjectOfEachOpponent(_)
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::ObjectOfFirstTargetsPlayer(_)
            | TargetSpec::ObjectControlledBy(..)
            | TargetSpec::ObjectOfEventPlayer(_)
            | TargetSpec::StackOrBattlefield(_)
            | TargetSpec::AnyTarget
            | TargetSpec::PlayerOrObject(_)
    )
}

/// Whether an effect reachable from `effects` (`Effect::walk`: through a
/// payment, a choice, a condition) is one `read` accepts.
fn anywhere(effects: &'static [Effect], read: impl Fn(&Effect) -> bool) -> bool {
    let mut found = false;
    let mut seen = 0;
    Effect::walk(effects, &mut seen, &mut |effect| found |= read(effect));
    found
}

/// Whether `effects` counter a spell or an ability anywhere inside them: a
/// hard counter, or one a payment can stop.
#[must_use]
pub fn counters_anywhere(effects: &'static [Effect]) -> bool {
    anywhere(effects, |effect| counters(std::slice::from_ref(effect)))
}

/// Whether `effects` take a permanent out of the game's reach or kill it,
/// anywhere inside them: destroy, exile, damage, a fight, a bounce, a tuck, a
/// steal or a shrink, one target or all that match.
#[must_use]
pub fn removes(effects: &'static [Effect]) -> bool {
    anywhere(effects, removes_one)
}

fn removes_one(effect: &Effect) -> bool {
    match effect {
        Effect::Destroy { target, .. }
        | Effect::Exile { target }
        | Effect::DealDamage { target, .. }
        | Effect::ReturnToHand { target } => reaches_permanents(target),
        Effect::DestroyTargetSlots { .. }
        | Effect::ExileTargetsCreateTokens { .. }
        | Effect::DealDamageDivided { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::Fight { .. }
        | Effect::PutTargetOnTopOfLibrary
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::ChangeController { .. }
        | Effect::DestroyAll { .. }
        | Effect::ExileAll { .. }
        | Effect::ReturnAllToHand { .. } => true,
        Effect::PumpTarget {
            power, toughness, ..
        }
        | Effect::PumpTargetSlot {
            power, toughness, ..
        } => power.is_negative() || toughness.is_negative(),
        _ => false,
    }
}

/// The effects a face's spell can have: its spell's, and each mode's.
fn spell_effects(abilities: &'static [AbilityDef]) -> impl Iterator<Item = &'static [Effect]> {
    abilities
        .iter()
        .flat_map(|ability| -> Vec<&'static [Effect]> {
            match ability {
                AbilityDef::Spell { effects, .. } => vec![effects],
                AbilityDef::ModalSpell { modes, .. } => {
                    modes.iter().map(|mode| mode.effects).collect()
                }
                _ => Vec::new(),
            }
        })
}

/// The effects of a face's own entering: what a flash creature does as it lands.
fn entering_effects(abilities: &'static [AbilityDef]) -> impl Iterator<Item = &'static [Effect]> {
    abilities.iter().filter_map(|ability| match ability {
        AbilityDef::Triggered {
            trigger: Trigger::ETB,
            effects,
            ..
        } => Some(*effects),
        _ => None,
    })
}

/// Whether face `face` of `def` counters: its spell or a mode of it, or, on a
/// permanent with flash, what it does as it enters.
#[must_use]
pub fn face_counters(def: &CardDef, face: usize) -> bool {
    let abilities = def.abilities_for_face(face);
    spell_effects(abilities).any(counters_anywhere)
        || (def.keywords_for_face(face).contains(KeywordSet::FLASH)
            && entering_effects(abilities).any(counters_anywhere))
}

/// Whether face `face` of `def` has a mana ability: a land, a rock, a dork.
#[must_use]
pub fn face_makes_mana(def: &CardDef, face: usize) -> bool {
    def.faces
        .get(face)
        .is_some_and(|f| f.types.contains(TypeSet::LAND))
        || def
            .abilities_for_face(face)
            .iter()
            .any(AbilityDef::is_mana_ability)
}

/// What one card offers at instant speed. A card may be several.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Independent readings of one card"
)]
pub struct Reactivity {
    /// A face is an instant.
    pub instant: bool,
    /// A face counters a spell or an ability (`face_counters`).
    pub counter: bool,
    /// An instant face removes a permanent (`removes`).
    pub removal: bool,
    /// A face that is not an instant has flash.
    pub flash: bool,
}

impl Reactivity {
    /// The reading of `def`, over the faces a player can cast from hand.
    #[must_use]
    pub fn of(def: &CardDef) -> Self {
        let mut out = Self::default();
        for (index, face) in def.faces.iter().enumerate() {
            if index > 0 && !face.castable_from_hand {
                continue;
            }
            if face.types.contains(TypeSet::LAND) {
                continue;
            }
            let instant = face.types.contains(TypeSet::INSTANT);
            out.instant |= instant;
            out.counter |= face_counters(def, index);
            out.removal |= instant && spell_effects(def.abilities_for_face(index)).any(removes);
            out.flash |= !instant && def.keywords_for_face(index).contains(KeywordSet::FLASH);
        }
        out
    }

    /// Whether the card interacts at instant speed in any way counted here.
    #[must_use]
    pub const fn interacts(self) -> bool {
        self.counter || self.removal || self.flash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str) -> &'static CardDef {
        let index = baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name}"));
        baylee_cards::by_index(index).unwrap_or_else(|| panic!("{name} has no rules"))
    }

    #[test]
    fn a_counterspell_reads_as_one() {
        let reading = Reactivity::of(card("Counterspell"));
        assert!(reading.counter && reading.instant);
        assert!(!reading.removal && !reading.flash);
    }

    #[test]
    fn narrow_counters_and_a_modal_one_read_as_counters() {
        for name in ["Negate", "Essence Scatter", "Cancel", "Izzet Charm"] {
            assert!(Reactivity::of(card(name)).counter, "{name}");
        }
        // The charm's other mode deals damage to a creature.
        assert!(Reactivity::of(card("Izzet Charm")).removal);
    }

    /// The layer's own reading stays one effect list deep, as it always read:
    /// a counter a payment can stop is the scan's and the count's, not the layer's.
    #[test]
    fn the_layer_reading_stays_one_list_deep() {
        let AbilityDef::ModalSpell { modes, .. } = &card("Izzet Charm").abilities[0] else {
            panic!("Izzet Charm is a modal spell");
        };
        assert!(!counters(modes[0].effects));
        assert!(counters_anywhere(modes[0].effects));
    }

    /// Mystic Snake counters as it enters, which it can do because it has flash.
    #[test]
    fn a_flash_creature_that_counters_on_entering_reads_as_both() {
        let reading = Reactivity::of(card("Mystic Snake"));
        assert!(reading.counter && reading.flash && !reading.instant);
    }

    #[test]
    fn instant_removal_reads_as_removal() {
        for name in ["Lightning Bolt", "Murder", "Unsummon", "Doom Blade"] {
            let reading = Reactivity::of(card(name));
            assert!(reading.removal && reading.instant, "{name}");
            assert!(!reading.counter, "{name}");
        }
    }

    #[test]
    fn sorcery_speed_spells_do_not_interact() {
        for name in ["Flame Slash", "Divination"] {
            assert!(!Reactivity::of(card(name)).interacts(), "{name}");
        }
    }

    #[test]
    fn a_flash_creature_reads_as_flash_and_a_vanilla_one_as_nothing() {
        assert!(Reactivity::of(card("Ambush Viper")).flash);
        assert!(!Reactivity::of(card("Grizzly Bears")).interacts());
    }

    #[test]
    fn a_land_and_a_rock_make_mana() {
        assert!(face_makes_mana(card("Island"), 0));
        assert!(face_makes_mana(card("Mind Stone"), 0));
        assert!(!face_makes_mana(card("Counterspell"), 0));
    }
}
