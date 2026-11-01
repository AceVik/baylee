#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// The pool prints none of these, so the AI test each one would need cannot
/// be written against a real card yet. When one of these fails, the mechanic
/// has arrived and the named row of `docs/ai-coverage-todo.md` is due.
#[test]
fn a_mechanic_the_pool_cannot_print_yet_has_no_ai_test_to_write() {
    let rows: [(&str, &str, usize); 2] = [
        (
            "Commander pair rules (every pairing but plain Partner)",
            "a game pairing the commander kinds that are not plain Partner, \
             where each one's tax and combat damage stay separate and a \
             partner's damage never counts toward the other's 21",
            // Written as an exclusion rather than a list of the four, so a
            // pairing added to `PartnerKind` later is caught by this test
            // instead of being silently left out of it.
            count(|def| !matches!(def.partner, PartnerKind::None | PartnerKind::Partner)),
        ),
        (
            "Proliferate and replacement effects (the poison half)",
            "an AI decision that treats poison counters as hostile and \
             +1/+1 counters as friendly, rather than assigning one sign to \
             every counter kind",
            count(|def| {
                def.all_keywords()
                    .contains(KeywordSet::INFECT.union(KeywordSet::WITHER))
            }),
        ),
    ];

    for (row, owed, found) in rows {
        assert_eq!(
            found, 0,
            "{found} card(s) in the pool now reach `{row}` in \
             docs/ai-coverage-todo.md. That row is no longer a TODO: write \
             {owed}, then delete this entry. A mechanic that arrives \
             without its AI test is a strategy nobody measured."
        );
    }
}

/// The pool already prints these, so the row is owed **now**. The assertion
/// is "more than none" and not an exact number on purpose: a row does not
/// stop being due because a second card joined it, and pinning the count
/// would train whoever adds a card to bump a number instead of writing a
/// test.
#[test]
#[allow(clippy::too_many_lines)] // one entry per table row, in one readable list
fn a_mechanic_the_pool_already_prints_is_owed_now_and_not_later() {
    let rows: [(&str, usize); 28] = [
        (
            "Commander pair rules (plain Partner)",
            count(|def| matches!(def.partner, PartnerKind::Partner)),
        ),
        (
            // Blocking to keep one paid, #75: `a_walker_that_would_die_is_\
            // chumped_for_and_one_that_would_not_is_not` in `baylee-ai`.
            "Planeswalker survival",
            count(|def| def.faces.iter().any(|face| face.loyalty.is_some())),
        ),
        (
            "Modal and multi-target effects",
            count(|def| {
                abilities(def).any(|a| {
                    matches!(
                        a,
                        AbilityDef::ModalSpell { .. }
                            | AbilityDef::ModalTriggered { .. }
                            | AbilityDef::ModalActivated { .. }
                    )
                })
            }),
        ),
        (
            // Paid, 23.09.2026: `a_fight_names_the_creature_its_fighter_\
            // kills_and_survives` and `a_fight_names_the_fighter_with_a_\
            // fight_worth_having` in `baylee-ai`. The first was the defect:
            // Bridgeworks Battle's pump made the whole spell read as a
            // benefit, so every creature across the table scored as one the
            // spell must not reach and "up to one" was answered with none.
            // Both instances of "target" are asked separately (CR 115.3), and
            // `fight::fight_targets` answers each by what the fight would do.
            "Modal and multi-target effects (a fight's two instances)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::Fight { .. } | Effect::DamageEqualToPower { .. }
                )
            }),
        ),
        // **This counts compiled faces, not printings**, and the difference
        // is filed as #115: the predicate is `faces.len() >= 2`, the printed
        // figure is 121, and what a client actually wants is the ~107 with a
        // separate back *image*. An adventure or a split prints both halves
        // on one physical face and still answers yes here. The number is a
        // correct measurement of the predicate and would be a wrong answer
        // to "how many cards have two faces", which is why the row says
        // which it is.
        (
            "Transformation and alternate zones (a second compiled face)",
            count(|def| def.faces.len() >= 2),
        ),
        (
            "Transformation and alternate zones (adventure and disturb)",
            count(|def| def.faces.iter().any(|face| face.adventure || face.disturb)),
        ),
        (
            "Variable costs (convoke and delve)",
            count(|def| def.faces.iter().any(|face| face.convoke || face.delve)),
        ),
        (
            // Paid, #74: `an_optional_additional_cost_is_paid_when_the_pool_\
            // covers_it`, `the_kicker_is_floated_before_the_cast` and the
            // real-engine `a_kicker_and_a_waterbend_are_paid_when_the_mana_is_\
            // there`. Kicker and "you may waterbend" are both this field.
            "Variable costs (an optional additional cost)",
            count(|def| {
                def.faces
                    .iter()
                    .any(|face| !face.additional_costs.is_empty())
            }),
        ),
        (
            // Paid, #73: `a_lore_counter_goes_on_my_own_saga_and_never_the_\
            // opponents` in `baylee-ai`. The probe stays because the test
            // needs a Saga in the pool to be about anything — if Urza's Saga
            // leaves, that test starts proving nothing and this row says so.
            "Contextual counters (the lore half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::SagaChapter { .. }))),
        ),
        (
            // Suspend is the half of that row the pool can already test:
            // a time counter on a suspended card is a clock the AI wants to
            // run down, and the same counter on vanishing is one it does not.
            //
            // Paid, #73: `a_time_counter_delays_the_suspended_card_that_is_\
            // about_to_cast`. Vanishing is still owed and cannot be written:
            // no card here prints it, and `clock_score` scores that case 0
            // rather than guessing at it.
            "Contextual counters (the suspend half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Suspend { .. }))),
        ),
        (
            // Paid, 23.09.2026: `a_plus_one_counter_does_not_go_on_my_own_\
            // undying_creature` and `a_minus_one_counter_prefers_the_persist_\
            // creature_it_keeps_down` in `baylee-ai`. This row moved here
            // from the "cannot print yet" list the day the pool printed the
            // two keywords, which is what the other test is for.
            //
            // Both are needed and neither alone is the rule: the sign of the
            // counter is still its own, and what `tactics::denies_a_return`
            // adds is that the counter a keyword's intervening `if` reads is
            // a denial on the creature that prints it (CR 702.93a,
            // CR 702.79a). One test would be satisfied by a second fixed
            // sign.
            "Contextual counters (the ±1/±1 payoff)",
            count(|def| {
                def.all_keywords()
                    .contains(KeywordSet::PERSIST.union(KeywordSet::UNDYING))
            }),
        ),
        (
            "Stack strategy (ward and taxes)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Ward { .. }))),
        ),
        // 35 on 19.09.2026, and two of them only because `Effect::walk`
        // reads behind a price: Flusterstorm and Malevolent Hermit counter
        // a spell *unless its controller pays*, which is a variant carrying
        // one effect rather than a list. A probe matching the top-level
        // list would have reported 33 and named no row for the other two.
        (
            "Stack strategy (counter wars)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::CounterTargetSpell
                        | Effect::CounterTargetSpellToExile
                        | Effect::CounterTargetSpellOrAbility
                        | Effect::CounterTargetAbility
                )
            }),
        ),
        (
            "Stack strategy (copying a spell)",
            count_reaching(|effect| matches!(effect, Effect::CopyTargetSpell { .. })),
        ),
        (
            "Stack strategy (redirecting a target)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::ChangeTarget { .. } | Effect::ChooseNewTargets
                )
            }),
        ),
        // The row's sentence is "scout refresh after shuffle, reveal, wish
        // and sideboarding". A library search is what forces the shuffle, so
        // it is the precondition rather than a neighbouring mechanic — and a
        // wish is the row's own word.
        (
            "Hidden-zone decisions (a library search)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::SearchLibrary { .. }
                        | Effect::SearchLibraryUpTo { .. }
                        | Effect::OptionalBasicLandSearchFor { .. }
                )
            }),
        ),
        (
            "Hidden-zone decisions (a wish from outside the game)",
            count_reaching(|effect| matches!(effect, Effect::WishToHand { .. })),
        ),
        // "Copy/control/zone changes preserve per-commander damage
        // identity", which needs a card that can change one.
        (
            "Commander identity changes (a controller change)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::ChangeController { .. }
                        | Effect::ExchangeControlOrSacrifice
                        | Effect::ControlRotation
                        | Effect::AllCreaturesToOwner
                )
            }),
        ),
        (
            "Commander identity changes (a token copy of a permanent)",
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::CreateTokenCopyOf { .. }
                        | Effect::CreateTokenCopyOfEquipped { .. }
                        | Effect::CreateTokenCopyOfFirstToken
                )
            }),
        ),
        // Treasure is asked by identity and not by name: the token table is
        // compiled, so a pointer comparison against `tokens::TREASURE`
        // cannot drift the way a string would.
        //
        // Spent, #223: `a_treasure_pays_for_a_spell_when_nothing_else_can`.
        // The agent read a registry token as printing nothing, so a Treasure
        // was never a mana source.
        (
            "Alternate resource engines (treasure)",
            count_reaching(|effect| {
                // All seven variants that carry a `TokenDef`, and that is
                // the point rather than a flourish: reading the two obvious
                // ones returned 5 and the pool makes 6, the sixth being
                // Fountainport. A probe over a taxonomy is only as honest as
                // the arm it forgot.
                let (Effect::CreateToken { token }
                | Effect::CreateTokenN { token, .. }
                | Effect::CreateTokenPtPerCount { token, .. }
                | Effect::CreateTokenForTargetController { token, .. }
                | Effect::ExileTargetsCreateTokens { token, .. }
                | Effect::Amass { token, .. }
                | Effect::CreateTokenFromLinked { token, .. }) = effect
                else {
                    return false;
                };
                std::ptr::eq(*token, &raw const baylee_cards::tokens::TREASURE)
            }),
        ),
        // Spent, #223: `restricted_mana_pays_for_the_spells_it_names_and_no_\
        // others` through the engine, when the view can read the filter.
        (
            "Alternate resource engines (restricted mana)",
            // A rider alone restricts nothing (#232).
            count_reaching(|effect| {
                matches!(
                    effect,
                    Effect::AddMana {
                        restriction: Some(restriction),
                        ..
                    } if restriction.restricts
                )
            }),
        ),
        // Taken when free, #223: `a_free_alternative_cost_is_taken_over_the_\
        // printed_one`; a pitch or an evoke keeps the printed cost.
        (
            "Alternate resource engines (an alternative cost)",
            count(|def| {
                def.faces
                    .iter()
                    .any(|face| !face.alternative_costs.is_empty())
            }),
        ),
        // The row's own words are "sacrifice/discard/exile/counter costs
        // with beneficial payoffs", which is an **activation** cost and not
        // a spell's additional one. Asking `additional_costs` returned 3,
        // and exactly one of those three prints a non-mana part at all
        // (Toxic Deluge's `PayLifeX`), so the label named three things the
        // predicate could not see.
        //
        // Asked of the abilities instead there are three numbers and this
        // row asserts on the first: **361 cards**, 382 abilities, 383 parts
        // — 195 `SacrificeSelf`, 85 `Sacrifice`, 71 `DiscardSelf`, 32
        // `Discard`. Fourteen cards pay two different kinds (nine of them
        // the Landscape cycle) and one ability pays two parts, which is the
        // whole of the gap between the three. No exile cost appears at all,
        // which is why the name below does not claim one.
        //
        // The first breakdown written here was taken inside an `any`, so it
        // stopped at each card's first matching ability and undercounted
        // every kind. It summed to 362 against a stated 361 — and a
        // breakdown that does not sum to its own total is how it was
        // caught, for the second time in one day.
        //
        // Weighed since 29.09.2026 (`worth`): a sacrifice, discard or exile
        // is priced at what it gives up and paid when the effect is worth
        // more (#223, `a_card_is_not_given_up_for_a_small_gain`, and the
        // Wasteland and Recurring Nightmare pairs in `baylee-ai`).
        (
            "Alternate resource engines (a sacrifice or discard cost)",
            count(|def| {
                abilities(def).any(|ability| {
                    let (AbilityDef::Activated { cost, .. }
                    | AbilityDef::ActivatedConditional { cost, .. }
                    | AbilityDef::ModalActivated { cost, .. }) = ability
                    else {
                        return false;
                    };
                    cost.parts.iter().any(|part| {
                        matches!(
                            part,
                            CostPart::Sacrifice(_)
                                | CostPart::SacrificeSelf
                                | CostPart::Discard(_)
                                | CostPart::DiscardSelf
                        )
                    })
                })
            }),
        ),
        // `has_variable` and not a spelling: the cost is parsed at compile
        // time, so the question "does this card announce an X" is answered
        // by the type rather than by looking for the letter.
        (
            "Variable costs (X in a printed cost)",
            count(|def| def.faces.iter().any(|face| face.mana_cost.has_variable())),
        ),
        (
            "Variable costs (a cost reduction)",
            count(|def| def.faces.iter().any(|face| face.cost_reduction.is_some())),
        ),
        (
            "Proliferate and replacement effects (the replacement half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Replacement(_)))),
        ),
        // Named for what it measures. **No card in this pool has flashback**;
        // these three give it to somebody else's card (Snapcaster Mage,
        // Emry, Stingcaster Mage). Written as "flashback" the cell promised
        // a card with the keyword and delivered three that hand it out,
        // which is a different test to write.
        (
            "Transformation and alternate zones (flashback granted to another card)",
            count_reaching(|effect| matches!(effect, Effect::GrantFlashback)),
        ),
        // `CopyOnEnter` is a permanent entering *as* a copy — the thirteen
        // clones (24.09.2026), Phyrexian Metamorph through Vesuva — and not
        // an ability copied off another card.
        //
        // Answered for what is copied, #227:
        // `a_clone_copies_what_is_worth_having_twice`.
        (
            "Transformation and alternate zones (enters as a copy)",
            count(|def| {
                abilities(def).any(|a| {
                    matches!(
                        a,
                        AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. }
                    )
                })
            }),
        ),
    ];

    for (row, found) in rows {
        assert!(
            found > 0,
            "no card in the pool reaches `{row}` in docs/ai-coverage-todo.md \
             any more. Either the mechanic left the pool — then move this \
             entry back to the tripwire test above, so its return is \
             noticed — or this probe stopped matching, which is the failure \
             the compiled pool is read for."
        );
    }
}
