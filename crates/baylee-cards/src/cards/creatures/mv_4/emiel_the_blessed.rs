//! Emiel the Blessed — {2}{W}{W} — Legendary Creature — Unicorn
//! Oracle: {3}: Exile another target creature you control, then return it to the battlefield under its owner's control.
//! Oracle: Whenever another creature you control enters, you may pay {G/W}. If you do, put a +1/+1 counter on it. If it's a Unicorn, put two +1/+1 counters on it instead. ({G/W} can be paid with either {G} or {W}.)
//! Set: 2X2 #10 — Double Masters 2022 | Scryfall ID: 0f594562-7e9f-47e6-a033-fb70e3cf1e10 | Oracle ID: b11c250c-f191-4c52-ba02-a9176f163447
// PARTIAL — a 4/4 Unicorn whose {3} blinks another creature you control:
// exiled and returned to the battlefield under its owner's control at once,
// which is the mode the card is played for.
// NOT SUPPORTED: "Whenever another creature you control enters, you may pay
// {G/W}. If you do, put a +1/+1 counter on it. If it's a Unicorn, put two
// +1/+1 counters on it instead." No `Effect` offers an optional payment of a
// *named* mana cost while an ability resolves, with the rest of the clause
// running on the **yes**: `Effect::PlayerMayPayOr` takes an `Amount` of
// generic mana — it cannot say `{G/W}` — and runs its branch on the refusal,
// and `Effect::MayDo` is a yes-or-no with nothing to pay. Written with
// either of them the counters would arrive free, which is a card *stronger*
// than the printing and the one thing `Coverage::Partial` may not be, so the
// whole ability is left off and Emiel plays exactly as though the line were
// not printed. ("If it's a Unicorn … instead" is expressible on its own —
// two triggers over mutually exclusive filters, the way Derevi writes one
// printed sentence as two abilities — so the payment is the whole blocker.)

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMIEL_THE_BLESSED,
    oracle_id = "b11c250c-f191-4c52-ba02-a9176f163447",
    scryfall_id = "0f594562-7e9f-47e6-a033-fb70e3cf1e10",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Emiel the Blessed",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(4),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial("the entering-creature counter trigger is not written"),
    abilities = &[activated!(
        cost!("{3}"),
        &[Effect::blink(TargetSpec::Object(
            &Filter::ANOTHER_CREATURE_YOU_CONTROL
        ))],
        target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL))
    )],
);

// Engine-level coverage belongs in `card_tests`: this is the pool's first
// `Effect::blink` on an *activated* ability — the five that exist hang off a
// spell, a trigger or a loyalty cost — so the test is `{3}` paid, another
// creature you control chosen through `AbilityDef::Activated.target`, and
// that creature leaving and re-entering the battlefield (a fresh object, its
// enter-triggers firing again).
