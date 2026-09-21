//! Ishgard, the Holy See // Faith & Grief — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: Return up to two target artifact and/or enchantment cards from your graveyard to your hand. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #283 — Final Fantasy | Scryfall ID: 068bc755-9d3d-430b-abc5-c775a5415bf9 | Oracle ID: 4f4358cb-59df-46d9-be27-69929f5a615c
//! Face: Ishgard, the Holy See —  — Land — Town
//! Face: Faith & Grief — {3}{W}{W} — Sorcery — Adventure
// PARTIAL — the land enters tapped and taps for {W}; the adventure face
// returns up to two artifact and/or enchantment cards from your graveyard
// to your hand. The adventure's self-exile rider is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ISHGARD_THE_HOLY_SEE,
    oracle_id = "4f4358cb-59df-46d9-be27-69929f5a615c",
    scryfall_id = "068bc755-9d3d-430b-abc5-c775a5415bf9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Ishgard, the Holy See",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Faith & Grief",
            mana_cost = mana!("{3}{W}{W}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
            // NOT SUPPORTED: "Then exile this card. You may play the land
            // later from exile." — nothing in the vocabulary makes a
            // resolving spell be exiled instead of put into its owner's
            // graveyard, and no Modifier grants playing a card from exile
            // (`PlayLandsFromGraveyard` is the graveyard, and there is no
            // exile twin of it).
            abilities = &[spell!(
                &[Effect::GraveyardToHand {
                    target: TargetSpec::CardInGraveyard(
                        &Filter::ARTIFACT_OR_ENCHANTMENT,
                        PlayerRel::You,
                    ),
                }],
                targets = Some(TargetReq::up_to(
                    TargetSpec::CardInGraveyard(&Filter::ARTIFACT_OR_ENCHANTMENT, PlayerRel::You),
                    2,
                )),
            )],
        ),
    ],
    coverage = Coverage::Partial(
        "the adventure's rider — \"Then exile this card. You may play the land \
         later from exile.\" — has no DSL spelling: nothing sends a resolving \
         spell to exile instead of its owner's graveyard, and no Modifier \
         grants playing a card from exile",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
);
