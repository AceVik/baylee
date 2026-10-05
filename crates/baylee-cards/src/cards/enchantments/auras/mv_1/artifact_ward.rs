//! Artifact Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by artifact creatures.
//! Oracle: Prevent all damage that would be dealt to enchanted creature by artifact sources.
//! Oracle: Enchanted creature can't be the target of abilities from artifact sources.
//! Set: ATQ #3 — Antiquities | Scryfall ID: b3a5101a-ec66-4658-950c-9ad49c29b836 | Oracle ID: 9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db
// PARTIAL — enchant creature and the artifact-creature block restriction are
// written; the damage prevention and the abilities-only targeting restriction
// are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Artifact creature" — the blocker the second printed sentence names.
static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::ARTIFACT_WARD,
    oracle_id = "9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db",
    scryfall_id = "b3a5101a-ec66-4658-950c-9ad49c29b836",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no static prevention keyed to a damage source's characteristics \
         (`Modifier::PreventDamageToIt` is combat damage only) and no \
         abilities-only targeting restriction (`Modifier::CantBeTargetedBy` \
         also stops artifact spells), so both sentences are off the card"
    ),
    faces = &[face!(
        name = "Artifact Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Prevent all damage that would be dealt to enchanted
    // creature by artifact sources." — the only prevention modifiers are
    // `PreventDamageToIt`, which stops combat damage from every source and no
    // noncombat damage at all, and `ProtectionFrom`, which additionally stops
    // targeting and blocking; neither is this sentence.
    // NOT SUPPORTED: "Enchanted creature can't be the target of abilities from
    // artifact sources." — `Modifier::CantBeTargetedBy` matches the spell or
    // the ability's source and so also stops artifact *spells*, which the card
    // does not; no filter can say "abilities only".
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::CantBeBlockedBy(&ARTIFACT_CREATURE)
        ),
    ],
);
