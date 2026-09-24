//! Cathedral of War — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)
//! Oracle: {T}: Add {C}.
//! Set: M13 #221 — Magic 2013 | Scryfall ID: dd222c07-0b28-41cb-9237-ad7991ab078f | Oracle ID: 5ff647e4-730a-498f-8f2c-5bd64d5a9780
// IMPLEMENTED — enters tapped, exalted, and taps for {C}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CATHEDRAL_OF_WAR,
    oracle_id = "5ff647e4-730a-498f-8f2c-5bd64d5a9780",
    scryfall_id = "dd222c07-0b28-41cb-9237-ad7991ab078f",
    faces = &[face!(
        name = "Cathedral of War",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // Exalted (CR 702.83a): "Whenever a creature you control attacks
        // alone, that creature gets +1/+1 until end of turn."
        //
        // "Attacks alone" is the only creature *declared* as an attacker
        // (CR 702.83b, CR 506.5), and the only spelling for it is an
        // intervening `if` on the attack trigger: you control at most one
        // attacking creature. CR 603.4 asks that again on resolution, where
        // exalted does not — but in this engine a creature becomes attacking
        // only by being declared (`declare_attackers` is the one writer of
        // `combat.attackers`; everything else removes), so the count can only
        // fall between the two checks and the second cannot fail where the
        // first held. The day something is put onto the battlefield
        // attacking (CR 508.4), this spelling is wrong and exalted needs a
        // trigger of its own. Grasping Shadows asks the same question the
        // same way.
        //
        // "That creature" is the event object (no target is chosen,
        // CR 115.10a), which `Filter::This` resolves to.
        triggered!(
            Trigger::Attacks(&Filter::YOUR_CREATURE),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, 1),
                Duration::UntilEndOfTurn
            )],
            targets = Some(TargetReq::one(TargetSpec::EventObject)),
            condition = Some(Condition::ControlCountAtMost(
                &Filter::ATTACKING_CREATURE,
                1
            )),
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
