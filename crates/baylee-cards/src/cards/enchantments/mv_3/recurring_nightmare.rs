//! Recurring Nightmare — {2}{B} — Enchantment
//! Oracle: Sacrifice a creature, Return this enchantment to its owner's hand: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: TPR #113 — Tempest Remastered | Scryfall ID: b50e1800-a45c-43bd-8886-8a06145d9346 | Oracle ID: a6708b11-1bcd-4208-a967-fe91f2e3313c
// IMPLEMENTED — sorcery-speed reanimation, bounce-to-hand cost.
// NOT SUPPORTED: `Sacrifice a creature` as part of the cost. A cost that
// names something to *choose* has nothing to ask the question with —
// `pay_cost` refuses `CostPart::Sacrifice` outright — so the ability is
// unpayable and `can_afford` therefore declines to offer it. The whole card
// is inert as a result, which is what `Partial` is here to say: it was
// `Implemented` while the engine offered its only ability and then took it
// back, and the deckbuilder was listing it as playable. It goes back to
// `Implemented` the day an activation can suspend on a choice during cost
// payment, the way the cast wizard already does for `ExileFromHand`.

static CREATURE_YOU_CONTROL: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

use baylee_cards_dsl::prelude::*;

card!(
    index = 2969,
    oracle_id = "a6708b11-1bcd-4208-a967-fe91f2e3313c",
    scryfall_id = "b50e1800-a45c-43bd-8886-8a06145d9346",
    faces = &[face!(
        name = "Recurring Nightmare",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    )],
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial("a sacrifice cost cannot be chosen during an activation"),
    abilities = &[activated!(
        cost!(Sacrifice(&CREATURE_YOU_CONTROL), ReturnSelfToHand),
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        )),
        timing = ActivationTiming::SorcerySpeed
    )],
);
