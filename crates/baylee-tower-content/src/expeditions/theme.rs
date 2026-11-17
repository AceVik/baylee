//! Mandatory thematic cores prevent generic role scoring from abandoning a leader.
use baylee_cards::{
    deckgen::{
        DeckGenError, Entry, Pool,
        profile::{
            MASK_INTERACTION, MASK_RAMP, ROLE_BLINK_ETB, ROLE_COUNTERS, ROLE_DEATH_PAYOFF,
            ROLE_GY_FILLER, ROLE_GY_PAYOFF, ROLE_LIFEGAIN, ROLE_SAC_OUTLET, ROLE_TOKENS,
        },
    },
    dsl::{AbilityDef, CounterKind, Effect},
};
use baylee_core::{
    generated::subtypes,
    ids::CardIndex,
    types::{SupertypeSet, TypeSet},
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    Tokens,
    Life,
    Spells,
    Spellcraft,
    Saprolings,
    Artifacts,
    Mana,
    Equipment,
    Negative,
    CatsDogs,
    Angels,
    Elves,
    Goblins,
    Enchantments,
    Graveyard,
    Returns,
    Combat,
    Control,
    Sacrifice,
    Legends,
    Counters,
}

fn theme(name: &str) -> Theme {
    match name {
        "Emmara, Soul of the Accord"
        | "Maja, Bretagard Protector"
        | "Arasta of the Endless Web" => Theme::Tokens,
        "Elas il-Kor, Sadistic Pilgrim"
        | "Ayara, First of Locthwain"
        | "Kambal, Consul of Allocation"
        | "Linden, the Steadfast Queen" => Theme::Life,
        "Talrand, Sky Summoner"
        | "Kykar, Wind's Fury"
        | "Tura Kennerüd, Skyknight"
        | "Birgi, God of Storytelling" => Theme::Spells,
        "Galazeth Prismari" => Theme::Spellcraft,
        "Slimefoot, the Stowaway" => Theme::Saprolings,
        "Pia and Kiran Nalaar"
        | "Padeem, Consul of Innovation"
        | "Glissa, the Traitor"
        | "Emry, Lurker of the Loch" => Theme::Artifacts,
        "Radha, Heir to Keld"
        | "Ashaya, Soul of the Wild"
        | "Titania, Protector of Argoth"
        | "Tatyova, Benthic Druid" => Theme::Mana,
        "Rishkar, Peema Renegade" => Theme::Counters,
        "Sram, Senior Edificer" => Theme::Equipment,
        "Hapatra, Vizier of Poisons" => Theme::Negative,
        "Rin and Seri, Inseparable" => Theme::CatsDogs,
        "Giada, Font of Hope" | "Lyra Dawnbringer" => Theme::Angels,
        "Dwynen, Gilt-Leaf Daen" => Theme::Elves,
        "Pashalik Mons" => Theme::Goblins,
        "Alela, Artful Provocateur"
        | "Sythis, Harvest's Hand"
        | "Ghen, Arcanum Weaver"
        | "Eutropia the Twice-Favored" => Theme::Enchantments,
        "Kess, Dissident Mage" | "Muldrotha, the Gravetide" => Theme::Graveyard,
        "Emiel the Blessed" | "Derevi, Empyrial Tactician" | "Garna, the Bloodflame" => {
            Theme::Returns
        }
        "Judith, the Scourge Diva"
        | "Yahenni, Undying Partisan"
        | "Mahadi, Emporium Master"
        | "Grist, the Hunger Tide" => Theme::Sacrifice,
        "Vona, Butcher of Magan"
        | "Chevill, Bane of Monsters"
        | "Ertai Resurrected"
        | "Kaervek, the Spiteful" => Theme::Control,
        "Arvad the Cursed" => Theme::Legends,
        _ => Theme::Combat,
    }
}

impl Theme {
    fn fits(self, entry: &Entry) -> bool {
        let role = |mask| entry.profile.roles & mask != 0;
        let tribal =
            |kind: baylee_core::ids::SubtypeId| entry.profile.subtypes.contains(&kind.get());
        match self {
            Self::Tokens => role(ROLE_TOKENS),
            Self::Life => role(ROLE_LIFEGAIN | ROLE_DEATH_PAYOFF) && entry.creature,
            Self::Spells => entry
                .profile
                .types
                .intersects(TypeSet::INSTANT.union(TypeSet::SORCERY)),
            Self::Spellcraft => entry.profile.types.intersects(
                TypeSet::INSTANT
                    .union(TypeSet::SORCERY)
                    .union(TypeSet::ARTIFACT),
            ),
            Self::Artifacts => entry.profile.types.contains(TypeSet::ARTIFACT),
            Self::Mana => role(MASK_RAMP),
            Self::Equipment => entry.profile.is_equipment_or_aura,
            Self::CatsDogs => tribal(subtypes::creature::CAT) || tribal(subtypes::creature::DOG),
            Self::Angels => tribal(subtypes::creature::ANGEL),
            Self::Elves => tribal(subtypes::creature::ELF),
            Self::Goblins => tribal(subtypes::creature::GOBLIN),
            Self::Enchantments => entry.profile.types.contains(TypeSet::ENCHANTMENT),
            Self::Graveyard => role(ROLE_GY_FILLER | ROLE_GY_PAYOFF),
            Self::Returns => entry.creature && role(ROLE_BLINK_ETB),
            Self::Combat => entry.creature,
            Self::Control => role(MASK_INTERACTION),
            Self::Sacrifice => {
                entry.creature && role(ROLE_SAC_OUTLET | ROLE_DEATH_PAYOFF | ROLE_TOKENS)
            }
            Self::Legends => {
                entry.creature && entry.profile.supertypes.contains(SupertypeSet::LEGENDARY)
            }
            Self::Counters => entry.creature && role(ROLE_COUNTERS),
            Self::Negative | Self::Saprolings => specific_effect(entry.card, self),
        }
    }
}

pub(super) fn core(
    pool: &Pool,
    commander: CardIndex,
    name: &str,
    seed: u64,
) -> Result<Vec<CardIndex>, DeckGenError> {
    let identity = baylee_cards::by_index(commander)
        .ok_or_else(|| DeckGenError("unknown commander".into()))?
        .color_identity;
    let theme = theme(name);
    let candidates: Vec<_> = pool
        .entries
        .iter()
        .filter(|entry| {
            entry.card != commander
                && !entry.land
                && entry.mana_value <= 6
                && entry.identity.difference(identity).is_empty()
                && !entry.profile.has_external_named_dependency
                && theme.fits(entry)
        })
        .collect();
    if candidates.len() < 6 {
        return Err(DeckGenError(format!(
            "{name} has only {} supported on-theme spells",
            candidates.len()
        )));
    }
    let mut selected = BTreeSet::new();
    if theme == Theme::Spellcraft {
        for (spells_half, targets) in [(true, [1, 2, 2, 3, 3, 4]), (false, [1, 2, 2, 3, 3, 4])] {
            for target in targets {
                let chosen = candidates
                    .iter()
                    .filter(|entry| {
                        !selected.contains(&entry.card)
                            && if spells_half {
                                entry
                                    .profile
                                    .types
                                    .intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
                            } else {
                                entry.profile.types.contains(TypeSet::ARTIFACT)
                                    && !entry.profile.is_equipment_or_aura
                            }
                    })
                    .min_by_key(|entry| {
                        let mixed = seed
                            .wrapping_add(u64::from(entry.card.get()))
                            .wrapping_mul(0x9e37_79b9_7f4a_7c15);
                        (entry.mana_value.abs_diff(target), mixed, entry.card)
                    });
                if let Some(entry) = chosen {
                    selected.insert(entry.card);
                }
            }
        }
        return Ok(selected.into_iter().collect());
    }
    for target in [1, 2, 2, 2, 3, 3, 3, 4, 4, 5] {
        let chosen = candidates
            .iter()
            .filter(|entry| !selected.contains(&entry.card))
            .min_by_key(|entry| {
                // Deterministic variety within each curve bucket, no wall clock or map order.
                let mixed = seed
                    .wrapping_add(u64::from(entry.card.get()))
                    .wrapping_mul(0x9e37_79b9_7f4a_7c15);
                (entry.mana_value.abs_diff(target), mixed, entry.card)
            });
        if let Some(entry) = chosen {
            selected.insert(entry.card);
        }
    }
    Ok(selected.into_iter().collect())
}

fn specific_effect(card: CardIndex, theme: Theme) -> bool {
    let Some(definition) = baylee_cards::by_index(card) else {
        return false;
    };
    let mut matched = false;
    let mut inspect = |effects: &'static [Effect]| {
        Effect::walk(effects, &mut 0, &mut |effect| {
            matched |= match (theme, effect) {
                (
                    Theme::Negative,
                    Effect::AddCounter { kind, .. }
                    | Effect::AddCounterFilter { kind, .. }
                    | Effect::AddCounterSource { kind, .. },
                ) => *kind == CounterKind::M1M1,
                (
                    Theme::Saprolings,
                    Effect::CreateToken { token }
                    | Effect::CreateTokenN { token, .. }
                    | Effect::CreateTokensFor { token, .. },
                ) => token.subtypes.contains(&subtypes::creature::SAPROLING),
                _ => false,
            };
        });
    };
    for ability in definition.abilities_for_face(0) {
        match ability {
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Triggered { effects, .. }
            | AbilityDef::Spell { effects, .. }
            | AbilityDef::Loyalty { effects, .. }
            | AbilityDef::SagaChapter { effects, .. } => inspect(effects),
            AbilityDef::ModalSpell { modes, .. }
            | AbilityDef::ModalActivated { modes, .. }
            | AbilityDef::ModalTriggered { modes, .. } => {
                for mode in *modes {
                    inspect(mode.effects);
                }
            }
            _ => {}
        }
    }
    matched
}
