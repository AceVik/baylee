//! Reading amounts and costs, statics on the battlefield, and saying why a
//! script is refused.

use super::*;

/// Buyback (CR 702.27a) is a field of the face, mana or parts, and a card
/// that prints it is read in full.
#[test]
fn buyback_is_read_onto_the_face_in_mana_or_in_parts() {
    let cost = |line: &str| {
        read(&format!(
            "Name:X\nManaCost:U\nTypes:Instant\nK:Buyback:{line}\nA:SP$ Draw | NumCards$ 1"
        ))
        .buyback
    };
    assert_eq!(cost("3").as_deref(), Some("Some(cost!(\"{3}\"))"));
    assert_eq!(cost("2 U").as_deref(), Some("Some(cost!(\"{2}{U}\"))"));
    assert_eq!(
        cost("PayLife<4>").as_deref(),
        Some("Some(cost!(PayLife(4)))")
    );
    let land = cost("Sac<1/Land/land>").expect("a land is a cost the cast pays");
    assert!(land.contains("Sacrifice("), "{land}");
}

/// A part the cast does not pay would be a free buyback, so it is refused.
#[test]
fn buyback_refuses_a_part_the_cast_does_not_pay() {
    assert!(refused(
        "Name:X\nManaCost:U\nTypes:Instant\nK:Buyback:T\nA:SP$ Draw | NumCards$ 1"
    ));
}

/// `MayFlashCost` is the optional additional mana cost that grants flash;
/// the ordinary casting route and its printed cost stay available.
#[test]
fn may_flash_cost_reads_mana_and_the_flash_rider() {
    let body =
        read("Name:X\nManaCost:R\nTypes:Sorcery\nK:MayFlashCost:2\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(body.optional_cost.as_deref(), Some("Some(cost!(\"{2}\"))"));
    assert!(body.optional_cost_grants_flash);
}

#[test]
fn may_flash_cost_refuses_invalid_or_conflicting_costs() {
    for script in [
        "Name:X\nTypes:Land\nK:MayFlashCost:2",
        "Name:X\nK:MayFlashCost:2",
        "Name:X\nTypes:Instant\nK:MayFlashCost:T",
        "Name:X\nTypes:Instant\nK:MayFlashCost:X",
        "Name:X\nTypes:Instant\nK:MayFlashCost:2\nK:MayFlashCost:3",
        "Name:X\nTypes:Instant\nK:MayFlashCost:2\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ 1",
    ] {
        assert!(refused(script), "must refuse: {script}");
    }
}

/// Multikicker (CR 702.33c) is a mana cost on the face, and "for each time it
/// was kicked" is the count the spell announced.
#[test]
fn multikicker_and_its_count_are_read() {
    let body = read(
        "Name:X\nManaCost:1 G\nTypes:Creature\nPT:1/1\nK:Multikicker:1 G\n\
         K:etbCounter:P1P1:X\nSVar:X:Count$TimesKicked",
    );
    assert_eq!(body.multikicker.as_deref(), Some("Some(mana!(\"{1}{G}\"))"));
    assert!(
        body.enter_modifiers[0].contains("Amount::TimesKicked"),
        "{:?}",
        body.enter_modifiers
    );
    assert!(refused(
        "Name:X\nManaCost:1 G\nTypes:Creature\nPT:1/1\nK:Multikicker:Sac<1/Land/land>"
    ));
}

/// `Amount$ X` over `SVar:X:Count$Valid …` is a counted mana amount.
///
/// Cabal Coffers' sentence, and the DSL has been able to say it since
/// `Amount::CountOf` — Gaea's Cradle is written with it by hand. What was
/// missing was a reader, which is the shape this report keeps producing:
/// the top blocker names a rule that exists and a value it cannot say.
///
/// The second half is the one that would be written rather than refused.
/// A definition that is not a battlefield count is named **by the
/// definition** and never by the letter, because `Amount$ X` is one
/// spelling standing for thirty different questions.
#[test]
fn a_counted_mana_amount_reads_the_definition_and_not_the_letter() {
    // Cabal Coffers' line with the one subtype this module's fixture
    // knows: `cats()` carries three land types and Swamp is not among
    // them, which is a fact about the fixture and not about the rule.
    let body = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ 2 T | Produced$ G | Amount$ X | \
         SpellDescription$ Add {G} for each Forest you control.\n\
         SVar:X:Count$Valid Forest.YouCtrl\n",
    );
    assert!(
        body.statics.contains(
            "static COUNT1: Filter = Filter::And(&[Filter::HasSubtype(subtypes::land::FOREST), \
             Filter::ControlledByYou]);"
        ),
        "{}",
        body.statics
    );
    assert_eq!(
        body.abilities,
        [concat!(
            "mana_ability!(cost!(\"{2}\", TapSelf), ",
            "&[Effect::mana_dynamic(ManaColor::Green, Amount::CountOf { ",
            "filter: &COUNT1, zone: ZoneSel::Battlefield })])"
        )]
    );

    let your_gy = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ B | Amount$ X | SpellDescription$ Add.\n\
         SVar:X:Count$ValidGraveyard Creature.Black+YouCtrl\n",
    );
    assert!(
        your_gy
            .abilities
            .join("")
            .contains("zone: ZoneSel::GraveyardOf(PlayerRel::You)")
    );

    let elsewhere = parse(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ B | Amount$ X | SpellDescription$ Add.\n\
         SVar:X:Count$ValidExile Card.Black\n",
    );
    assert!(transcode(&elsewhere, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&elsewhere, &cats(), None).as_deref(),
        Some("count `Count$ValidExile Card.Black`"),
        "the definition is the worklist entry; the letter is not"
    );
}

/// `Produced$ Combo U R | Amount$ 2` is a pick **per mana**.
///
/// "Add {U}{U}, {U}{R}, or {R}{R}" is the filter cycle's sentence and it
/// is not "two mana of any one color" — the two answers may differ, which
/// is `combination: true` and the reason the land is worth playing.
/// `Produced$ Any | Amount$ 3` is the neighbouring sentence with one pick
/// for the whole amount, and the two constructors are what keep them
/// apart.
///
/// `Combo Any` is the five colours written as one word, so Baxter
/// Building's "four mana in any combination of colors" is the same rule
/// as the filter lands' two, and `Combo ColorIdentity` is a *source*
/// rather than a list — no card can name a commander's colours (CR
/// 903.4). A combination word that is neither refuses by its own name.
#[test]
fn a_combination_picks_once_per_mana_and_any_one_color_picks_once() {
    let filter_land = read(
        "Name:Cascade Bluffs\nTypes:Land\n\
         A:AB$ Mana | Cost$ UR T | Produced$ Combo U R | Amount$ 2 | \
         SpellDescription$ Add {U}{U}, {U}{R}, or {R}{R}.\n",
    );
    assert_eq!(
        filter_land.abilities,
        [concat!(
            "mana_ability!(cost!(\"{U/R}\", TapSelf), ",
            "&[Effect::mana_combination(&[ManaColor::Blue, ManaColor::Red], ",
            "Amount::Fixed(2))])"
        )]
    );

    let any_one = read(
        "Name:Lotus Vale\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | Amount$ 3 | \
         SpellDescription$ Add three mana of any one color.\n",
    );
    assert_eq!(
        any_one.abilities,
        ["mana_ability!(&[Effect::mana_choice_dynamic(ALL_MANA_COLORS, Amount::Fixed(3))])"],
        "one pick for three mana, which `mana_combination` would have \
         asked three times"
    );

    let every_colour = read(
        "Name:Baxter Building\nTypes:Land\n\
         A:AB$ Mana | Cost$ 4 T | Produced$ Combo Any | Amount$ 4 | \
         SpellDescription$ Add four mana in any combination of colors.\n",
    );
    assert_eq!(
        every_colour.abilities,
        [concat!(
            "mana_ability!(cost!(\"{4}\", TapSelf), ",
            "&[Effect::mana_combination(ALL_MANA_COLORS, Amount::Fixed(4))])"
        )]
    );

    let commander = read(
        "Name:Hidden Hideout\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo ColorIdentity | \
         SpellDescription$ Add one mana of any color in your commander's color identity.\n",
    );
    assert_eq!(
        commander.abilities,
        ["mana_ability!(&[Effect::mana_commander_identity()])"]
    );

    // "Add two mana of different colors" lowers to `mana_different_colors`
    // for fixed 2..=5, while out-of-range counts and `Each$` stay refused.
    let different = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ 1 T | Produced$ Combo AnyDifferent | Amount$ 2 | \
         SpellDescription$ Add two mana of different colors.\n",
    );
    assert_eq!(
        different.abilities,
        [concat!(
            "mana_ability!(cost!(\"{1}\", TapSelf), ",
            "&[Effect::mana_different_colors(ALL_MANA_COLORS, Amount::Fixed(2))])"
        )]
    );

    let bad_different = parse(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ 1 T | Produced$ Combo AnyDifferent | Amount$ 1\n",
    );
    assert!(transcode(&bad_different, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&bad_different, &cats(), None).as_deref(),
        Some("`Mana` combination over `AnyDifferent`")
    );
}

/// `Cost$ UR T` is the filter cycle's price, and the letters run together.
///
/// - colour pairs keep their **printed order** (`{U/R}` and not `{R/U}`);
/// - two-or-color (`2W` -> `{2/W}`), Phyrexian (`WP` -> `{W/P}`), and hybrid
///   Phyrexian (`URP` -> `{U/R/P}`) lower to the exact symbols `ManaCost`
///   parses;
/// - doubled pairs (`WW`, `WWP`), non-color symbols (`2C`, `CP`), and
///   three-color tokens (`WUB`) keep refusing **by name** so `ColorPair::new`
///   can never panic at compile time.
#[test]
fn a_hybrid_activation_cost_keeps_the_order_the_card_prints() {
    let body = read(
        "Name:Cascade Bluffs\nTypes:Land\n\
         A:AB$ Mana | Cost$ UR T | Produced$ U | SpellDescription$ Add {U}.\n",
    );
    assert_eq!(
        body.abilities,
        ["mana_ability!(cost!(\"{U/R}\", TapSelf), &[Effect::mana(ManaColor::Blue, 1)])"]
    );

    let special = read(
        "Name:SpecialMana\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ 2W WP URP | NumCards$ 1\n",
    );
    assert_eq!(
        special.abilities,
        ["activated!(cost!(\"{2/W}{W/P}{U/R/P}\"), &[Effect::draw(1)])"]
    );

    for token in ["WW", "WUB", "2C", "CP", "WWP"] {
        let script = parse(&format!(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ {token} T | Produced$ U | SpellDescription$ Add {{U}}.\n"
        ));
        assert!(
            transcode(&script, &cats(), None).is_none(),
            "`{token}` is not a valid mana symbol"
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(format!("cost `{token}`").as_str()),
            "and it refuses by its own name rather than by the rule's"
        );
    }
}

/// `Cost$ ExileFromGrave<1/Creature>` is a card the player names out of
/// their own graveyard, so it is the filter as written with no "you
/// control" added — the zone is the whole of the "your" — and the source
/// naming itself
/// is refused, because eternalize's "exile this card from your
/// graveyard" is paid from a zone no ability is activated from yet.
/// Three cards is three objects, which this reader refuses by name as it
/// does for every other kind.
#[test]
fn a_graveyard_exile_cost_is_a_filter_over_the_payers_own_pile() {
    let body = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Draw | Cost$ W U T ExileFromGrave<1/Creature> | NumCards$ 1\n",
    );
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{W}{U}\", TapSelf, ExileFromGraveyard(&Filter::CREATURE)), \
             &[Effect::draw(1)])"
        ],
        "the spelling Moorland Haunt is written in by hand"
    );
    assert!(
        body.statics.is_empty(),
        "a named filter needs no static, and nothing was added to it: {}",
        body.statics
    );

    for (cost, why) in [
        (
            "ExileFromGrave<1/CARDNAME/this card>",
            "cost `ExileFromGrave<1/CARDNAME/this card>` naming itself",
        ),
        ("ExileFromGrave<4/Card>", "a cost naming `4` objects"),
    ] {
        let script = parse(&format!(
            "Name:X\nTypes:Land\n\
             A:AB$ Draw | Cost$ 1 {cost} | NumCards$ 1\n"
        ));
        assert!(
            transcode(&script, &cats(), None).is_none(),
            "{cost} is refused, not paid with one card"
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(why),
            "and it says so by name"
        );
    }
}

/// `Cost$ Return<1/Forest>` is a permanent the *player* names, so it
/// comes out as a filter and a `static` above the card, the way a
/// target's does — and `Return<1/CARDNAME>` is the source and carries no
/// filter at all. Quirion Ranger and Recurring Nightmare print the two
/// halves, which is why one rule reads both.
#[test]
fn a_return_cost_is_a_filter_unless_it_names_the_card_itself() {
    let body = read(
        "Name:Quirion Ranger\nManaCost:G\nTypes:Creature Elf Ranger\nPT:1/1\n\
         A:AB$ Untap | Cost$ Return<1/Forest> | ValidTgts$ Creature | ActivationLimit$ 1\n",
    );
    assert!(
        body.statics.contains(
            "static COST1: Filter = Filter::And(&[Filter::HasSubtype(subtypes::land::FOREST), \
             Filter::ControlledByYou]);"
        ),
        "{}",
        body.statics
    );
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(ReturnToHand(&COST1)), &[Effect::UntapTarget], \
             target = Some(TargetSpec::Object(&Filter::CREATURE)), \
             limit = ActivationLimit::PerTurn(1))",
        ]
    );

    let itself = read(
        "Name:X\nManaCost:2 B\nTypes:Enchantment\n\
         A:AB$ Untap | Cost$ Return<1/CARDNAME> | ValidTgts$ Creature\n",
    );
    assert_eq!(
        itself.abilities,
        [
            "activated!(cost!(ReturnSelfToHand), &[Effect::UntapTarget], \
          target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ],
    );
    assert!(
        itself.statics.is_empty(),
        "the source needs no filter: {}",
        itself.statics
    );
}

/// A bracketed cost carries the reference's own prose as its last field,
/// and that prose has spaces in it. Splitting the cost on whitespace cut
/// the part in two and refused the card under the leftover, reported as
/// the cost "artifact>" — which is a part nobody wrote and a card nobody
/// could have fixed. 731 of the reference's costs are written this way.
///
/// The counter-test is the point of the second half, and it changed its
/// shape when the reader learned these costs: a sacrifice of something
/// other than the source used to be refused, and the refusal was the
/// proof that the tokeniser had handed the whole bracket over in one
/// piece. It is read now, so the proof is what comes *out* — one `{1}`
/// and one `Sacrifice`, with the reference's own prose ("another
/// creature", spaces and all) nowhere in the filter. A tokeniser that
/// split on whitespace would put it there.
#[test]
fn a_cost_is_not_split_inside_its_own_brackets() {
    let body = read(
        "Name:X\nManaCost:no cost\nTypes:Artifact\n\
         A:AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME/this artifact> | LifeAmount$ 3\n",
    );
    assert_eq!(
        body.abilities,
        ["activated!(cost!(\"{2}\", TapSelf, SacrificeSelf), &[Effect::gain_life(3)])"]
    );

    let quirion = read(
        "Name:Quirion Ranger\nManaCost:G\nTypes:Creature Elf Ranger\nPT:1/1\n\
         A:AB$ Untap | Cost$ Return<1/Forest/a Forest> | ValidTgts$ Creature\n",
    );
    assert!(
        quirion
            .statics
            .contains("Filter::HasSubtype(subtypes::land::FOREST)"),
        "{}",
        quirion.statics
    );

    let script = CardScript::default();
    let cats = cats();
    let mut tx = Tx::new(&script, &cats, None);
    let cost = tx
        .cost_expr("1 Sac<1/Creature.Other/another creature>")
        .expect("a bracketed sacrifice is one token");
    assert_eq!(cost, "cost!(\"{1}\", Sacrifice(&COST1))");
    assert!(
        tx.body.statics.contains("Filter::Another")
            && tx.body.statics.contains("Filter::ControlledByYou")
            && !tx.body.statics.contains("another creature"),
        "the prose is the reference's own label, not part of the filter: {}",
        tx.body.statics
    );
    assert_eq!(
        tx.unclaimed.into_inner(),
        None,
        "nothing was refused, so nothing has a reason to give"
    );
}

/// The other half of the same rule: with a valid-string the untap is the
/// chosen permanent's, and the ability carries the target it was read
/// from. Asserted beside the self case so neither can quietly become the
/// other.
#[test]
fn an_untap_with_a_valid_string_untaps_the_chosen_permanent() {
    let body = read(
        "Name:X\nManaCost:1 U\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ Untap | Cost$ T | ValidTgts$ Creature | TgtPrompt$ Select target creature\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::UntapTarget"), "{text}");
    assert!(!text.contains("Effect::UntapSelf"), "{text}");
    assert!(text.contains("target = Some("), "{text}");
}

/// Wall of Roots is two rules in one line: a counter the DSL had no name
/// for, and a sentence that caps the activation.
///
/// `M0M1` is read by shape rather than by name — CR 122.1a is one rule
/// over an open-ended set of pairs, and the corpus prints eleven of them
/// — while `P1P1` and `M1M1` keep the constants that spell them, which
/// are the *same value* and not a second meaning. That is the half worth
/// asserting both ways: a rule that emitted the general form for +1/+1
/// would rewrite hundreds of cards to say the same thing longer.
#[test]
fn a_wall_that_wears_its_own_counters_reads_as_one_mana_ability() {
    let body = read(
        "Name:Wall of Roots\nManaCost:1 G\nTypes:Creature Plant Wall\nPT:0/5\n\
         K:Defender\n\
         A:AB$ Mana | Cost$ AddCounter<1/M0M1> | Produced$ G | ActivationLimit$ 1\n",
    );
    assert_eq!(
        body.abilities,
        [
            "mana_ability!(cost!(PutCounterSelf { kind: CounterKind::Minus { power: 0, \
             toughness: 1 }, n: 1 }), &[Effect::mana(ManaColor::Green, 1)], \
             limit = ActivationLimit::PerTurn(1))"
        ]
    );

    // The two Magic prints everywhere keep their names.
    let body = read(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ PutCounter | Cost$ T | CounterType$ P1P1 | CounterNum$ 1\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("CounterKind::P1P1"), "{text}");

    // And a limit of more than one is a count, not a flag.
    let body = read(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | ActivationLimit$ 2\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("limit = ActivationLimit::PerTurn(2)"),
        "{text}"
    );
}

/// A loyalty cost lowers strictly to `loyalty!(...)` on `Planeswalker$ True`
/// abilities (CR 606.3–4), and NEVER leaks into an ordinary `activated!`
/// counter cost when `Planeswalker$ True` is absent.
#[test]
fn a_loyalty_cost_does_not_become_an_ordinary_counter_cost() {
    let pw = read(
        "Name:X\nManaCost:2 W W\nTypes:Legendary Planeswalker Ajani\nLoyalty:4\n\
         A:AB$ PutCounter | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | \
         CounterType$ P1P1 | CounterNum$ 1 | ValidTgts$ Creature",
    );
    assert_eq!(
        pw.abilities,
        [
            "loyalty!(1, &[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    // Without `Planeswalker$ True`, neither AddCounter nor SubCounter of LOYALTY
    // may ever become an ordinary `activated!` counter cost.
    for cost in ["AddCounter<1/LOYALTY>", "SubCounter<2/LOYALTY>"] {
        let script = format!(
            "Name:X\nManaCost:2 W W\nTypes:Artifact\n\
             A:AB$ Draw | Cost$ {cost} | NumCards$ 1"
        );
        assert!(refused(&script), "must refuse non-PW loyalty cost: {cost}");
        assert_eq!(
            refusal_reason(&parse(&script), &cats(), None).as_deref(),
            Some("a loyalty cost"),
            "{cost}"
        );
    }
}

/// A pump that grants keywords carries them in the same effect — and
/// a "keyword" that is really a sentence refuses the card.
#[test]
fn a_pump_carries_only_keywords_the_engine_has_a_bit_for() {
    let body = read(
        "Name:X\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | \
         KW$ Trample & Haste\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("KeywordSet::TRAMPLE.union(KeywordSet::HASTE)"),
        "{text}"
    );

    let cant_block = read(
        "Name:X\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +0 | NumDef$ +0 | \
         KW$ HIDDEN CARDNAME can't block.",
    );
    assert!(
        cant_block
            .abilities
            .join("")
            .contains("KeywordSet::CANT_BLOCK")
    );
    assert!(refused(
        "Name:X\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +0 | NumDef$ +0 | \
         KW$ HIDDEN CARDNAME doesn't untap."
    ));
    // Any duration but the default is a different lifetime.
    assert!(refused(
        "Name:X\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ +1 | Duration$ Permanent"
    ));
    // A pump with no target pumps nothing.
    assert!(refused(
        "Name:X\nManaCost:G\nTypes:Instant\nA:SP$ Pump | NumAtt$ +1 | NumDef$ +1"
    ));
}

/// Where a target lives is the effect's business, not the valid
/// string's: `TargetSpec::Object` enumerates the battlefield only.
#[test]
fn a_counterspell_targets_the_stack_and_not_the_battlefield() {
    let body = read(
        "Name:Negate\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.nonCreature\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("TargetSpec::Spell"), "{text}");
    assert!(!text.contains("TargetSpec::Object"), "{text}");
}

/// The corpus's `Any` means creature, planeswalker, battle *or player*, and
/// no `TargetSpec` spans objects and players. Read as `Filter::Any` it
/// silently produced a burn spell that could not point at a player.
/// The corpus's `Any` means creature, planeswalker, battle *or player*, so
/// it is a `TargetSpec`, not a `Filter` — nothing on a player can be
/// filtered on. Read as `Filter::Any` it silently produced a burn
/// spell that could not point at a face.
#[test]
fn any_target_spans_objects_and_players() {
    let body = read(
        "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("TargetSpec::AnyTarget"), "{text}");
    assert!(!text.contains("Filter::Any"), "{text}");
}

/// of these would otherwise generate a card missing half its rules.
#[test]
fn an_anthem_is_a_static_ability_on_the_layer_it_belongs_to() {
    let body = read(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.Goblin+Other+YouCtrl | AddPower$ 1 | \
         Description$ Other Goblins you control get +1/+0.\n",
    );
    let a = body.abilities.join("\n");
    // `ModifyPT` *is* the "7c, not 7b" claim now: the macro takes no
    // layer and `Modifier::layer` derives one from the other.
    assert!(a.starts_with("static_ability!("), "{a}");
    assert!(a.contains("Modifier::ModifyPT(1, 0)"), "+1/+0, 7c: {a}");
    assert!(
        a.contains("Filter::Another"),
        "\"other\" is part of the filter: {a}"
    );
    assert!(
        a.contains("Filter::ControlledByYou"),
        "and so is \"you control\": {a}"
    );
}

#[test]
fn a_counted_power_bonus_keeps_controller_scope() {
    let body = read(
        "Name:Nim Lasher\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ X | AddToughness$ 0\n\
         SVar:X:Count$Valid Artifact.YouCtrl\n",
    );
    let a = format!("{}{}", body.statics, body.abilities.join("\n"));
    assert!(a.contains("Modifier::ModifyPTPerCount"), "{a}");
    assert!(a.contains("Filter::ARTIFACT"), "{a}");
    assert!(a.contains("Filter::ControlledByYou"), "{a}");

    assert!(refused(
        "Name:Mixed Scope\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ X | AddToughness$ 0\n\
         SVar:X:Count$Valid Artifact.YouCtrl,Creature\n"
    ));
}

/// A colour word is a colour (CR 105.2), not an unknown subtype: Bad
/// Moon, Crusade, Terror and Northern Paladin were all refused over it.
#[test]
fn a_colour_word_in_a_valid_string_is_the_colour() {
    let body = read(
        "Name:X\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.Black | AddPower$ 1 | AddToughness$ 1 | \
         Description$ Black creatures get +1/+1.\n",
    );
    assert_eq!(
        body.abilities,
        ["static_ability!(Filter::And(&[Filter::CREATURE, \
          Filter::HasColor(ColorSet::from_slice(&[Color::Black]))]), Modifier::ModifyPT(1, 1))"]
    );
    let terror = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.nonArtifact+nonBlack | NoRegen$ True\n",
    );
    let text = format!("{}{}", terror.statics, terror.abilities.join("\n"));
    assert!(
        text.contains("Filter::LacksType(TypeSet::ARTIFACT)"),
        "{text}"
    );
    assert!(
        text.contains("Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black])))"),
        "{text}"
    );
    // A fixed number compared with a power is read; one compared with
    // another object's power is not.
    assert_eq!(
        stat_atom("powerLE2").as_deref(),
        Some("Filter::PowerAtMost(2)")
    );
    assert_eq!(
        stat_atom("PowerGE3").as_deref(),
        Some("Filter::PowerAtLeast(3)")
    );
    assert_eq!(
        stat_atom("toughnessLT3").as_deref(),
        Some("Filter::ToughnessAtMost(2)")
    );
    assert_eq!(stat_atom("toughnessLTX"), None);
    assert_eq!(color_atom("nonWhite"), Some((true, "White")));
    assert_eq!(color_atom("Goblin"), None);
}

/// "Destroy all lands" and "destroy all creatures. They can't be
/// regenerated" (Armageddon, Wrath of God): a sweep, not a target.
#[test]
fn destroy_all_sweeps_what_its_valid_string_names() {
    let body = read("Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Land\n");
    assert_eq!(
        body.abilities,
        ["spell!(&[Effect::destroy_all(&Filter::LAND)])"]
    );
    let wrath =
        read("Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Creature | NoRegen$ True\n");
    assert_eq!(
        wrath.abilities,
        ["spell!(&[Effect::destroy_all_no_regen(&Filter::CREATURE)])"]
    );
    // "Destroy all Forests" is the land type, and the Disk's three
    // types are an `or`.
    let disk = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ DestroyAll | Cost$ 1 T | ValidCards$ Artifact,Creature,Enchantment\n",
    );
    assert!(
        disk.abilities[0].contains("Effect::destroy_all("),
        "{:?}",
        disk.abilities
    );
    assert!(
        read("Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Forest\n").abilities[0]
            .contains("Filter::HasSubtype(")
    );
    assert!(refused(
        "Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Land | NoRegen$ Maybe\n"
    ));
}

/// Protection (CR 702.16a) is a static ability with a quality, printed
/// or granted: the Knights print it, the Wards grant it with the one
/// exception that keeps the Ward itself attached.
#[test]
fn protection_from_a_quality_is_a_static_ability() {
    let knight = read("Name:X\nTypes:Creature\nK:First Strike\nK:Protection from black\n");
    assert_eq!(knight.keywords, ["KeywordSet::FIRST_STRIKE"]);
    assert_eq!(
        knight.abilities,
        ["static_ability!(Filter::This, Modifier::ProtectionFrom(\
          &Filter::HasColor(ColorSet::from_slice(&[Color::Black]))))"]
    );
    let ward = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | \
         AddKeyword$ Protection:Card.White:white:Card.CardUID_HostCardUID | \
         Description$ Enchanted creature has protection from white.\n",
    );
    let a = ward.abilities.join("\n");
    assert!(
        a.contains(
            "Modifier::ProtectionFrom(&Filter::And(&[Filter::HasColor(\
             ColorSet::from_slice(&[Color::White])), Filter::Not(&Filter::This)]))"
        ),
        "{a}"
    );
    assert!(a.contains("Filter::AttachedToBySource"), "{a}");
    assert!(refused(
        "Name:X\nTypes:Creature\nK:Protection:Card.White:white:Card.Other\n"
    ));
}

#[test]
fn one_line_that_moves_two_layers_becomes_two_abilities() {
    // CR 613.1 applies layer 6 before layer 7c, so "get +1/+1 and have
    // flying" is two effects, not one.
    let body = read(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1 | \
         AddKeyword$ Flying\n",
    );
    assert_eq!(body.abilities.len(), 2, "{:?}", body.abilities);
    let a = body.abilities.join("\n");
    // Each layer is named by its modifier rather than beside it — the
    // macro takes none, and `Modifier::layer` derives it. What order the
    // two are written in says nothing: the engine sorts a continuous
    // effect by its layer, not by where it sat in an ability list.
    assert!(a.contains("Modifier::ModifyPT(1, 1)"), "7c: {a}");
    assert!(
        a.contains("Modifier::AddKeyword(KeywordSet::FLYING)"),
        "6: {a}"
    );
}

#[test]
fn a_static_ability_refuses_what_it_cannot_say() {
    // A keyword that carries data is an ability, not a bit — granting
    // it as a bit would grant a keyword no rule reads.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Equip:2\n"
    ));
    // Setting only one half of P/T is `Modifier::SetPower` / `Modifier::SetToughness`;
    // a dynamic `SetPower$ X` with an unknown SVar still refuses.
    let half_pt = read(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | SetPower$ 4\n",
    );
    assert!(half_pt.abilities[0].contains("Modifier::SetPower(4)"));
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | SetPower$ X\n\
         SVar:X:Count$ValidExile Card.YouOwn\n"
    ));
    // A condition is a rule of its own; one it cannot read refuses.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | \
         IsPresent$ Island.YouCtrl | PresentCompare$ EQ2\n"
    ));
    // A mode that is not Continuous is not this rule.
    assert!(refused(
        "Name:X\nTypes:Creature\nS:Mode$ CantPhase | ValidCard$ Card.Self\n"
    ));
}

#[test]
fn a_pump_of_x_reads_the_spells_x_and_keeps_its_sign() {
    let body = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$xPaid\n",
    );
    let a = body.abilities.join("");
    assert!(a.contains("power: Amount::X"), "{a}");
    assert!(a.contains("toughness: Amount::X"), "{a}");

    let body = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
         SVar:X:Count$xPaid\n",
    );
    // The sign lives in the variant, not in a negated `X` — the engine
    // negates `NegX` at the use site and would double-negate otherwise.
    assert!(body.abilities.join("").contains("Amount::NegX"));
}

/// The letter is not the number, and reading it as one gave seven cards
/// in this pool a pump of nothing.
///
/// One of the three cases this pinned has since moved, which is what a
/// pinned limitation is for. `Count$Valid …` is read now — see
/// [`Self::a_pump_counts_in_either_direction`] — so what is left here is
/// the counts that still have no shape in the DSL, and a reader that had
/// become a catch-all would fail on them.
#[test]
fn a_pump_of_a_counted_x_is_refused_and_not_read_as_the_announced_one() {
    // Domain and your hand size now lower to their exact `Amount` variants —
    // never to `Amount::X`.
    let domain = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$Domain\n",
    );
    assert!(
        domain
            .abilities
            .join("")
            .contains("Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)")
    );
    let hand = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
         SVar:X:Count$ValidHand Card.YouOwn\n",
    );
    assert!(
        hand.abilities
            .join("")
            .contains("Amount::Negated(&Amount::HandSize(PlayerRel::You))")
    );
    // Counts with no DSL representation still refuse rather than reading `X`.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$ValidExile Card.YouOwn\n"
    ));
    // And a **triggered** ability announces no number at all, so even
    // `Count$xPaid` is `x.unwrap_or(0)` there.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
         ValidCard$ Card.Self | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$xPaid\n"
    ));
    // The refusal names what the value resolves through, so the report
    // ranks the counts and not the letter.
    let script = parse(
        "Name:X\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$ValidExile Card.YouOwn\n",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("pump amount `+X` = `Count$ValidExile Card.YouOwn`")
    );
}

#[test]
fn a_refusal_says_which_key_it_choked_on() {
    // The report is only a worklist if it names the thing to build; a
    // second table of each rule's keys would rot, so the transcoder
    // reports what it actually failed to claim.
    let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | Bogus$ 2");
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("unclaimed parameter `Draw.Bogus`")
    );

    let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(refusal_reason(&script, &cats(), None), None, "read in full");

    // An unknown API is a missing effect, not a missing case in a rule
    // that exists, and is reported as its own kind. Leaving it silent
    // was worse than it looked: the report's fallback then guessed, and
    // named the first API *it* did not recognise — for a land whose
    // only unread line was `DB$ Discard`, that was the `R:Event$ Moved`
    // the transcoder had read perfectly well.
    let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Bogus | Defined$ Self");
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("effect `Bogus`")
    );

    // A rule that exists but met a value it cannot say says so.
    let script = parse(
        "Name:X\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ 1 | Duration$ Permanent",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("unreadable value in `Pump`")
    );

    // A static ability names the mode it cannot read, so the worklist
    // ranks `ReduceCost` and `Continuous` as the different work they are.
    let script = parse("Name:X\nTypes:Creature\nS:Mode$ CantPhase | ValidCard$ Card.Self");
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("static ability `S: Mode$ CantPhase`")
    );
}

/// Every refusal names one, and the counter-test is the same set read
/// the other way round.
///
/// A report whose largest bucket is "refused with no reason recorded" is
/// a worklist naming no work — the fault `refusal_cause` was written to
/// fix one level up, and it had simply moved down here: `?` is silent,
/// so a rule that met a cost, a trigger mode or a missing `SVar` it
/// could not read refused without saying which. Over the 33666 scripts
/// of the corpus that bucket is now empty, and this is the part of
/// that measurement a build can make.
#[test]
#[allow(clippy::too_many_lines)] // one case per refusal point, and the list is the point
fn a_refused_script_always_says_why() {
    // One per refusal point that used to be silent. The expected string
    // is spelled out rather than merely asserted non-empty, because
    // "some reason" is what a fallback produces too.
    let cases: &[(&str, &str)] = &[
        // `Discard<1/…>` is read now, so the refusal moved to the
        // count: `CostPart` carries one object per part, and a cost
        // paid with one card where the script charges two is a
        // discount.
        (
            "Name:X\nTypes:Creature\nA:AB$ Draw | Cost$ Discard<4/Card> | NumCards$ 1",
            "a cost naming `4` objects",
        ),
        (
            "Name:X\nTypes:Creature\nA:AB$ Draw | NumCards$ 1",
            "an activated ability with no `Cost$`",
        ),
        (
            "Name:X\nTypes:Creature\nT:Mode$ Championed | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "trigger mode `Championed`",
        ),
        (
            "Name:X\nTypes:Creature\n\
             T:Mode$ Phase | Phase$ Cleanup | ValidPlayer$ You | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "trigger at step `Cleanup`",
        ),
        (
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Origin$ Library | Destination$ Exile | \
             ValidCard$ Card.Self | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "trigger on a move from Library to Exile",
        ),
        (
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             ValidCard$ Card.Self | Execute$ NoSuchSVar",
            "`Execute$ NoSuchSVar` names no SVar",
        ),
        (
            "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | SubAbility$ NoSuchSVar",
            "`SubAbility$ NoSuchSVar` names no SVar",
        ),
        (
            "Name:X\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 1 | SubAbility$ Second\n\
             SVar:Second:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 1 | SubAbility$ Third\n\
             SVar:Third:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 1 | SubAbility$ Fourth\n\
             SVar:Fourth:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 1",
            "unsupported independent target in one chain",
        ),
        // Both doors a counter noun comes through. A word the registry
        // has not assigned an id to is refused at each of them, and
        // neither door may invent one — `counters::ASSIGNED` is where
        // that decision is made. Blaze counters (Obsidian Fireheart) are
        // printed by no card in this pool and have no id. (This was
        // hatchling until Eumidian Hatchery's word was assigned one.)
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ PutCounter | Cost$ T | CounterType$ BLAZE | CounterNum$ 1",
            "counter `BLAZE`",
        ),
        (
            "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<1/BLAZE>",
            "counter `BLAZE`",
        ),
        // The count is read before the noun, and it is a number or
        // nothing: every one of the 434 `AddCounter<…>` costs in the
        // corpus writes a literal 0-4, and an announced X in a cost is
        // `RemoveCounterSelfX`'s stage, not this one.
        (
            "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<X/M1M1>",
            "counter count `X`",
        ),
        // The two activation limits that are not a count. `GE4` is a
        // threshold on what has already been spent and `X` a number the
        // board works out; five corpus scripts print them between them.
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1 | ActivationLimit$ GE4",
            "activation limit `GE4`",
        ),
        (
            "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | ActivationLimit$ 1",
            "`ActivationLimit$` on a spell line",
        ),
        // A mixed-sign P/T counter is not a counter Magic prints, and
        // `CounterKind` has no way to say one.
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ PutCounter | Cost$ T | CounterType$ P1M1 | CounterNum$ 1",
            "counter `P1M1`",
        ),
        // Multi-return costs above three or variable X remain refused by count.
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ Untap | Cost$ Return<4/Forest> | ValidTgts$ Creature",
            "a cost naming `4` objects",
        ),
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ Untap | Cost$ Return<X/Forest> | ValidTgts$ Creature",
            "a cost naming `X` objects",
        ),
        // "Doesn't untap" is a rule about *your* untap step and about
        // nothing else this reader can say. Both keys are required
        // rather than defaulted: one script leaves the step off and two
        // replace the untap with a counter removal instead of stopping
        // it, and neither is this modifier.
        (
            "Name:X\nTypes:Artifact\n\
             R:Event$ Untap | ValidCard$ Card.Self | Layer$ CantHappen",
            "a `doesn't untap` during `any untap step`",
        ),
        (
            "Name:X\nTypes:Artifact\n\
             R:Event$ Untap | ValidCard$ Card.Self | \
             ValidStepTurnToController$ You | ReplaceWith$ RepRemoveCounter",
            "an untap replacement on layer `none`",
        ),
        (
            "Name:X\nTypes:Artifact\nR:Event$ Untap | \
             ValidStepTurnToController$ You | Layer$ CantHappen",
            "a `doesn't untap` with no `ValidCard$`",
        ),
        // The one `transcode` refuses after every rule has been read.
        (
            "Name:X\nTypes:Creature",
            "a script that reads as an empty card",
        ),
    ];
    for (script, why) in cases {
        let parsed = parse(script);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "this case is supposed to be refused: {script}"
        );
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(*why),
            "the reason given for: {script}"
        );
    }

    // The counter-test, and the reason the list above is not enough on
    // its own: a `deny` that named the wrong thing would still pass a
    // non-empty check. Each of these is one clause away from a case
    // above and is read in full.
    for script in [
        "Name:X\nTypes:Creature\nA:AB$ Draw | Cost$ T | NumCards$ 1",
        "Name:X\nTypes:Creature\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
        "Name:X\nTypes:Creature\n\
         T:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
        "Name:X\nTypes:Creature\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
         ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
        "Name:X\nTypes:Creature\n\
         A:AB$ PutCounter | Cost$ T | CounterType$ P1P1 | CounterNum$ 1",
        "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<1/M1M1>",
        "Name:X\nTypes:Creature\n\
         A:AB$ Mana | Cost$ T | Produced$ G | ActivationLimit$ 1",
        "Name:X\nTypes:Creature\n\
         A:AB$ PutCounter | Cost$ T | CounterType$ M0M1 | CounterNum$ 1",
        "Name:X\nTypes:Creature\n\
         A:AB$ Untap | Cost$ Return<1/Forest> | ValidTgts$ Creature",
        "Name:X\nTypes:Creature\n\
         A:AB$ Untap | Cost$ Return<1/CARDNAME> | ValidTgts$ Creature",
        "Name:X\nTypes:Artifact\n\
         R:Event$ Untap | ValidCard$ Card.Self | \
         ValidStepTurnToController$ You | Layer$ CantHappen",
        "Name:X\nTypes:Artifact\n\
         R:Event$ Untap | ActiveZones$ Battlefield | ValidCard$ Card.Self | \
         ValidStepTurnToController$ You | Layer$ CantHappen",
    ] {
        let parsed = parse(script);
        assert!(
            transcode(&parsed, &cats(), None).is_some(),
            "the near miss is supposed to be read: {script}"
        );
        assert_eq!(refusal_reason(&parsed, &cats(), None), None, "{script}");
    }
}

/// Doomskar's `K:Foretell:1 W W` is the later cast cost. The fixed {2}
/// special action belongs to the engine, and a non-mana or extended spelling
/// remains an honest stub rather than silently charging a different price.
#[test]
fn foretell_reads_only_its_complete_mana_cost() {
    let body = read("Name:Doomskar\nTypes:Sorcery\nK:Foretell:1 W W");
    assert_eq!(body.foretell.as_deref(), Some("Some(mana!(\"{1}{W}{W}\"))"));
    assert!(!body.is_empty());
    for script in [
        "Name:X\nTypes:Sorcery\nK:Foretell:",
        "Name:X\nTypes:Sorcery\nK:Foretell:Sac<1/Creature>",
        "Name:X\nTypes:Sorcery\nK:Foretell:1 W W:Unexpected",
    ] {
        let parsed = parse(script);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
    }
}

/// Flashback's mana and non-mana halves share the ordinary alternative-cost
/// payment path. Only parts that path actually pays may be emitted.
#[test]
fn flashback_reads_its_complete_supported_cost() {
    let mana = read("Name:X\nTypes:Instant\nK:Flashback:4 R\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(mana.flashback.as_deref(), Some("Some(mana!(\"{4}{R}\"))"));
    assert!(mana.flashback_parts.is_empty());

    let parts = read(
        "Name:X\nTypes:Instant\nK:Flashback:2 PayLife<3> Sac<1/Mountain>\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 1",
    );
    assert_eq!(parts.flashback.as_deref(), Some("Some(mana!(\"{2}\"))"));
    assert_eq!(
        parts.flashback_parts,
        ["PayLife(3)".to_string(), "Sacrifice(&COST1)".to_string()]
    );
    assert_eq!(
        parts.statics,
        concat!(
            "static COST1: Filter = Filter::And(&[",
            "Filter::HasSubtype(subtypes::land::MOUNTAIN), ",
            "Filter::ControlledByYou]);\n\n"
        )
    );

    let bounce = read("Name:X\nTypes:Instant\nK:Flashback:Return<1/Mountain>");
    assert_eq!(bounce.flashback.as_deref(), Some("Some(ManaCost::ZERO)"));
    assert_eq!(bounce.flashback_parts, ["ReturnToHand(&COST1)"]);

    for script in [
        "Name:X\nTypes:Instant\nK:Flashback:",
        "Name:X\nTypes:Instant\nK:Flashback:T",
        "Name:X\nTypes:Instant\nK:Flashback:Discard<1/Card>",
        "Name:X\nTypes:Instant\nK:Flashback:Return<1/CARDNAME>",
        "Name:X\nTypes:Instant\nK:Flashback:1 R:Unexpected",
    ] {
        assert!(refused(script), "{script}");
    }
}

/// Madness keeps zero, ordinary mana, announced X and its printed life payment
/// distinct. Every other or extended cost remains an honest stub.
#[test]
fn madness_reads_its_complete_supported_cost() {
    let zero = read("Name:X\nTypes:Creature\nK:Madness:0");
    assert_eq!(zero.madness.as_deref(), Some("Some(cost!(\"{0}\"))"));
    assert!(!zero.is_empty());

    let mana = read("Name:X\nTypes:Instant\nK:Madness:1 R R\nA:SP$ Draw | NumCards$ 1");
    assert_eq!(mana.madness.as_deref(), Some("Some(cost!(\"{1}{R}{R}\"))"));

    let life = read("Name:X\nTypes:Creature\nK:Madness:2 B PayLife<8>");
    assert_eq!(
        life.madness.as_deref(),
        Some("Some(cost!(\"{2}{B}\", PayLife(8)))")
    );

    let x = read(
        "Name:X\nTypes:Instant\nK:Madness:X U U\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\nSVar:X:Count$xPaid",
    );
    assert_eq!(x.madness.as_deref(), Some("Some(cost!(\"{X}{U}{U}\"))"));

    for script in [
        "Name:X\nTypes:Creature\nK:Madness:",
        "Name:X\nTypes:Creature\nK:Madness:X",
        "Name:X\nTypes:Creature\nK:Madness:T",
        "Name:X\nTypes:Creature\nK:Madness:Sac<1/Creature>",
        "Name:X\nTypes:Creature\nK:Madness:Discard<1/Card>",
        "Name:X\nTypes:Creature\nK:Madness:1 G:Unexpected",
    ] {
        assert!(refused(script), "{script}");
    }
}

/// Morph (CR 702.37) writes a fixed mana cost onto `FaceDef::morph`, including
/// `{0}`, multi-colour costs and two-colour hybrid symbols, and a card whose
/// only rules line is `K:Morph:<mana>` transcodes as a non-empty body.
#[test]
fn morph_reads_fixed_mana_costs_onto_the_face() {
    let only_morph = read("Name:X\nManaCost:3 U\nTypes:Creature\nPT:2/2\nK:Morph:2 U");
    assert_eq!(only_morph.morph.as_deref(), Some("Some(mana!(\"{2}{U}\"))"));
    assert!(!only_morph.is_empty());
    assert!(only_morph.notes.iter().any(|n| n == "morph"));

    let zero = read("Name:X\nManaCost:3\nTypes:Artifact Creature\nPT:2/2\nK:Morph:0");
    assert_eq!(zero.morph.as_deref(), Some("Some(mana!(\"{0}\"))"));

    let hybrid = read("Name:X\nManaCost:2 G U\nTypes:Creature\nPT:3/3\nK:Morph:1 GU");
    assert_eq!(hybrid.morph.as_deref(), Some("Some(mana!(\"{1}{G/U}\"))"));

    let multi = read("Name:X\nManaCost:3 W B G\nTypes:Creature\nPT:4/4\nK:Morph:2 W B G");
    assert_eq!(
        multi.morph.as_deref(),
        Some("Some(mana!(\"{2}{W}{B}{G}\"))")
    );
}

/// Morph costs that are empty, carry extra fields, contain `X`, charge non-mana
/// `{X}` and non-mana `Morph` / `Megamorph` costs lower cleanly, while
/// unrepresentable parts (`Reveal`), extra colon fields, duplicate/conflicting
/// face-down keywords, and empty costs stay refused with honest reasons.
#[test]
fn morph_refuses_non_mana_x_extra_and_megamorph_with_honest_reasons() {
    let reason = |script: &str| {
        let parsed = parse(script);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
        refusal_reason(&parsed, &cats(), None)
    };

    let x_morph = read("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:X R\nSVar:X:Count$xPaid");
    assert_eq!(x_morph.morph.as_deref(), Some("Some(mana!(\"{X}{R}\"))"));
    assert!(x_morph.morph_parts.is_empty());

    let life_morph = read("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:PayLife<5>");
    assert_eq!(life_morph.morph.as_deref(), Some("Some(ManaCost::ZERO)"));
    assert_eq!(life_morph.morph_parts, ["PayLife(5)"]);

    let custom_cats = SubtypeCatalogs {
        land: vec!["Mountain".into()],
        ..SubtypeCatalogs::default()
    };
    let parsed_sac = parse("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:Sac<1/Mountain>");
    let sac_morph = transcode(&parsed_sac, &custom_cats, None).expect("sac morph");
    assert_eq!(sac_morph.morph.as_deref(), Some("Some(ManaCost::ZERO)"));
    assert_eq!(sac_morph.morph_parts.len(), 1);
    assert!(sac_morph.morph_parts[0].starts_with("Sacrifice(&"));

    let mega = read("Name:X\nTypes:Creature\nPT:2/2\nK:Megamorph:2 G");
    assert_eq!(mega.megamorph.as_deref(), Some("Some(mana!(\"{2}{G}\"))"));

    assert_eq!(
        reason("Name:X\nTypes:Creature\nPT:2/2\nK:Morph"),
        Some("an empty `Morph` cost".to_string())
    );
    assert_eq!(
        reason("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:"),
        Some("an empty `Morph` cost".to_string())
    );
    assert_eq!(
        reason("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:1 U:Extra"),
        Some("a `Morph` with extra fields `1 U:Extra`".to_string())
    );
    assert_eq!(
        reason("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:Reveal<1/Card.Blue/blue>"),
        Some("cost `Reveal`".to_string())
    );
    assert_eq!(
        reason("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:1 U\nK:Morph:2 U"),
        Some("a second `Morph` cost on one card".to_string())
    );
    assert_eq!(
        reason("Name:X\nTypes:Creature\nPT:2/2\nK:Morph:1 U\nK:Megamorph:2 U"),
        Some("a conflicting `Megamorph` cost on one card".to_string())
    );
    assert!(refused(
        "Name:X\nTypes:Creature\nPT:2/2\nK:Morph:2 U\nA:AB$ UnknownEffect | Cost$ T"
    ));
}

/// `T:Mode$ TurnFaceUp` on `Card.Self` emits `Trigger::TurnedFaceUp`, while
/// triggers watching other permanents, non-battlefield zones, unclaimed keys or
/// `R:Event$ TurnFaceUp` replacement effects stay refused.
#[test]
fn turn_face_up_trigger_reads_self_and_refuses_other_subjects_and_zones() {
    let body = read(
        "Name:X\nManaCost:2 G\nTypes:Creature\nPT:2/2\nK:Morph:1 G\n\
         T:Mode$ TurnFaceUp | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert_eq!(body.morph.as_deref(), Some("Some(mana!(\"{1}{G}\"))"));
    assert_eq!(
        body.abilities,
        ["triggered!(Trigger::TurnedFaceUp, &[Effect::draw(1)])"]
    );

    let paired = read_with_tokens(
        "Name:X\nManaCost:2 R\nTypes:Creature\nPT:2/2\nK:Morph:2 R\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigTok\n\
         T:Mode$ TurnFaceUp | ValidCard$ Card.Self | TriggerZones$ Battlefield | Secondary$ True | Execute$ TrigTok\n\
         SVar:TrigTok:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ TriggeredCardController",
    );
    assert_eq!(paired.abilities.len(), 2);
    assert!(paired.abilities[1].starts_with("triggered!(Trigger::TurnedFaceUp,"));

    let reason = |script: &str| {
        let parsed = parse(script);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
        refusal_reason(&parsed, &cats(), None)
    };

    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ TurnFaceUp | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("a `TurnFaceUp` trigger with no `ValidCard$`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ TurnFaceUp | ValidCard$ Permanent | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("`TurnFaceUp` on `Permanent`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ TurnFaceUp | ValidCard$ Card.Self,Creature.Other+YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("`TurnFaceUp` on `Card.Self,Creature.Other+YouCtrl`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ TurnFaceUp | ValidCard$ Card.AttachedBy | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("`TurnFaceUp` on `Card.AttachedBy`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ TurnFaceUp | ValidCard$ Card.Self | TriggerZones$ Graveyard | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("`TriggerZones$ Graveyard`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ TurnFaceUp | ValidCard$ Card.Self | Secondary$ False | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("`Secondary$ False`".to_string())
    );
    let your_turn = read(
        "Name:X\nTypes:Creature\nPT:2/2\nK:Morph:1 R\n\
         T:Mode$ TurnFaceUp | ValidCard$ Card.Self | TriggerZones$ Battlefield | PlayerTurn$ True | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        your_turn
            .abilities
            .join("")
            .contains("condition = Some(Condition::YourTurn)")
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\nK:Morph:1 R\n\
             T:Mode$ TurnFaceUp | ValidCard$ Card.Self | TriggerZones$ Battlefield | FirstTime$ True | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("unclaimed parameter `TurnFaceUp.FirstTime`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Creature\nPT:2/2\nK:Morph:2 G\n\
             R:Event$ TurnFaceUp | ValidCard$ Card.Self | ActiveZones$ Battlefield | ReplaceWith$ Add\n\
             SVar:Add:DB$ PutCounter | Defined$ Self | CounterNum$ 1 | CounterType$ P1P1"
        ),
        Some("replacement `R: Event$ TurnFaceUp`".to_string())
    );
}

#[test]
fn cycled_trigger_reads_self_and_battlefield_subjects() {
    let self_tap = read(
        "Name:X\nTypes:Instant\nK:Cycling:1 U\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigTap | OptionalDecider$ You | \
         TriggerDescription$ When you cycle CARDNAME, you may tap target creature.\n\
         SVar:TrigTap:DB$ Tap | ValidTgts$ Creature",
    );
    assert_eq!(
        self_tap.abilities,
        [
            "activated!(cost!(\"{1}{U}\", DiscardSelf), &[Effect::draw(1)], zone = ActivationZone::Hand)",
            "triggered!(Trigger::CycledThis, &[Effect::MayDo { effects: &[Effect::TapTarget] }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let self_decider_ctrl = read(
        "Name:X\nTypes:Instant\nK:Cycling:1 W\n\
         Oracle:Cycling {1}{W} When you cycle X, you may gain 2 life.\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigGain | OptionalDecider$ TriggeredCardController\n\
         SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    );
    assert_eq!(
        self_decider_ctrl.abilities,
        [
            "activated!(cost!(\"{1}{W}\", DiscardSelf), &[Effect::draw(1)], zone = ActivationZone::Hand)",
            "triggered!(Trigger::CycledThis, &[Effect::MayDo { effects: &[Effect::gain_life(2)] }])"
        ]
    );

    let rift = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Cycled | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ TrigDmg\n\
         SVar:TrigDmg:AB$ DealDamage | Cost$ 1 | ValidTgts$ Any | NumDmg$ 2",
    );
    assert_eq!(
        rift.abilities,
        [
            "triggered!(Trigger::Cycled { by: PlayerRel::EachPlayer, filter: &Filter::Any }, &[Effect::PlayerMayPayThen { player: PlayerRel::You, mana: Amount::Fixed(1), effects: &[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::AnyTarget }] }], targets = Some(TargetReq::one(TargetSpec::AnyTarget)))"
        ]
    );

    let yours = read(
        "Name:X\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Cycled | ValidCard$ Card.YouOwn | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +2 | NumDef$ +0",
    );
    assert_eq!(
        yours.abilities,
        [
            "triggered!(Trigger::Cycled { by: PlayerRel::You, filter: &Filter::Any }, &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(2), toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );

    let opp_creature = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Cycled | ValidCard$ Creature.OppOwn | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert_eq!(
        opp_creature.abilities,
        [
            "triggered!(Trigger::Cycled { by: PlayerRel::EachOpponent, filter: &Filter::CREATURE }, &[Effect::draw(1)])"
        ]
    );

    let plague_hex = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ Cycled | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ TrigCounter\n\
         SVar:TrigCounter:DB$ PutCounter | Defined$ Self | CounterType$ PLAGUE | CounterNum$ 1",
    );
    assert_eq!(
        plague_hex.abilities,
        [
            "spell!(&[Effect::AttachSelf { target: TargetSpec::Object(&Filter::CREATURE) }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))",
            "triggered!(Trigger::Cycled { by: PlayerRel::EachPlayer, filter: &Filter::Any }, &[Effect::AddCounter { kind: counters::PLAGUE, amount: Amount::Fixed(1) }])"
        ]
    );
}

#[test]
fn cycled_trigger_refuses_mismatches() {
    let reason = |script: &str| {
        let parsed = parse(script);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
        refusal_reason(&parsed, &cats(), None)
    };

    assert_eq!(
        reason(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ Cycled | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("a `Cycled` trigger with no `ValidCard$`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Instant\n\
             T:Mode$ Cycled | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("a self-`Cycled` trigger with `TriggerZones$ Battlefield`".to_string())
    );
    assert_eq!(
        reason(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ Cycled | ValidCard$ Card | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ),
        Some("a `Cycled` trigger with no `TriggerZones$`".to_string())
    );
    assert!(refused(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Cycled | ValidCard$ Card.YouOwn | ValidPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
    ));
    assert!(refused(
        "Name:X\nTypes:Creature\nPT:1/1\n\
         T:Mode$ Cycled | ValidCard$ Card.Other | ValidPlayer$ You | TriggerZones$ Battlefield | FirstTime$ True | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
    ));
    // Decree of Savagery shape: Oracle/description says "you may", but Forge forgot
    // `OptionalDecider$ You` / `Optional$ True`; must stay refused rather than mandatory.
    assert!(refused(
        "Name:Decree Fixture\nTypes:Instant\nK:Cycling:4 G G\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigPutCounter | \
         TriggerDescription$ When you cycle CARDNAME, you may put four +1/+1 counters on target creature.\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 4"
    ));
}

#[test]
fn optional_unless_counter_and_end_step_blink_and_wizard_pump() {
    let complicate = read(
        "Name:X\nManaCost:2 U\nTypes:Instant\nK:Cycling:2 U\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigExile | \
         TriggerDescription$ When you cycle CARDNAME, you may counter target spell unless its controller pays {1}.\n\
         SVar:TrigExile:DB$ Counter | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card | Optional$ True | UnlessCost$ 1",
    );
    assert_eq!(
        complicate.abilities,
        [
            "activated!(cost!(\"{2}{U}\", DiscardSelf), &[Effect::draw(1)], zone = ActivationZone::Hand)",
            "triggered!(Trigger::CycledThis, &[Effect::MayDo { effects: &[Effect::PlayerMayPayOr { player: PlayerRel::ControllerOfTarget, mana: Amount::Fixed(1), effect: &Effect::CounterTargetSpell }] }], targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))"
        ]
    );
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | Optional$ Maybe | UnlessCost$ 1"
    ));

    let slide = read(
        "Name:X\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ Cycled | ValidCard$ Card | Execute$ TrigExile | TriggerZones$ Battlefield | OptionalDecider$ You | \
         TriggerDescription$ Whenever a player cycles a card, you may exile target creature. If you do, return that card to the battlefield under its owner's control at the beginning of the next end step.\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DelTrigEffect\n\
         SVar:DelTrigEffect:DB$ DelayedTrigger | Mode$ Phase | Phase$ End Of Turn | Execute$ TrigReturn | RememberObjects$ RememberedLKI | TriggerDescription$ Return the exiled card. | SubAbility$ DBCleanup | AILogic$ Always\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Exile | Destination$ Battlefield\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert_eq!(
        slide.abilities,
        [
            "triggered!(Trigger::Cycled { by: PlayerRel::EachPlayer, filter: &Filter::Any }, &[Effect::MayDo { effects: &[Effect::ExileAndReturnAtEndStep] }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let tapped_blink = read(
        "Name:X\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ Del\n\
         SVar:Del:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ Ret | RememberObjects$ RememberedLKI | SubAbility$ Clean\n\
         SVar:Ret:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Exile | Destination$ Battlefield | Tapped$ True\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert_eq!(
        tapped_blink.abilities,
        [
            "spell!(&[Effect::ExileAndReturnTappedAtEndStep], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let blink_base = "Name:X\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ Del\n\
         SVar:Del:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ Ret | RememberObjects$ RememberedLKI | SubAbility$ Clean\n\
         SVar:Ret:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Exile | Destination$ Battlefield\n\
         SVar:Clean:DB$ Cleanup | ClearRemembered$ True";
    for (from, to) in [
        (" | SubAbility$ Del", ""),
        ("Phase$ End of Turn", "Phase$ Upkeep"),
        (
            "RememberObjects$ RememberedLKI",
            "RememberObjects$ TriggeredCard",
        ),
        (
            "Destination$ Battlefield",
            "Destination$ Battlefield | GainControl$ True",
        ),
        (
            "DB$ Cleanup | ClearRemembered$ True",
            "DB$ Draw | Defined$ You | NumCards$ 1",
        ),
        (
            "Phase$ End of Turn",
            "Phase$ End of Turn | ValidPlayer$ You",
        ),
        ("RememberChanged$ True", "RememberChanged$ False"),
    ] {
        assert!(refused(&blink_base.replace(from, to)), "{from} -> {to}");
    }

    let sorcerer = read(
        "Name:Gempalm Fixture\nManaCost:2 U\nTypes:Creature Human Wizard\nPT:2/2\n\
         Oracle:Cycling {2}{U}\\nWhen you cycle Gempalm Fixture, Wizard creatures gain flying until end of turn.\n\
         K:Cycling:2 U\n\
         T:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigPumpAll\n\
         SVar:TrigPumpAll:DB$ PumpAll | ValidCards$ Wizard | KW$ Flying",
    );
    assert!(
        sorcerer
            .statics
            .contains("static PUMP_ALL1: Filter = Filter::And(&[Filter::CREATURE, Filter::HasSubtype(subtypes::creature::WIZARD)]);"),
        "{}",
        sorcerer.statics
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn reduce_cost_lowers_colored_dynamic_cycling_facedown_shared_and_self_stack() {
    // 1. Colored-only reduction (`IgnoreGeneric$ True`) and spillover reduction.
    let colored_only = read(
        "Name:Synthetic Cleric Reducer\nManaCost:W B\nTypes:Creature Wizard\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Wizard | Type$ Spell | Activator$ You | \
         Amount$ 1 | Color$ W B | IgnoreGeneric$ True",
    );
    assert_eq!(
        colored_only.abilities,
        [
            "static_ability!(Filter::And(&[Filter::HasSubtype(subtypes::creature::WIZARD), Filter::ControlledByYou]), Modifier::SpellsCostLessMana { cost: mana!(\"{W}{B}\"), colored_only: true })"
        ]
    );

    let spillover_self = read(
        "Name:Synthetic Storm Entity\nManaCost:3 U U\nTypes:Creature Elemental\nPT:3/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | EffectZone$ All | Type$ Spell | \
         Amount$ 1 | Color$ 2 U | CheckSVar$ X\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl",
    );
    assert_eq!(
        spillover_self.abilities,
        [
            "static_ability!(Filter::This, Modifier::SpellsCostLessMana { cost: mana!(\"{2}{U}\"), colored_only: false }, zone = ZoneRef::Stack, condition = Some(Condition::CastInstantOrSorceryThisTurn))"
        ]
    );

    // 2. Cycling cost reduction (`Fluctuator`).
    let cycler = read(
        "Name:Synthetic Cycling Engine\nManaCost:2\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Cycling | Activator$ You | Amount$ 2",
    );
    assert_eq!(
        cycler.abilities,
        ["static_ability!(Filter::ControlledByYou, Modifier::CyclingCostsLess(2))"]
    );

    // 3. Face-down creature spell reduction (`Dream Chisel` & `ValidCard$ ...faceDown`).
    let face_down = read(
        "Name:Synthetic Face-Down Chisel\nManaCost:2\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Creature | ValidSpell$ Spell.isCastFaceDown | \
         Activator$ You | Amount$ 1",
    );
    assert_eq!(
        face_down.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::FaceDown, Filter::ControlledByYou]), Modifier::SpellsCostLess(1))"
        ]
    );
    let mask_maker = read(
        "Name:Synthetic Mask Reducer\nManaCost:1 R\nTypes:Creature Goblin\nPT:1/1\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.faceDown | Type$ Spell | Activator$ You | Amount$ 2",
    );
    assert_eq!(
        mask_maker.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::FaceDown, Filter::ControlledByYou]), Modifier::SpellsCostLess(2))"
        ]
    );

    // 4. Shared creature type (`Mistform Warchief`) and chosen creature type (`Urza's Incubator` / `Morophon`).
    let shared_type = read(
        "Name:Synthetic Type-Shifter\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/3\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.sharesCreatureTypeWith | Type$ Spell | \
         Activator$ You | Amount$ 1",
    );
    assert_eq!(
        shared_type.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::SharesCreatureTypeWith(&Filter::This), Filter::ControlledByYou]), Modifier::SpellsCostLess(1))"
        ]
    );

    let incubator = read(
        "Name:Synthetic Kindred Incubator\nManaCost:3\nTypes:Artifact\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.ChosenType | Type$ Spell | Amount$ 2",
    );
    assert_eq!(incubator.enter_modifiers, ["EnterModifier::ChooseSubtype"]);
    assert_eq!(
        incubator.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]), Modifier::SpellsCostLess(2))"
        ]
    );

    let wubrg_chosen = read(
        "Name:Synthetic Omni-Shapeshifter\nManaCost:7\nTypes:Creature Shapeshifter\nPT:6/6\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ ReduceCost | ValidCard$ Card.ChosenType | Type$ Spell | Activator$ You | \
         Amount$ 1 | Color$ W U B R G | IgnoreGeneric$ True",
    );
    assert_eq!(
        wubrg_chosen.abilities,
        [
            "static_ability!(Filter::And(&[Filter::MatchesChosenTypeOfSource, Filter::ControlledByYou]), Modifier::SpellsCostLessMana { cost: mana!(\"{W}{U}{B}{R}{G}\"), colored_only: true })"
        ]
    );

    // 5. Dynamic `ZoneRef::Stack` self-cost statics and conditional reductions.
    let domain_self = read(
        "Name:Synthetic Domain Binding\nManaCost:5 W\nTypes:Enchantment\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | Amount$ X\n\
         SVar:X:Count$Domain",
    );
    assert_eq!(
        domain_self.abilities,
        [
            "static_ability!(Filter::This, Modifier::SpellsCostLessByAmount(Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)), zone = ZoneRef::Stack)"
        ]
    );

    let opp_count_self = read(
        "Name:Synthetic Opponents Protector\nManaCost:10 G\nTypes:Creature Giant\nPT:10/10\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ Stack | Amount$ X\n\
         SVar:X:Count$Valid Creature.OppCtrl",
    );
    assert_eq!(
        opp_count_self.abilities,
        [
            "static_ability!(Filter::This, Modifier::SpellsCostLessByAmount(Amount::CountOf { filter: &Filter::OPPONENT_CREATURE, zone: ZoneSel::Battlefield }), zone = ZoneRef::Stack)"
        ]
    );

    let wizard_bolt = read(
        "Name:Synthetic Wizard Bolt\nManaCost:2 R\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | \
         Amount$ 2 | IsPresent$ Wizard.YouCtrl\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        wizard_bolt.abilities[0],
        "static_ability!(Filter::This, Modifier::SpellsCostLess(2), zone = ZoneRef::Stack, condition = Some(Condition::ControlCount(&CHECK1, 1)))"
    );

    let low_life = read(
        "Name:Synthetic Desperate Avatar\nManaCost:6 W W\nTypes:Creature Angel\nPT:4/9\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | \
         Amount$ 6 | CheckSVar$ NeedHope | SVarCompare$ LE3\n\
         SVar:NeedHope:Count$YourLifeTotal",
    );
    assert_eq!(
        low_life.abilities,
        [
            "static_ability!(Filter::This, Modifier::SpellsCostLess(6), zone = ZoneRef::Stack, condition = Some(Condition::LifeAtMost(3)))"
        ]
    );

    let second_spell = read(
        "Name:Synthetic Focus\nManaCost:4 U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | \
         Amount$ 2 | CheckSVar$ X\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert_eq!(
        second_spell.abilities[0],
        "static_ability!(Filter::This, Modifier::SpellsCostLess(2), zone = ZoneRef::Stack, condition = Some(Condition::Not(&Condition::YouCastNoSpellThisTurn)))"
    );

    // 6. Refusals: chosen card-type mismatch (`Type$ Card`), dynamic colored reduction,
    // `EffectZone$ All` on other spells, `ValidTarget$`, `OnlyFirstSpell$`,
    // `ValidSpell$ Activated.!ManaAbility`, `EffectZone$ Graveyard`, `Amount$ 0`, and `SVarCompare$ EQ1`.
    for bad in [
        "Name:Bad\nTypes:Artifact\n\
         K:ETBReplacement:Other:ChooseCT\nSVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Card\n\
         S:Mode$ ReduceCost | ValidCard$ Card.ChosenType | Type$ Spell | Activator$ You | Amount$ 1",
        "Name:Bad\nTypes:Creature\nPT:8/8\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | Color$ G | Amount$ X\n\
         SVar:X:Count$Valid Creature.Green+YouCtrl",
        "Name:Bad\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Creature | Type$ Spell | EffectZone$ All | Amount$ 1",
        "Name:Bad\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | Amount$ 3 | ValidTarget$ Player.Opponent",
        "Name:Bad\nTypes:Creature\nPT:3/3\n\
         S:Mode$ ReduceCost | ValidCard$ Creature.faceDown | Type$ Spell | OnlyFirstSpell$ True | Activator$ You | Amount$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl",
        "Name:Bad\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Card | ValidSpell$ Activated.Cycling | Activator$ You | Amount$ X\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
        "Name:Bad\nTypes:Artifact\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | EffectZone$ Graveyard | Activator$ You | Amount$ 1",
        "Name:Bad\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Activator$ You | ValidSpell$ Activated.!ManaAbility | Amount$ 2 | MinMana$ 1",
        "Name:Bad\nTypes:Creature\nPT:2/2\n\
         S:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1 | CheckSVar$ X | SVarCompare$ EQ1\n\
         SVar:X:Count$ThisTurnCast_Card.YouCtrl",
        "Name:Bad\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | EffectZone$ All | Amount$ 0",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn kicker_variable_x_and_non_mana_spell_cost_helpers() {
    let custom_cats = SubtypeCatalogs {
        creature: vec!["Goblin".into(), "Vampire".into()],
        land: vec!["Forest".into()],
        ..SubtypeCatalogs::default()
    };

    // Variable-{X} kicker lines on `Tx::lower_kicker_line`.
    let parsed_x = parse("Name:Ancient\nManaCost:1 G\nOracle:Kicker {X}\nTypes:Creature\nPT:2/2");
    let mut tx = Tx::new(&parsed_x, &custom_cats, None);
    assert!(tx.lower_kicker_line("X").is_some());
    assert!(tx.has_x);
    assert_eq!(tx.body.additional_costs, ["cost!(\"{X}\")"]);

    let parsed_x2 =
        parse("Name:Keeper\nManaCost:2 W U\nOracle:Kicker {X}{2}\nTypes:Creature\nPT:2/2");
    let mut tx = Tx::new(&parsed_x2, &custom_cats, None);
    assert!(tx.lower_kicker_line("X 2").is_some());
    assert!(tx.has_x);
    assert_eq!(tx.body.additional_costs, ["cost!(\"{X}{2}\")"]);

    // Non-mana and mixed kickers.
    for (oracle, raw_kicker, want) in [
        (
            "Kicker—Return a creature you control to its owner's hand.",
            "Return<1/Creature>",
            "cost!(ReturnToHand(&Filter::YOUR_CREATURE))",
        ),
        (
            "Kicker—{2}{B}, Discard a creature card.",
            "2 B Discard<1/Creature>",
            "cost!(\"{2}{B}\", Discard(&Filter::CREATURE))",
        ),
        (
            "Kicker—Sacrifice two lands.",
            "Sac<2/Land>",
            "cost!(Sacrifice(&Filter::YOUR_LAND), Sacrifice(&Filter::YOUR_LAND))",
        ),
        ("Kicker—Pay 3 life.", "PayLife<3>", "cost!(PayLife(3))"),
        (
            "Kicker—Tap an untapped Vampire you control.",
            "tapXType<1/Vampire>",
            "cost!(TapN { filter: &COST1, count: 1 })",
        ),
        (
            "Kicker—Sacrifice an artifact or creature.",
            "Sac<1/Creature;Artifact/artifact or creature>",
            "cost!(Sacrifice(&COST1))",
        ),
    ] {
        let p = parse(&format!(
            "Name:K\nManaCost:1 B\nOracle:{oracle}\nTypes:Instant"
        ));
        let mut tx = Tx::new(&p, &custom_cats, None);
        assert!(tx.lower_kicker_line(raw_kicker).is_some(), "{raw_kicker}");
        assert!(!tx.has_x);
        assert_eq!(tx.body.additional_costs, [want], "{raw_kicker}");
    }

    // Refused kicker forms (`XMin1`, empty, unsupported token, `{X}` kicker beside `{X}` mana cost).
    for (mana, bad) in [
        ("1 U", "XMin1 X"),
        ("1 U", ""),
        ("1 G", "Forage"),
        ("X U", "X"),
    ] {
        let p = parse(&format!(
            "Name:BadK\nManaCost:{mana}\nTypes:Creature\nPT:1/1"
        ));
        let mut tx = Tx::new(&p, &custom_cats, None);
        assert!(
            tx.lower_kicker_line(bad).is_none(),
            "must refuse {mana} / {bad}"
        );
    }

    // Mandatory additional spell `CostPart` lowering.
    let spell_script = parse("Name:Costs\nManaCost:1 B\nTypes:Sorcery\nSVar:X:Count$xPaid");
    let mut tx = Tx::new(&spell_script, &custom_cats, None);
    tx.has_x = true;
    for (raw, want) in [
        (
            "1 B Sac<All/Creature>",
            vec!["CostPart::SacrificeAll(&Filter::YOUR_CREATURE)"],
        ),
        (
            "1 Sac<X/Creature>",
            vec!["CostPart::SacrificeX(&Filter::YOUR_CREATURE)"],
        ),
        ("G tapXType<X/Goblin>", vec!["CostPart::TapX(&COST1)"]),
        (
            "tapXType<2/Goblin>",
            vec!["CostPart::TapN { filter: &COST1, count: 2 }"],
        ),
        (
            "ExileFromTop<2/Card>",
            vec!["CostPart::ExileTopOfLibrary(2)"],
        ),
        (
            "ExileFromHand<1/Creature>",
            vec!["CostPart::ExileFromHand(&Filter::CREATURE)"],
        ),
        (
            "ExileFromGrave<1/Creature>",
            vec!["CostPart::ExileFromGraveyard(&Filter::CREATURE)"],
        ),
        ("Discard<1/Random>", vec!["CostPart::DiscardRandom"]),
        ("DiscardHand", vec!["CostPart::DiscardHand"]),
        ("PayLife<X>", vec!["CostPart::PayLifeX"]),
        ("PayEnergy<3>", vec!["CostPart::PayEnergy(3)"]),
        ("PayEnergy<X>", vec!["CostPart::PayEnergyX"]),
        (
            "G Return<1/Land>",
            vec!["CostPart::ReturnToHand(&Filter::YOUR_LAND)"],
        ),
    ] {
        let got = tx
            .parse_spell_cost_parts(raw)
            .unwrap_or_else(|| panic!("failed to parse {raw}"));
        assert_eq!(got, want, "{raw}");
    }

    // Without announced X, `X` object/life/energy costs must refuse.
    let mut no_x = Tx::new(&spell_script, &custom_cats, None);
    for bad in [
        "PayLife<X>",
        "PayEnergy<X>",
        "Sac<X/Creature>",
        "Sac<1/CARDNAME>",
        "Forage",
    ] {
        assert!(
            no_x.parse_spell_cost_parts(bad).is_none(),
            "must refuse {bad}"
        );
    }
}

#[test]
fn chosen_source_conditional_life_gain_reads_as_turn_long_shield() {
    let story = read(
        "Name:Synthetic Conditional Shield\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ ChooseSource | Choices$ Card,Emblem | SubAbility$ MakeShield\n\
         SVar:MakeShield:DB$ Effect | ReplacementEffects$ WaitForDamage | \
         ConditionDefined$ ChosenCard | ConditionPresent$ Card,Emblem | ConditionCompare$ GE1\n\
         SVar:WaitForDamage:Event$ DamageDone | ValidTarget$ You | \
         ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | ReplaceWith$ ConditionalLife | \
         PreventionEffect$ True\n\
         SVar:ConditionalLife:DB$ ImmediateTrigger | Execute$ LifeForDamage | ConditionCheckSVar$ ColoredSource\n\
         SVar:LifeForDamage:DB$ GainLife | Defined$ You | LifeAmount$ Prevented\n\
         SVar:Prevented:Spawner>ReplaceCount$DamageAmount\n\
         SVar:ColoredSource:ReplacedSource$Valid Card.BlackSource,Card.RedSource",
    );
    assert_eq!(
        story.abilities,
        [
            "spell!(&[Effect::PreventFromChosenSourceWithLifeCondition { target: Some(TargetSpec::Player(PlayerRel::You)), sources: &Filter::Any, gain_life_sources: &SOURCE1 }])"
        ]
    );
    assert!(story.statics.contains("Filter::Or"), "{}", story.statics);
    assert!(story.statics.contains("Color::Black"), "{}", story.statics);
    assert!(story.statics.contains("Color::Red"), "{}", story.statics);
}

#[test]
#[allow(clippy::too_many_lines)]
fn prevention_per_source_and_chosen_color_or_source_shields() {
    // 1. Urza's Armor & Spheres (`ValidTarget$ You` only -> Filter::Not(&Filter::Any)).
    let armor = read(
        "Name:Synthetic Armor\nManaCost:6\nTypes:Artifact\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You | \
         ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ 1",
    );
    assert_eq!(
        armor.abilities,
        [
            "static_ability!(Filter::Not(&Filter::Any), Modifier::PreventDamagePerSource { sources: &Filter::Any, include_you: true, combat_only: false, amount: Amount::Fixed(1) })"
        ]
    );

    let sphere = read(
        "Name:Synthetic Red Sphere\nManaCost:3 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.RedSource | \
         ValidTarget$ You | ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ 2",
    );
    assert_eq!(
        sphere.abilities,
        [
            "static_ability!(Filter::Not(&Filter::Any), Modifier::PreventDamagePerSource { sources: &SOURCE1, include_you: true, combat_only: false, amount: Amount::Fixed(2) })"
        ]
    );
    assert!(sphere.statics.contains("Color::Red"), "{}", sphere.statics);

    let artifact_sphere = read(
        "Name:Synthetic Artifact Sphere\nManaCost:3 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Artifact.inZoneBattlefield | \
         ValidTarget$ You | ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ 1",
    );
    assert!(
        artifact_sphere
            .statics
            .contains("static SOURCE1: Filter = Filter::And(&[Filter::InZone(ZoneRef::Battlefield), Filter::ARTIFACT]);"),
        "{}",
        artifact_sphere.statics
    );

    // 2. Cover of Winter (`You,Creature.YouCtrl` + `Count$CardCounters.AGE`).
    let cover = read(
        "Name:Synthetic Winter Cover\nManaCost:2 W\nTypes:Snow Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You,Creature.YouCtrl | \
         ValidSource$ Creature | IsCombat$ True | ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ X\n\
         SVar:X:Count$CardCounters.AGE",
    );
    assert_eq!(
        cover.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::PreventDamagePerSource { sources: &Filter::CREATURE, include_you: true, combat_only: true, amount: Amount::CountersOnSource(counters::AGE) })"
        ]
    );

    // 3. Daunting Defender / Shield (`PreventDamageToAffectedPerSource`).
    let defender = read(
        "Name:Synthetic Defender\nManaCost:4 W\nTypes:Creature Wizard\nPT:3/3\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Wizard.YouCtrl | \
         ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ 1",
    );
    assert_eq!(
        defender.abilities,
        [
            "static_ability!(Filter::And(&[Filter::HasSubtype(subtypes::creature::WIZARD), Filter::ControlledByYou]), Modifier::PreventDamageToAffectedPerSource { sources: &Filter::Any, combat_only: false, amount: Amount::Fixed(1) })"
        ]
    );

    // Callous Giant: prevent the whole event at three or less, never cap four.
    let giant = read(
        "Name:Synthetic Callous Giant\nManaCost:4 R R\nTypes:Creature Giant\nPT:4/4\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | \
         ValidTarget$ Card.Self | DamageAmount$ LE3",
    );
    assert_eq!(
        giant.abilities,
        ["static_ability!(Filter::This, Modifier::PreventDamageToAffectedIfAtMost(3))"]
    );

    // 4. Infinite persistent shields (`Glacial Chasm`, `Bubble Matrix`, `Dolmen Gate`).
    let personal_shield = read(
        "Name:Synthetic Sanctuary\nManaCost:2 W\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidTarget$ You | PlayerTurn$ True",
    );
    assert_eq!(
        personal_shield.abilities,
        [
            "static_ability!(Filter::This, Modifier::PreventDamageToYouFrom(&Filter::Any), condition = Some(Condition::YourTurn))"
        ]
    );

    let bubble = read(
        "Name:Synthetic Bubble\nManaCost:4\nTypes:Artifact\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidTarget$ Creature",
    );
    assert_eq!(
        bubble.abilities,
        ["static_ability!(Filter::CREATURE, Modifier::PreventDamageFrom(&Filter::Any))"]
    );

    let combat_attackers = read(
        "Name:Synthetic Dolmen\nManaCost:2\nTypes:Artifact\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | \
         ValidTarget$ Creature.attacking+YouCtrl | IsCombat$ True",
    );
    assert_eq!(
        combat_attackers.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::Attacking, Filter::ControlledByYou]), Modifier::PreventCombatDamageFrom(&Filter::Any))"
        ]
    );

    // 5. Chosen-color `ChooseSource` (Story Circle) & repeatable `ChooseSource` shields.
    let story = read(
        "Name:Synthetic Story\nManaCost:1 W W\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseColor\n\
         SVar:ChooseColor:DB$ ChooseColor | Defined$ You\n\
         A:AB$ ChooseSource | Cost$ W | Choices$ Card.ChosenColorSource | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | SubAbility$ DBCleanup | \
         ConditionDefined$ ChosenCard | ConditionPresent$ Card | ConditionCompare$ GE1\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCard+ChosenColorSource | \
         ValidTarget$ You | ReplaceWith$ ExileEffect | PreventionEffect$ True\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert_eq!(story.enter_modifiers, ["EnterModifier::ChooseColor"]);
    assert!(
        story.abilities[0].contains(
            "Effect::PreventNextFromChosenSource { sources: &Filter::OfSourceChosenColor, combat_only: false, all_but: 0, gain_life: false }"
        ),
        "{:?}",
        story.abilities
    );

    let surveillance = read(
        "Name:Synthetic Surveillance\nManaCost:3 W\nTypes:Enchantment\n\
         A:AB$ ChooseSource | Cost$ Sac<1/CARDNAME> | Choices$ Card,Emblem | RememberChosen$ True | \
         ChoiceTitle$ Choose a source | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | RememberObjects$ Remembered | \
         ConditionDefined$ Remembered | ConditionPresent$ Card,Emblem | ConditionCompare$ GE1 | \
         SubAbility$ DBCleanup\n\
         SVar:RPrevent:Event$ DamageDone | ActiveZones$ Command | \
         ValidSource$ Card.IsRemembered,Emblem.IsRemembered | ValidTarget$ You | Prevent$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True",
    );
    assert!(
        surveillance.abilities[0].contains(
            "Effect::PreventFromChosenSource { target: Some(TargetSpec::Player(PlayerRel::You)), sources: &Filter::Any, next_only: false, combat_only: false, all_but: 0, gain_life: false }"
        ),
        "{:?}",
        surveillance.abilities
    );

    let forge_tender = read(
        "Name:Synthetic Tender\nManaCost:W\nTypes:Creature Wizard\nPT:1/1\n\
         A:AB$ ChooseSource | Cost$ Sac<1/CARDNAME> | Choices$ Card.RedSource | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | SubAbility$ DBCleanup | \
         ConditionDefined$ ChosenCard | ConditionPresent$ Card | ConditionCompare$ GE1\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict+RedSource | Prevent$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        forge_tender.abilities[0].contains(
            "Effect::PreventFromChosenSource { target: None, sources: &SOURCE1, next_only: false, combat_only: false, all_but: 0, gain_life: false }"
        ),
        "{:?}",
        forge_tender.abilities
    );

    // 6. `ChooseColor` -> `Effect` turn-long color shield (`Prismatic Strands` / `Avacyn`).
    let strands = read(
        "Name:Synthetic Strands\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ ChooseColor | Defined$ You | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | ActiveZones$ Command | ValidSource$ Card.ChosenColor",
    );
    assert_eq!(
        strands.abilities,
        [
            "spell!(&[Effect::ChooseColor, Effect::PreventAllDamageFromMatchingThisTurn { target: None, sources: &Filter::OfSourceChosenColor, combat_only: false }])"
        ]
    );

    let guardian = read(
        "Name:Synthetic Guardian\nManaCost:3 W W\nTypes:Creature Angel\nPT:4/4\n\
         A:AB$ ChooseColor | Cost$ 1 W | Defined$ You | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | Name$ Guard | ValidTgts$ Creature.Other | \
         ReplacementEffects$ RPrevent | RememberObjects$ Targeted | ForgetOnMoved$ Battlefield\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | ActiveZones$ Command | \
         ValidTarget$ Card.IsRemembered | ValidSource$ Card.ChosenColor",
    );
    assert_eq!(
        guardian.abilities,
        [
            "activated!(cost!(\"{1}{W}\"), &[Effect::ChooseColor, Effect::PreventAllDamageFromMatchingThisTurn { target: Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE)), sources: &Filter::OfSourceChosenColor, combat_only: false }], target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE)))"
        ]
    );

    // 7. Staged `Circle of Solace` & targeted `ChooseSource` helpers in `prevention.rs`.
    let solace_script = parse(
        "Name:Synthetic Solace\nManaCost:3 W\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenType+inZoneBattlefield | \
         ValidTarget$ You | ReplaceWith$ ExileEffect | PreventionEffect$ True\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile",
    );
    let base_cats = cats();
    let mut solace_tx = Tx::new(&solace_script, &base_cats, None);
    solace_tx
        .body
        .enter_modifiers
        .push("EnterModifier::ChooseSubtype".to_string());
    let (_, effect_params) = Params::parse("DB$ Effect | ReplacementEffects$ RPrevent").unwrap();
    let mut solace_chain = Chain::default();
    let solace_fx = solace_tx
        .prevention_effect_shield(effect_params, &mut solace_chain)
        .expect("solace helper");
    assert_eq!(
        solace_fx,
        [
            "Effect::PreventNextFromMatchingSources { target: Some(TargetSpec::Player(PlayerRel::You)), sources: &SOURCE1, combat_only: false }"
        ]
    );
    assert!(
        solace_tx
            .body
            .statics
            .contains("static SOURCE1: Filter = Filter::And(&[Filter::InZone(ZoneRef::Battlefield), Filter::MatchesChosenTypeOfSource]);"),
        "{}",
        solace_tx.body.statics
    );

    let peddler_script = parse(
        "Name:Synthetic Peddler\nManaCost:1 W\nTypes:Creature\nPT:1/1\n\
         SVar:DBEffect:DB$ Effect | ValidTgts$ Creature | ReplacementEffects$ RPrevent | \
         ImprintCards$ Targeted | SubAbility$ DBCleanup | ConditionDefined$ ChosenCard | \
         ConditionPresent$ Card,Emblem\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | \
         ValidTarget$ Card.IsImprinted | ReplaceWith$ ExileEffect | PreventionEffect$ True\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    let mut peddler_tx = Tx::new(&peddler_script, &base_cats, None);
    let (_, cs_params) = Params::parse("DB$ ChooseSource | Choices$ Card,Emblem").unwrap();
    let mut peddler_chain = Chain::default();
    let peddler_fx = peddler_tx
        .prevent_from_chosen_source_chained(cs_params, Some("DBEffect"), &mut peddler_chain)
        .expect("peddler helper");
    assert_eq!(
        peddler_fx,
        [
            "Effect::PreventFromChosenSource { target: Some(TargetSpec::Object(&Filter::CREATURE)), sources: &Filter::Any, next_only: true, combat_only: false, all_but: 0, gain_life: false }"
        ]
    );
    assert_eq!(
        peddler_chain.target.as_deref(),
        Some("TargetSpec::Object(&Filter::CREATURE)")
    );

    // 8. Refusals: opponent-covering finite shield, noncombat `Prevent$ True`, `You` with combat-only
    // `Prevent$ True`, `You,Permanent.YouCtrl` with `IsCombat$ True`, and `ChosenColor` without `ChooseColor`.
    for bad in [
        "Name:Bad\nTypes:Creature\nPT:1/2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Spell | \
         ValidTarget$ Permanent,Player | ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ 1",
        "Name:Bad\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | \
         ValidSource$ Creature | IsCombat$ False",
        "Name:Bad\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | \
         ValidTarget$ You | IsCombat$ True",
        "Name:Bad\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | \
         ValidTarget$ You,Permanent.YouCtrl | IsCombat$ True",
        "Name:Bad\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | \
         ValidSource$ Card.ChosenColor | ValidTarget$ You",
        "Name:Bad\nTypes:Enchantment\n\
         R:Event$ DamageDone | ActiveZones$ Command | ValidTarget$ You | \
         ReplaceWith$ DBReplace | PreventionEffect$ True\n\
         SVar:DBReplace:DB$ ReplaceDamage | Amount$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave4_redirect_counters_flashback_tap_reduce_cost_and_computed_pt() {
    let custom_cats = SubtypeCatalogs {
        creature: vec!["Goblin".into(), "Human".into()],
        ..SubtypeCatalogs::default()
    };
    let read_cat = |script: &str| {
        let p = parse(script);
        transcode(&p, &custom_cats, None).unwrap_or_else(|| {
            panic!(
                "expected transcode for:\n{script}\nreason: {:?}",
                refusal_reason(&p, &custom_cats, None)
            )
        })
    };

    // 1. Damage redirection to self (`RedirectDamageToYou`) & counter absorption (`CountersPreventDamage`).
    let archangel = read(
        "Name:Synthetic Guardian\nManaCost:4 W W\nTypes:Creature Angel\nPT:5/8\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ You | \
         ReplaceWith$ DmgSelf | DamageTarget$ Self\n\
         SVar:DmgSelf:DB$ ReplaceEffect | VarName$ Affected | VarValue$ Self | VarType$ Card",
    );
    assert_eq!(
        archangel.abilities,
        ["static_ability!(Filter::This, Modifier::RedirectDamageToYou(&Filter::Any))"]
    );

    let bodyguard = read(
        "Name:Synthetic Guard\nManaCost:3 W W\nTypes:Creature Soldier\nPT:2/5\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Creature.unblocked | \
         ValidTarget$ You | ReplaceWith$ DmgSelf | IsPresent$ Card.Self+untapped | DamageTarget$ Self\n\
         SVar:DmgSelf:DB$ ReplaceEffect | VarName$ Affected | VarValue$ Self | VarType$ Card",
    );
    assert!(
        bodyguard.abilities[0].contains(
            "Modifier::RedirectDamageToYou(&SOURCE2), condition = Some(Condition::SourceMatches(&CHECK1))"
        ),
        "{:?}",
        bodyguard.abilities
    );
    assert!(
        bodyguard.statics.contains("Filter::Unblocked"),
        "{}",
        bodyguard.statics
    );

    let hydra = read(
        "Name:Synthetic Counter Hydra\nManaCost:X R R\nTypes:Creature Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:X\n\
         R:Event$ DamageDone | ValidTarget$ Card.Self | ActiveZones$ Battlefield | \
         ReplaceWith$ RepDmg | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:RepDmg:DB$ ReplaceDamage | Amount$ Z | SubAbility$ RemCtrs\n\
         SVar:RemCtrs:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ NumDmg\n\
         SVar:NumDmg:ReplaceCount$DamageAmount\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$CardCounters.P1P1\n\
         SVar:Z:SVar$NumDmg/LimitMax.Y",
    );
    assert_eq!(
        hydra.abilities,
        ["static_ability!(Filter::This, Modifier::CountersPreventDamage(CounterKind::P1P1))"]
    );

    // 2. Combat-only redirect, multi-recipient redirect, and all-or-nothing counter shield stay refused.
    for bad in [
        "Name:Bad\nTypes:Creature\nPT:2/5\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Creature.unblocked | \
         ValidTarget$ You | IsCombat$ True | ReplaceWith$ DmgSelf | DamageTarget$ Self\n\
         SVar:DmgSelf:DB$ ReplaceEffect | VarName$ Affected | VarValue$ Self | VarType$ Card",
        "Name:Bad\nTypes:Creature\nPT:2/5\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | \
         ValidTarget$ You,Permanent.Other+YouCtrl | ReplaceWith$ DmgSelf | DamageTarget$ Self\n\
         SVar:DmgSelf:DB$ ReplaceEffect | VarName$ Affected | VarValue$ Self | VarType$ Card",
        "Name:Bad\nTypes:Creature\nPT:0/0\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | \
         ReplaceWith$ Rem | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:Rem:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }

    // 3. `Effect | ReplacementEffects$` untargeted prevention wired through `regeneration.rs`
    // and `TapN` non-mana Flashback (`Prismatic Strands` shape).
    let strands = read(
        "Name:Synthetic Shield Strands\nManaCost:2 W\nTypes:Instant\n\
         K:Flashback:tapXType<1/Creature.White/white creature>\n\
         A:SP$ ChooseColor | Defined$ You | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | ActiveZones$ Command | ValidSource$ Card.ChosenColor",
    );
    assert_eq!(strands.flashback.as_deref(), Some("Some(ManaCost::ZERO)"));
    assert_eq!(
        strands.flashback_parts,
        ["TapN { filter: &COST1, count: 1 }"]
    );

    let solace = read(
        "Name:Synthetic Solace Card\nManaCost:3 W\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseCT\n\
         SVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         A:AB$ Effect | Cost$ 1 W | ReplacementEffects$ RPrevent\n\
         SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenType+inZoneBattlefield | \
         ValidTarget$ You | ReplaceWith$ ExileEffect | PreventionEffect$ True\n\
         SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile",
    );
    assert!(
        solace.abilities[0].contains("Effect::PreventNextFromMatchingSources"),
        "{:?}",
        solace.abilities
    );

    // 4. `S: Mode$ ReduceCost`: omitted `ValidCard$`, `PlayerTurn$ You`, combined `IsPresent$` + inline `CheckSVar$`,
    // and `PresentZone$ Hand` / `Graveyard` with `YouOwn`.
    let calendar = read(
        "Name:Synthetic Calendar\nManaCost:5\nTypes:Artifact\n\
         S:Mode$ ReduceCost | Type$ Spell | Activator$ You | Amount$ 1",
    );
    assert_eq!(
        calendar.abilities,
        ["static_ability!(Filter::ControlledByYou, Modifier::SpellsCostLess(1))"]
    );

    let drake = read(
        "Name:Synthetic Offturn Drake\nManaCost:2 U\nTypes:Creature Drake\nPT:2/3\n\
         S:Mode$ ReduceCost | Type$ Spell | Activator$ You | Amount$ 1 | Condition$ NotPlayerTurn",
    );
    assert_eq!(
        drake.abilities,
        [
            "static_ability!(Filter::ControlledByYou, Modifier::SpellsCostLess(1), condition = Some(Condition::Not(&Condition::YourTurn)))"
        ]
    );

    let modulation = read(
        "Name:Synthetic Turn Reduction\nManaCost:1 U\nTypes:Instant\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 1 | \
         PlayerTurn$ You | EffectZone$ All\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(
        modulation.abilities[0],
        "static_ability!(Filter::This, Modifier::SpellsCostLess(1), zone = ZoneRef::Stack, condition = Some(Condition::YourTurn))"
    );

    let pair_reduce = read_cat(
        "Name:Synthetic Pair Draw\nManaCost:2 U\nTypes:Sorcery\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | \
         IsPresent$ Creature.Human+YouCtrl | CheckSVar$ Count$Valid Creature.nonHuman+YouCtrl\n\
         A:SP$ Draw | NumCards$ 2",
    );
    assert_eq!(
        pair_reduce.abilities[0],
        "static_ability!(Filter::This, Modifier::SpellsCostLess(2), zone = ZoneRef::Stack, condition = Some(Condition::All(&[Condition::ControlCount(&CHECK1, 1), Condition::ControlCount(&CHECK2, 1)])))"
    );

    let no_lands_hand = read(
        "Name:Synthetic Hand Check\nManaCost:2 G G\nTypes:Creature Snake\nPT:2/3\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | \
         IsPresent$ Land.YouOwn | PresentZone$ Hand | PresentCompare$ EQ0",
    );
    assert_eq!(
        no_lands_hand.abilities,
        [
            "static_ability!(Filter::This, Modifier::SpellsCostLess(2), zone = ZoneRef::Stack, condition = Some(Condition::Not(&Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::LAND, zone: ZoneSel::HandYou }, 1))))"
        ]
    );

    let grave_reduce = read(
        "Name:Synthetic Grave Check\nManaCost:8 U U\nTypes:Creature Octopus\nPT:8/8\n\
         S:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 8 | EffectZone$ All | \
         IsPresent$ Instant.YouOwn,Sorcery.YouOwn | PresentZone$ Graveyard | PresentCompare$ GE8",
    );
    assert_eq!(
        grave_reduce.abilities,
        [
            "static_ability!(Filter::This, Modifier::SpellsCostLess(8), zone = ZoneRef::Stack, condition = Some(Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::INSTANT_OR_SORCERY, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, 8)))"
        ]
    );

    // 5. Computed P/T: `DistinctColorsAmong`, graveyard `YouOwn` fallthrough, `/Twice`.
    let color_flail = read(
        "Name:Synthetic Color Bearer\nManaCost:1 G W\nTypes:Creature Treefolk\nPT:0/0\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$Valid Permanent.YouCtrl$Colors",
    );
    assert_eq!(
        color_flail.abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPTByCount(PtCount::DistinctColorsAmong(&Filter::ControlledByYou)))"
        ]
    );

    let cda_colors = read(
        "Name:Synthetic Color Star\nManaCost:2 G\nTypes:Creature Elemental\nPT:*/*\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X\n\
         SVar:X:Count$Valid Permanent.YouCtrl$Colors",
    );
    assert_eq!(
        cda_colors.abilities,
        [
            "static_ability!(Filter::This, Modifier::CharacteristicPT { count: PtCount::DistinctColorsAmong(&Filter::ControlledByYou), toughness_plus: 0 })"
        ]
    );

    let grave_pt = read(
        "Name:Synthetic Grave Body\nManaCost:B B G G\nTypes:Creature Zombie\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn",
    );
    assert_eq!(
        grave_pt.abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPTByAmount { power: Amount::CountOf { filter: &Filter::CREATURE, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, toughness: Amount::CountOf { filter: &Filter::CREATURE, zone: ZoneSel::GraveyardOf(PlayerRel::You) } })"
        ]
    );

    let twice_goblin = read_cat(
        "Name:Synthetic Twice Brute\nManaCost:3 R\nTypes:Creature Goblin\nPT:2/4\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X\n\
         SVar:X:Count$Valid Goblin.Other+YouCtrl/Twice",
    );
    assert_eq!(
        twice_goblin.abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPTPerCount { filter: &COUNTED1, p: 2, t: 0 })"
        ]
    );
}

#[test]
fn wave5_graveyard_keywords_bestow_and_morph_unlocks() {
    let jump =
        read("Name:SparkJump\nManaCost:1 R\nTypes:Instant\nK:Jump-start\nA:SP$ Draw | NumCards$ 1");
    assert!(jump.jump_start);
    assert!(refused(
        "Name:BadJump\nTypes:Creature\nPT:1/1\nK:Jump-start"
    ));
    assert!(refused(
        "Name:DupJump\nTypes:Sorcery\nK:Jump-start\nK:Jump-start\nA:SP$ Draw | NumCards$ 1"
    ));

    let after = read(
        "Name:SparkAfter\nManaCost:2 W\nTypes:Sorcery\nK:Aftermath\nA:SP$ GainLife | LifeAmount$ 2",
    );
    assert!(after.aftermath);
    assert!(refused(
        "Name:BadAfter\nTypes:Creature\nPT:1/1\nK:Aftermath"
    ));

    let unearth_mana = read("Name:BoneWalker\nManaCost:1 B\nTypes:Creature\nPT:2/1\nK:Unearth:1 B");
    assert_eq!(unearth_mana.abilities, ["unearth!(\"{1}{B}\")"]);

    let unearth_energy =
        read("Name:BoltRunner\nManaCost:2 R\nTypes:Creature\nPT:3/1\nK:Unearth:1 R PayEnergy<2>");
    assert_eq!(
        unearth_energy.abilities,
        ["unearth!(cost!(\"{1}{R}\", PayEnergy(2)))"]
    );

    let encore = read("Name:EchoSprite\nManaCost:2 U\nTypes:Creature\nPT:1/3\nK:Encore:4 U");
    assert_eq!(encore.abilities, ["encore!(\"{4}{U}\")"]);

    let embalm = read("Name:SunInitiate\nManaCost:W\nTypes:Creature\nPT:1/1\nK:Embalm:2 W");
    assert_eq!(embalm.abilities, ["embalm!(\"{2}{W}\")"]);

    let eternalize_mana =
        read("Name:WiseAven\nManaCost:2 U\nTypes:Creature\nPT:2/1\nK:Eternalize:3 U U");
    assert_eq!(eternalize_mana.abilities, ["eternalize!(\"{3}{U}{U}\")"]);

    let eternalize_discard = read(
        "Name:AshScholar\nManaCost:2 B\nTypes:Creature\nPT:2/2\nK:Eternalize:3 B Discard<1/Card>",
    );
    assert_eq!(
        eternalize_discard.abilities,
        [
            "activated!(cost!(\"{3}{B}\", Discard(&Filter::Any), ExileSelf), &[Effect::CreateTokenCopyOfSource { mods: ETERNALIZE_MODS }], zone = ActivationZone::Graveyard, timing = ActivationTiming::SorcerySpeed)"
        ]
    );

    for bad_gy in [
        "Name:Bad1\nTypes:Creature\nPT:1/1\nK:Unearth:",
        "Name:Bad2\nTypes:Creature\nPT:1/1\nK:Encore:T",
        "Name:Bad3\nTypes:Creature\nPT:1/1\nK:Embalm:X W\nSVar:X:Count$xPaid",
        "Name:Bad4\nTypes:Creature\nPT:1/1\nK:Eternalize:1 B:Extra",
    ] {
        assert!(refused(bad_gy), "must refuse: {bad_gy}");
    }

    let bestow =
        read("Name:NimbusEidolon\nManaCost:1 U\nTypes:Enchantment Creature\nPT:1/2\nK:Bestow:4 U");
    assert_eq!(bestow.bestow.as_deref(), Some("Some(cost!(\"{4}{U}\"))"));

    let bestow_hint = read(
        "Name:SirenEidolon\nManaCost:U\nTypes:Enchantment Creature\nPT:1/1\nK:Bestow:5 U U:GainControl",
    );
    assert_eq!(
        bestow_hint.bestow.as_deref(),
        Some("Some(cost!(\"{5}{U}{U}\"))")
    );

    let bestow_x = read(
        "Name:VariableEidolon\nManaCost:G\nTypes:Enchantment Creature\nPT:0/0\nK:Bestow:X G\nSVar:X:Count$xPaid",
    );
    assert_eq!(bestow_x.bestow.as_deref(), Some("Some(cost!(\"{X}{G}\"))"));

    for bad_bestow in [
        "Name:BadB1\nTypes:Creature\nPT:2/2\nK:Bestow:3 W",
        "Name:BadB2\nTypes:Enchantment Creature\nPT:2/2\nK:Bestow:2 U Discard<1/Card>",
        "Name:BadB3\nTypes:Enchantment Creature\nPT:2/2\nK:Bestow:2 U:UnknownHint",
        "Name:BadB4\nTypes:Enchantment Creature\nPT:2/2\nK:Bestow:",
    ] {
        assert!(refused(bad_bestow), "must refuse: {bad_bestow}");
    }
}

#[test]
fn wave6_grant_activated_and_triggered_definitions() {
    let equip_ping = read(
        "Name:RunedBow\nManaCost:2\nTypes:Artifact Equipment\n\
         K:Equip:3\n\
         S:Mode$ Continuous | AffectedDefined$ Equipped | AddAbility$ Ping\n\
         SVar:Ping:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | SorcerySpeed$ True | ActivationLimit$ 1",
    );
    assert!(
        equip_ping
            .statics
            .contains("static GRANTED_ACTIVATED_1: AbilityDef = activated!(")
            && equip_ping
                .statics
                .contains("timing = ActivationTiming::SorcerySpeed")
            && equip_ping.statics.contains("limit = ActivationLimit::PerTurn(1)")
            && equip_ping.abilities.iter().any(|a| {
                a == "static_ability!(Filter::AttachedToBySource, Modifier::GrantActivatedDefinition(&GRANTED_ACTIVATED_1))"
            }),
        "{}\n{:?}",
        equip_ping.statics,
        equip_ping.abilities
    );

    let grant_trig = read(
        "Name:SparkHarness\nManaCost:2\nTypes:Artifact Equipment\n\
         K:Equip:1\n\
         S:Mode$ Continuous | AffectedDefined$ Equipped | AddPower$ 1 | AddTrigger$ AtkPing | AddSVar$ PingDmg\n\
         SVar:AtkPing:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ PingDmg\n\
         SVar:PingDmg:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 2",
    );
    assert!(
        grant_trig
            .statics
            .contains("static GRANTED_TRIGGER_1: AbilityDef = triggered!(")
            && grant_trig.abilities.iter().any(|a| {
                a == "static_ability!(Filter::AttachedToBySource, Modifier::GrantTriggeredDefinition(&GRANTED_TRIGGER_1))"
            }),
        "{}\n{:?}",
        grant_trig.statics,
        grant_trig.abilities
    );

    let threshold_trig = read(
        "Name:GraveHall\nManaCost:G W\nTypes:Enchantment\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddTrigger$ EnemyCast | Condition$ Threshold\n\
         SVar:EnemyCast:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ Draw1 | OptionalDecider$ You\n\
         SVar:Draw1:DB$ Draw | NumCards$ 1",
    );
    assert!(
        threshold_trig
            .statics
            .contains("static GRANTED_TRIGGER_1: AbilityDef = triggered!(")
            && threshold_trig.abilities.iter().any(|a| {
                a == "static_ability!(Filter::This, Modifier::GrantTriggeredDefinition(&GRANTED_TRIGGER_1), condition = Some(Condition::GraveyardCountAtLeast(7)))"
            }),
        "{}\n{:?}",
        threshold_trig.statics,
        threshold_trig.abilities
    );

    for bad in [
        "Name:BadZoneTrig\nTypes:Artifact Equipment\nK:Equip:1\n\
         S:Mode$ Continuous | AffectedDefined$ Equipped | AddTrigger$ GyTrig\n\
         SVar:GyTrig:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Graveyard | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1",
        "Name:MissingSVar\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddTrigger$ Ok | AddSVar$ MissingHelper\n\
         SVar:Ok:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ Draw | NumCards$ 1",
        "Name:BadAltAbil\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddAbility$ AltAb\n\
         SVar:AltAb:AB$ Draw | Cost$ 1 | AlternateCost$ PayLife<2> | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave6_phasing_lock_roll_dice_vanishing_and_etb_counters() {
    let phase_lock = read(
        "Name:PhaseAnchor\nManaCost:1 U\nTypes:Enchantment\n\
         S:Mode$ CantPhaseOut | ValidCard$ Creature.YouCtrl",
    );
    assert_eq!(
        phase_lock.abilities,
        ["static_ability!(Filter::YOUR_CREATURE, Modifier::CantPhaseOut)"]
    );
    assert!(refused(
        "Name:BadPhase\nTypes:Enchantment\n\
         S:Mode$ CantPhaseOut | EffectZone$ Graveyard | ValidCard$ Creature.YouCtrl"
    ));

    let dice_adv = read(
        "Name:FortuneSprite\nManaCost:1 U\nTypes:Creature\nPT:1/2\n\
         R:Event$ RollDice | ActiveZones$ Battlefield | ValidPlayer$ You | ReplaceWith$ PlusRoll\n\
         SVar:PlusRoll:DB$ ReplaceEffect | VarName$ Number | VarValue$ ReplaceCount$Number/Plus.1 | SubAbility$ IgnoreLowest\n\
         SVar:IgnoreLowest:DB$ ReplaceEffect | VarName$ Ignore | VarValue$ ReplaceCount$Ignore/Plus.1",
    );
    assert_eq!(
        dice_adv.abilities,
        ["static_ability!(Filter::Any, Modifier::RollAdditionalDieIgnoreLowest)"]
    );
    assert!(refused(
        "Name:BadSidesDice\nTypes:Creature\nPT:1/1\n\
         R:Event$ RollDice | ActiveZones$ Battlefield | ValidPlayer$ You | ValidSides$ 6 | ReplaceWith$ PlusRoll\n\
         SVar:PlusRoll:DB$ ReplaceEffect | VarName$ Number | VarValue$ ReplaceCount$Number/Plus.1 | SubAbility$ IgnoreLowest\n\
         SVar:IgnoreLowest:DB$ ReplaceEffect | VarName$ Ignore | VarValue$ ReplaceCount$Ignore/Plus.1"
    ));

    let vanish_n = read("Name:TimerWurm\nManaCost:3 G\nTypes:Creature\nPT:7/6\nK:Vanishing:2");
    assert_eq!(
        vanish_n.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::Time, amount: Amount::Fixed(2) }"]
    );
    assert_eq!(
        vanish_n.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, \
             &[Effect::RemoveCounterSelf { kind: CounterKind::Time, n: 1 }], \
             condition = Some(Condition::CountersOnSelf(CounterKind::Time, 1)))",
            "triggered!(Trigger::LastCounterRemoved { kind: CounterKind::Time, by: None }, \
             &[Effect::SacrificeSelf])",
        ]
    );

    let vanish_bare = read("Name:TimeTide\nManaCost:2 U\nTypes:Creature\nPT:0/3\nK:Vanishing");
    assert!(vanish_bare.enter_modifiers.is_empty());
    assert_eq!(vanish_bare.abilities.len(), 2);
    assert!(refused(
        "Name:BadVan\nTypes:Creature\nPT:1/1\nK:Vanishing:0"
    ));

    let life_etb = read(
        "Name:SunWurm\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         K:etbCounter:P1P1:X:no Condition:desc\nSVar:X:Count$LifeYouGainedThisTurn",
    );
    assert_eq!(
        life_etb.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::LifeGainedThisTurn(PlayerRel::You) }"
        ]
    );

    let domain_etb = read(
        "Name:RealmWalker\nManaCost:3 G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X::Domain desc\nSVar:X:Count$Domain",
    );
    assert_eq!(
        domain_etb.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::BasicLandTypesAmong(&Filter::YOUR_LAND) }"
        ]
    );

    let colors_etb = read(
        "Name:PrismBody\nManaCost:4\nTypes:Artifact Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\nSVar:X:Count$Valid Permanent.YouCtrl$Colors",
    );
    assert_eq!(
        colors_etb.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::DistinctColorsAmong(&Filter::ControlledByYou) }"
        ]
    );

    let total_mana_etb = read(
        "Name:ManaShell\nManaCost:X U\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X\nSVar:X:Count$TotalManaSpent",
    );
    assert_eq!(
        total_mana_etb.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::ManaSpentToCast }"
        ]
    );

    assert!(refused(
        "Name:BadSelfPow\nTypes:Creature\nPT:2/2\n\
         K:etbCounter:P1P1:X\nSVar:X:Count$CardPower"
    ));
}

#[test]
fn wave6b_miracle_dash_blitz_spectacle_surge_prowl_and_conditions() {
    let miracle_mana = read(
        "Name:BeaconBolt\nManaCost:2 W\nTypes:Instant\n\
         K:Miracle:W\nA:SP$ GainLife | LifeAmount$ 3",
    );
    assert_eq!(
        miracle_mana.miracle.as_deref(),
        Some("Some(mana!(\"{W}\"))")
    );

    let miracle_x = read(
        "Name:SkyCall\nManaCost:X W W\nTypes:Sorcery\n\
         K:Miracle:X W\nSVar:X:Count$xPaid\nA:SP$ GainLife | LifeAmount$ X",
    );
    assert_eq!(
        miracle_x.miracle.as_deref(),
        Some("Some(mana!(\"{X}{W}\"))")
    );

    let dash = read("Name:RedRunner\nManaCost:2 R\nTypes:Creature\nPT:2/1\nK:Dash:1 R");
    assert_eq!(dash.dash.as_deref(), Some("Some(mana!(\"{1}{R}\"))"));

    let blitz_mana = read("Name:AshCharger\nManaCost:2 R\nTypes:Creature\nPT:3/1\nK:Blitz:1 R");
    assert_eq!(blitz_mana.blitz.as_deref(), Some("Some(cost!(\"{1}{R}\"))"));

    let blitz_life = read(
        "Name:BloodBruiser\nManaCost:1 B\nTypes:Creature\nPT:3/2\n\
         K:Blitz:2 B B PayLife<2>",
    );
    assert_eq!(
        blitz_life.blitz.as_deref(),
        Some("Some(cost!(\"{2}{B}{B}\", PayLife(2)))")
    );

    let spectacle = read(
        "Name:TorchSpark\nManaCost:2 R\nTypes:Sorcery\n\
         K:Spectacle:R\nA:SP$ LoseLife | ValidTgts$ Opponent | LifeAmount$ 3",
    );
    assert_eq!(
        spectacle.alternative_costs,
        ["AlternativeCost::spectacle(cost!(\"{R}\"))"]
    );

    let surge_x = read(
        "Name:TideSurge\nManaCost:X X R\nTypes:Instant\n\
         K:Surge:X R\nSVar:X:Count$xPaid\nA:SP$ LoseLife | ValidTgts$ Opponent | LifeAmount$ X",
    );
    assert_eq!(
        surge_x.alternative_costs,
        ["AlternativeCost::surge(cost!(\"{X}{R}\"))"]
    );

    let prowl = read(
        "Name:NightProwler\nManaCost:3 U\nTypes:Creature Rogue\nPT:2/2\n\
         K:Prowl:1 U",
    );
    assert_eq!(
        prowl.alternative_costs,
        ["AlternativeCost::prowl(cost!(\"{1}{U}\"))"]
    );

    let conds = read(
        "Name:BloodSeer\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         A:AB$ Draw | Cost$ T | ConditionCheckSVar$ OppLost | ConditionSVarCompare$ GE1 | NumCards$ 1\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 1 | CheckSVar$ AltPaid | SVarCompare$ GE1\n\
         SVar:OppLost:PlayerCountRegisteredOpponents$HasPropertyLostLifeThisTurn\n\
         SVar:AltPaid:Count$AltCost.1.0",
    );
    let abilities = conds.abilities.join("\n");
    assert!(
        abilities.contains("condition: Condition::OpponentLostLifeThisTurn")
            && abilities.contains("condition = Some(Condition::AltCostPaid)"),
        "{abilities}"
    );
}

#[test]
fn wave6b_alternate_keyword_refusals() {
    for bad in [
        "Name:LandMiracle\nTypes:Land\nK:Miracle:1 W",
        "Name:BadMiracleX\nManaCost:2 W\nTypes:Sorcery\nK:Miracle:X W\nA:SP$ GainLife | LifeAmount$ 2",
        "Name:DupMiracle\nManaCost:2 W\nTypes:Instant\nK:Miracle:W\nK:Miracle:1 W\nA:SP$ GainLife | LifeAmount$ 2",
        "Name:SpellDash\nManaCost:1 R\nTypes:Instant\nK:Dash:R\nA:SP$ GainLife | LifeAmount$ 1",
        "Name:DashX\nManaCost:2 R\nTypes:Creature\nPT:1/1\nK:Dash:X R\nSVar:X:Count$xPaid",
        "Name:SpellBlitz\nManaCost:1 B\nTypes:Sorcery\nK:Blitz:B\nA:SP$ GainLife | LifeAmount$ 1",
        "Name:BlitzDiscard\nManaCost:2 R\nTypes:Creature\nPT:2/2\nK:Blitz:1 R Discard<1/Card>",
        "Name:BlitzExtra\nManaCost:2 R\nTypes:Creature\nPT:2/2\nK:Blitz:1 R:Extra",
        "Name:LandSurge\nTypes:Land\nK:Surge:1 U",
        "Name:TwoAlts\nManaCost:2 U\nTypes:Instant\nK:Surge:U\nK:Prowl:1 U\nA:SP$ Draw | NumCards$ 1",
        "Name:ProwlDiscard\nManaCost:2 B\nTypes:Instant\nK:Prowl:B Discard<1/Card>\nA:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave7_prepare_and_copy_face_from_transcode() {
    let direct = read(
        "Name:FrontMage\nManaCost:2 W\nTypes:Creature Human\nPT:2/2\n\
         K:ETBReplacement:Other:DBPrepare\n\
         SVar:DBPrepare:DB$ AlterAttribute | Attributes$ Prepared\n\
         AlternateMode:Prepare\n\
         ALTERNATE\n\
         Name:BackBolt\nManaCost:R\nTypes:Instant\n\
         A:SP$ LoseLife | ValidTgts$ Opponent | LifeAmount$ 2",
    );
    assert!(direct.back_is_prepare);
    assert_eq!(direct.enter_modifiers, ["EnterModifier::Prepared"]);
    assert!(direct.back.is_some());

    for bad in [
        "Name:FrontMage\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         K:ETBReplacement:Other:DBPrepare\n\
         SVar:DBPrepare:DB$ AlterAttribute | Attributes$ Prepared\n\
         AlternateMode:Prepare\nALTERNATE\nName:Missing\nCopyFaceFrom:Missing",
        "Name:BadPrep\nManaCost:W\nTypes:Creature\nPT:1/1\n\
         K:ETBReplacement:Other:DBPrep\n\
         SVar:DBPrep:DB$ AlterAttribute | Attributes$ Monstrous",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave7_etb_counter_extensions() {
    let spent_body = read(
        "Name:ManaHydra\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no condition\nSVar:X:Count$TotalManaSpent\nK:Haste",
    );
    assert_eq!(
        spent_body.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::ManaSpentToCast }"
        ]
    );
    let broken_after = parse(
        "Name:ManaHydra\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no condition\nSVar:X:Count$TotalManaSpent\nK:WeirdFutureMech",
    );
    let reason = refusal_reason(&broken_after, &cats(), None).expect("later rule must refuse");
    assert_ne!(reason, "keyword `etbCounter`", "{reason}");

    let dual = read(
        "Name:DualBeast\nManaCost:3 G\nTypes:Creature\nPT:2/2\n\
         K:Kicker:1 W:2 B\nK:etbCounter:P1P1:2:CheckSVar$ Kicked\n\
         SVar:Kicked:Count$Kicked.1.0",
    );
    assert_eq!(
        dual.enter_modifiers,
        ["EnterModifier::IfKicked { index: None, counters: 2, modifiers: &[] }"]
    );

    let double_x = read(
        "Name:TwinHydra\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:Y:no condition\nSVar:Y:SVar$X/Twice\nSVar:X:Count$xPaid",
    );
    assert_eq!(
        double_x.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::DoubleX }"]
    );

    let x_plus = read(
        "Name:PlusHydra\nManaCost:X R\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:Y:no condition\nSVar:Y:Count$xPaid/Plus.1",
    );
    assert_eq!(
        x_plus.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::Plus { base: &Amount::X, offset: 1 } }"
        ]
    );

    let opp_lost = read(
        "Name:BloodSlime\nManaCost:2 B\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no condition\nSVar:X:Count$LifeOppsLostThisTurn",
    );
    assert_eq!(
        opp_lost.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::OpponentsLifeLostThisTurn }"
        ]
    );

    let died = read(
        "Name:CarrionBeast\nManaCost:3 B\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no condition\nSVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature",
    );
    assert_eq!(
        died.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::CreaturesDiedThisTurn }"
        ]
    );

    let grave = read(
        "Name:CryptWurm\nManaCost:4 B\nTypes:Creature\nPT:0/0\n\
         K:etbCounter:P1P1:X:no condition\nSVar:X:Count$LastStateGraveyard Creature.YouOwn",
    );
    assert!(
        grave.enter_modifiers[0].contains("ZoneSel::GraveyardOf(PlayerRel::You)"),
        "{:?}",
        grave.enter_modifiers
    );

    for bad in [
        "Name:Cond\nManaCost:2 G\nTypes:Creature\nPT:1/1\nK:etbCounter:P1P1:1:CheckSVar$ L\nSVar:L:Count$Valid Land",
        "Name:DualLoyalty\nManaCost:3 G\nTypes:Planeswalker\nLoyalty:2\nK:Kicker:W:U\nK:etbCounter:LOYALTY:1:CheckSVar$ K\nSVar:K:Count$Kicked.1.0",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave8a_combat_keywords_and_command_zone_activations_and_reductions() {
    let kws = read(
        "Name:WarCaptain\nManaCost:3 W\nTypes:Creature\nPT:2/2\n\
         K:Battle cry\nK:Dethrone\nK:Melee\nK:Mentor\nK:Training",
    );
    assert_eq!(
        kws.keywords,
        [
            "KeywordSet::BATTLE_CRY",
            "KeywordSet::DETHRONE",
            "KeywordSet::MELEE",
            "KeywordSet::MENTOR",
            "KeywordSet::TRAINING",
        ]
    );

    let granted_kws = read(
        "Name:RallyBanner\nManaCost:2 R\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Battle Cry & Mentor",
    );
    assert_eq!(
        granted_kws.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::AddKeyword(KeywordSet::BATTLE_CRY.union(KeywordSet::MENTOR)))"
        ]
    );

    let cmd_act = read(
        "Name:CommandMage\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 3 | NumCards$ 1 | ActivationZone$ Command\n\
         A:AB$ GainLife | Cost$ 1 | LifeAmount$ 2 | ActivationZone$ Battlefield,Command",
    );
    assert_eq!(
        cmd_act.abilities,
        [
            "activated!(cost!(\"{3}\"), &[Effect::draw(1)], zone = ActivationZone::Command)",
            "activated!(cost!(\"{1}\"), &[Effect::gain_life(2)], zone = ActivationZone::BattlefieldOrCommand)",
        ]
    );

    let cmd_modal = read(
        "Name:CommandCharm\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Charm | Cost$ 2 | ActivationZone$ Command | Choices$ M1,M2\n\
         SVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        cmd_modal.abilities[0].contains("zone = ActivationZone::Command"),
        "{:?}",
        cmd_modal.abilities
    );

    let eminence = read(
        "Name:ElderWyrm\nManaCost:4 R G\nTypes:Creature Goblin\nPT:6/6\n\
         S:Mode$ ReduceCost | EffectZone$ Battlefield,Command | ValidCard$ Goblin.Other | Type$ Spell | Activator$ You | Amount$ 1\n\
         S:Mode$ ReduceCost | EffectZone$ Command | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1",
    );
    assert!(
        eminence.abilities[0].contains("zone = ZoneRef::BattlefieldOrCommand")
            && eminence.abilities[1].contains("zone = ZoneRef::Command"),
        "{:?}",
        eminence.abilities
    );

    let cant_cast = read(
        "Name:VoidSeal\nTypes:Enchantment\n\
         S:Mode$ CantBeCast | EffectZone$ Command | ValidCard$ Instant,Sorcery | Caster$ You",
    );
    assert!(
        cant_cast.abilities[0].contains("zone = ZoneRef::Command")
            && cant_cast.abilities[0].contains("Modifier::PlayersCantCast"),
        "{:?}",
        cant_cast.abilities
    );

    let double_kicked = read(
        "Name:ScaleMind\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         K:Multikicker:1 U\nK:etbCounter:P1P1:X:CheckSVar$ X\nSVar:X:Count$TimesKicked/Twice",
    );
    assert_eq!(
        double_kicked.enter_modifiers,
        [
            "EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::Times { base: &Amount::TimesKicked, factor: 2 } }"
        ]
    );

    for bad in [
        "Name:BadCmdMana\nTypes:Creature\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | ActivationZone$ Command",
        "Name:BadAnyPlayerCmd\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | Activator$ Player | ActivationZone$ Command",
        "Name:BadGrantCmd\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature.YouCtrl | AddAbility$ G\nSVar:G:AB$ Draw | Cost$ 1 | NumCards$ 1 | ActivationZone$ Command",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn exile_self_from_graveyard_and_refusals() {
    let tomb = read(
        "Name:TombExiler\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 B ExileFromGrave<1/CARDNAME> | NumCards$ 1 | ActivationZone$ Graveyard",
    );
    assert!(
        tomb.abilities[0].contains("zone = ActivationZone::Graveyard")
            && tomb.abilities[0].contains("ExileSelf"),
        "{:?}",
        tomb.abilities
    );

    let ordered = read(
        "Name:TombPayExiler\nManaCost:3 B\nTypes:Creature\nPT:3/3\n\
         A:AB$ GainLife | Cost$ 1 ExileFromGrave<1/CARDNAME/this card> PayLife<2> | LifeAmount$ 3 | ActivationZone$ Graveyard",
    );
    let ab = &ordered.abilities[0];
    let mana_pos = ab.find("\"{1}\"").expect(ab);
    let exile_pos = ab.find("ExileSelf").expect(ab);
    let life_pos = ab.find("PayLife(2)").expect(ab);
    assert!(mana_pos < exile_pos && exile_pos < life_pos, "{ab}");

    let modal = read(
        "Name:GraveModalExiler\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Charm | Cost$ 1 ExileFromGrave<1/CARDNAME> | Choices$ M1,M2 | ActivationZone$ Graveyard\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        modal.abilities[0].contains("modal_activated!")
            && modal.abilities[0].contains("zone = ActivationZone::Graveyard")
            && modal.abilities[0].contains("ExileSelf"),
        "{:?}",
        modal.abilities
    );

    for bad in [
        "Name:BadDoubleSelfGrave\nManaCost:B\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ ExileFromGrave<2/CARDNAME> | NumCards$ 1 | ActivationZone$ Graveyard",
        "Name:BadFieldExileSelf\nManaCost:B\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ ExileFromGrave<1/CARDNAME> | NumCards$ 1",
        "Name:BadHandExileSelf\nManaCost:B\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ ExileFromGrave<1/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn hand_activation_and_cost_reductions() {
    let channeler = read(
        "Name:HandDiscarder\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 G Discard<1/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
    );
    assert!(
        channeler.abilities[0].contains("zone = ActivationZone::Hand")
            && channeler.abilities[0].contains("DiscardSelf"),
        "{:?}",
        channeler.abilities
    );

    let nick_channeler = read(
        "Name:NickHandDiscarder\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         A:AB$ GainLife | Cost$ Discard<1/NICKNAME/this card> PayLife<2> | LifeAmount$ 3 | ActivationZone$ Hand",
    );
    let nick_ab = &nick_channeler.abilities[0];
    let discard_pos = nick_ab.find("DiscardSelf").expect(nick_ab);
    let pay_life_pos = nick_ab.find("PayLife(2)").expect(nick_ab);
    assert!(
        nick_ab.contains("zone = ActivationZone::Hand") && discard_pos < pay_life_pos,
        "{nick_ab}"
    );

    let hand_exiler = read(
        "Name:HandExiler\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 U ExileFromHand<1/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
    );
    assert!(
        hand_exiler.abilities[0].contains("zone = ActivationZone::Hand")
            && hand_exiler.abilities[0].contains("ExileSelf"),
        "{:?}",
        hand_exiler.abilities
    );

    let channel_reduce = read(
        "Name:ChannelReducer\nTypes:Legendary Land\n\
         A:AB$ Draw | Cost$ 2 G Discard<1/CARDNAME> | ReduceCost$ X | NumCards$ 1 | ActivationZone$ Hand\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
    );
    assert!(
        channel_reduce.abilities[0].contains("zone = ActivationZone::Hand")
            && channel_reduce.abilities[0].contains("DiscardSelf")
            && channel_reduce.abilities[0].contains("CostReduction::PerCount"),
        "{:?}",
        channel_reduce.abilities
    );

    let modal_reduce = read(
        "Name:ModalHandReducer\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Charm | Cost$ 3 Discard<1/CARDNAME> | ReduceCost$ Count$Valid Creature.YouCtrl | Choices$ M1,M2 | ActivationZone$ Hand\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        modal_reduce.abilities[0].contains("modal_activated!")
            && modal_reduce.abilities[0].contains("zone = ActivationZone::Hand")
            && modal_reduce.abilities[0].contains("CostReduction::PerCount"),
        "{:?}",
        modal_reduce.abilities
    );

    let cond_reduce = read(
        "Name:TurnReducer\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ GainLife | Cost$ 3 | ReduceCost$ X | LifeAmount$ 2\n\
         SVar:X:Count$Compare Y GE1.2.0\n\
         SVar:Y:PlayerCountPropertyYou$HasPropertyActive",
    );
    assert!(
        cond_reduce.abilities[0]
            .contains("CostReduction::If { condition: Condition::YourTurn, amount: 2 }"),
        "{:?}",
        cond_reduce.abilities
    );

    let direct_turn_reduce = read(
        "Name:DirectTurnReducer\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ GainLife | Cost$ 3 | ReduceCost$ PlayerCountPropertyYou$HasPropertyActive | LifeAmount$ 2",
    );
    assert!(
        direct_turn_reduce.abilities[0]
            .contains("CostReduction::If { condition: Condition::YourTurn, amount: 1 }"),
        "{:?}",
        direct_turn_reduce.abilities
    );

    for bad in [
        "Name:BadFieldSelfDiscard\nManaCost:1 G\nTypes:Creature\nPT:2/2\nA:AB$ Draw | Cost$ Discard<1/CARDNAME> | NumCards$ 1",
        "Name:BadCommandMana\nManaCost:1 G\nTypes:Creature\nPT:2/2\nA:AB$ Mana | Cost$ Sac<1/Food> | Produced$ G | ActivationZone$ Command",
        "Name:BadHandPlayer\nManaCost:1 G\nTypes:Creature\nPT:2/2\nA:AB$ Draw | Cost$ Discard<1/CARDNAME> | NumCards$ 1 | Activator$ Player | ActivationZone$ Hand",
        "Name:BadHandReveal\nManaCost:1 U\nTypes:Creature\nPT:2/2\nA:AB$ Draw | Cost$ 1 U Reveal<1/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
        "Name:BadDoubleHandDiscard\nManaCost:1 G\nTypes:Creature\nPT:2/2\nA:AB$ Draw | Cost$ Discard<2/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
        "Name:BadDoubleHandExile\nManaCost:1 U\nTypes:Creature\nPT:2/2\nA:AB$ Draw | Cost$ ExileFromHand<2/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
        "Name:BadSpellReduce\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | ReduceCost$ 1",
        "Name:BadReduceAmt\nManaCost:2 W\nTypes:Creature\nPT:2/2\nA:AB$ GainLife | Cost$ 3 | ReduceAmount$ 1 | LifeAmount$ 2",
        "Name:BadMissingReduceSvar\nManaCost:2 W\nTypes:Creature\nPT:2/2\nA:AB$ GainLife | Cost$ 3 | ReduceCost$ Missing | LifeAmount$ 2",
        "Name:BadCardPowerHand\nManaCost:2 W\nTypes:Creature\nPT:2/2\nA:AB$ GainLife | Cost$ 3 Discard<1/CARDNAME> | ReduceCost$ X | LifeAmount$ 2 | ActivationZone$ Hand\nSVar:X:Count$CardPower",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn etb_counter_x_kicker_and_inline_count() {
    let kicked_x = read(
        "Name:XKickerBehemoth\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         K:Kicker:X\nK:etbCounter:P1P1:X:CheckSVar$ WasKicked\n\
         SVar:X:Count$xPaid\nSVar:WasKicked:Count$Kicked.1.0",
    );
    assert_eq!(
        kicked_x.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::X }"]
    );

    let inline_count = read(
        "Name:InlineCounterScout\nManaCost:2 G\nTypes:Creature\nPT:1/1\n\
         K:etbCounter:P1P1:Count$Valid Creature.YouCtrl:no condition",
    );
    assert!(
        inline_count.enter_modifiers.len() == 1
            && inline_count.enter_modifiers[0].contains("WithCounters")
            && inline_count.enter_modifiers[0].contains("CounterKind::P1P1")
            && inline_count.enter_modifiers[0].contains("YOUR_CREATURE"),
        "{:?}",
        inline_count.enter_modifiers
    );

    assert!(refused(
        "Name:BadXKickerOnXCost\nManaCost:X G\nTypes:Creature\nPT:0/0\n\
         K:Kicker:X\nK:etbCounter:P1P1:X:CheckSVar$ WasKicked\n\
         SVar:X:Count$xPaid\nSVar:WasKicked:Count$Kicked.1.0"
    ));
}

#[test]
fn ninjutsu_commander_ninjutsu_and_exert_nickname() {
    let hand_ninja =
        read("Name:ShadowCourier\nManaCost:3 U\nTypes:Creature Ninja\nPT:2/2\nK:Ninjutsu:1 U");
    assert_eq!(
        hand_ninja.abilities,
        [
            "activated!(cost!(\"{1}{U}\", ReturnUnblockedAttacker), &[Effect::PutNinjutsuOntoBattlefield], zone = ActivationZone::Hand, condition = Some(Condition::InCombatAfterBlockers))"
        ]
    );

    let hybrid_ninja =
        read("Name:DuskAgent\nManaCost:U B\nTypes:Creature Ninja\nPT:2/1\nK:Ninjutsu:UB");
    assert_eq!(
        hybrid_ninja.abilities,
        [
            "activated!(cost!(\"{U/B}\", ReturnUnblockedAttacker), &[Effect::PutNinjutsuOntoBattlefield], zone = ActivationZone::Hand, condition = Some(Condition::InCombatAfterBlockers))"
        ]
    );

    let cmd_ninja = read(
        "Name:ClanLeader\nManaCost:1 U B\nTypes:Legendary Creature Ninja\nPT:1/3\nK:Ninjutsu:U B:Commander",
    );
    assert_eq!(
        cmd_ninja.abilities,
        [
            "activated!(cost!(\"{U}{B}\", ReturnUnblockedAttacker), &[Effect::PutNinjutsuOntoBattlefield], zone = ActivationZone::HandOrCommand, condition = Some(Condition::InCombatAfterBlockers))"
        ]
    );

    let walker_ninja = read(
        "Name:MaskWalker\nManaCost:2 U B\nTypes:Legendary Planeswalker Mask\nLoyalty:4\nK:Ninjutsu:1 U B",
    );
    assert!(walker_ninja.abilities[0].contains("Effect::PutNinjutsuOntoBattlefield"));

    let exert_nick = read(
        "Name:BoldChampion\nManaCost:W\nTypes:Creature\nPT:2/1\n\
         A:AB$ Draw | Cost$ W T Exert<1/NICKNAME> | NumCards$ 1",
    );
    assert!(
        exert_nick.abilities[0].contains("cost!(\"{W}\", TapSelf, ExertSelf)"),
        "{:?}",
        exert_nick.abilities
    );

    let flavor_equip =
        read("Name:BannerBlade\nManaCost:2\nTypes:Artifact Equipment\nK:Equip:3:Flavor Battle Cry");
    assert_eq!(flavor_equip.abilities, ["equip!(cost!(\"{3}\"))"]);

    let once_equip = read(
        "Name:BucklerGear\nManaCost:1\nTypes:Artifact Equipment\n\
         K:Equip:0:::ActivationLimit$ 1:Activate only once each turn",
    );
    assert!(
        once_equip.abilities[0].contains("limit = ActivationLimit::PerTurn(1)")
            && once_equip.abilities[0].contains("Effect::AttachSelf"),
        "{:?}",
        once_equip.abilities
    );

    for bad in [
        "Name:BadSpellNinja\nManaCost:U\nTypes:Instant\nK:Ninjutsu:U\nA:SP$ Draw | NumCards$ 1",
        "Name:BadEmptyNinja\nManaCost:U\nTypes:Creature\nPT:1/1\nK:Ninjutsu:",
        "Name:BadExtraNinja\nManaCost:U\nTypes:Creature\nPT:1/1\nK:Ninjutsu:1 U:Other",
        "Name:BadTapNinja\nManaCost:U\nTypes:Creature\nPT:1/1\nK:Ninjutsu:1 T",
        "Name:BadHandExert\nManaCost:W\nTypes:Creature\nPT:2/1\nA:AB$ Draw | Cost$ Exert<1/CARDNAME> | NumCards$ 1 | ActivationZone$ Hand",
        "Name:BadGraveExert\nManaCost:W\nTypes:Creature\nPT:2/1\nA:AB$ Draw | Cost$ Exert<1/NICKNAME> | NumCards$ 1 | ActivationZone$ Graveyard",
        "Name:BadDoubleExert\nManaCost:W\nTypes:Creature\nPT:2/1\nA:AB$ Draw | Cost$ Exert<2/NICKNAME> | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn activation_phases_turns_and_named_conditions() {
    let before_atk = read(
        "Name:ScoutCaptain\nManaCost:1 W\nTypes:Creature\nPT:1/1\n\
         A:AB$ Tap | Cost$ T | ValidTgts$ Creature | PlayerTurn$ True | ActivationPhases$ Upkeep->BeginCombat | ActivationFirstCombat$ True",
    );
    assert!(
        before_atk.abilities[0]
            .contains("Condition::All(&[Condition::YourTurn, Condition::BeforeStep(StepKind::DeclareAttackers)])"),
        "{:?}",
        before_atk.abilities
    );

    let after_blk = read(
        "Name:BlockSneak\nManaCost:2 W\nTypes:Creature\nPT:2/3\n\
         A:AB$ Untap | Cost$ T | ValidTgts$ Creature | ActivationPhases$ Declare Blockers->EndCombat | ActivationAfterBlockers$ True",
    );
    assert!(
        after_blk.abilities[0].contains("Condition::InCombatAfterBlockers"),
        "{:?}",
        after_blk.abilities
    );

    let any_upkeep = read(
        "Name:UpkeepSage\nTypes:Land\nA:AB$ Draw | Cost$ T | NumCards$ 1 | ActivationPhases$ Upkeep",
    );
    assert!(
        any_upkeep.abilities[0].contains("Condition::DuringStep(StepKind::Upkeep)"),
        "{:?}",
        any_upkeep.abilities
    );

    let opp_draw = read(
        "Name:OppWatcher\nManaCost:2 U\nTypes:Creature\nPT:1/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | OpponentTurn$ True | ActivationPhases$ Upkeep->Main1 | ActivationFirstCombat$ True",
    );
    assert!(
        opp_draw.abilities[0]
            .contains("Condition::All(&[Condition::OpponentsTurn, Condition::BeforeStep(StepKind::CombatBegin)])"),
        "{:?}",
        opp_draw.abilities
    );

    for (flag, expected) in [
        ("Delirium", "Condition::GraveyardTypesAtLeast(4)"),
        (
            "Metalcraft",
            "Condition::ControlCount(&Filter::ARTIFACT, 3)",
        ),
        ("Hellbent", "Condition::HandSizeAtMost(0)"),
        ("Blessing", "Condition::CitysBlessing"),
    ] {
        let body = read(&format!(
            "Name:CondArtifact\nManaCost:1\nTypes:Artifact\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1 | Activation$ {flag}"
        ));
        assert!(
            body.abilities[0].contains(expected),
            "{flag}: {:?}",
            body.abilities
        );
    }

    for bad in [
        "Name:BadNoFirstCombat\nManaCost:1 W\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ T | NumCards$ 1 | ActivationPhases$ Upkeep->BeginCombat",
        "Name:BadBothTurns\nManaCost:1 W\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ T | NumCards$ 1 | PlayerTurn$ True | OpponentTurn$ True",
        "Name:BadSolvedAct\nManaCost:1 W\nTypes:Enchantment\nA:AB$ Draw | Cost$ T | NumCards$ 1 | Activation$ Solved",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave11a_squad_offspring_and_escape() {
    let mana_squad =
        read("Name:SquadInfantry\nManaCost:2 W\nTypes:Creature Soldier\nPT:2/2\nK:Squad:1 W");
    assert_eq!(mana_squad.squad.as_deref(), Some("Some(cost!(\"{1}{W}\"))"));

    let ench_squad = read("Name:SquadBanner\nManaCost:2 W\nTypes:Enchantment\nK:Squad:2");
    assert_eq!(ench_squad.squad.as_deref(), Some("Some(cost!(\"{2}\"))"));

    let discard_squad = read(
        "Name:SquadRaider\nManaCost:2 R\nTypes:Creature Warrior\nPT:2/1\nK:Squad:1 Discard<1/Card>",
    );
    assert_eq!(
        discard_squad.squad.as_deref(),
        Some("Some(cost!(\"{1}\", Discard(&Filter::Any)))")
    );

    let offspring = read(
        "Name:OffspringMage\nManaCost:1 R\nTypes:Creature Wizard\nPT:1/2\nK:Offspring:1 R PayLife<2>",
    );
    assert_eq!(
        offspring.offspring.as_deref(),
        Some("Some(cost!(\"{1}{R}\", PayLife(2)))")
    );

    let esc_creature = read(
        "Name:EscapeStalker\nManaCost:1 B\nTypes:Creature Shade\nPT:2/2\n\
         K:Escape:2 B ExileFromGrave<3/Card.Other/other cards>",
    );
    assert_eq!(
        esc_creature.escape.as_deref(),
        Some("Some(Escape { cost: mana!(\"{2}{B}\"), exile: 3 })")
    );

    let esc_spell = read(
        "Name:EscapeBolt\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2\n\
         K:Escape:2 R ExileFromGrave<2/Card.Other>",
    );
    assert_eq!(
        esc_spell.escape.as_deref(),
        Some("Some(Escape { cost: mana!(\"{2}{R}\"), exile: 2 })")
    );

    for bad in [
        "Name:BadSquadSpell\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nK:Squad:2",
        "Name:BadOffspringLand\nManaCost:no cost\nTypes:Land\nK:Offspring:2",
        "Name:BadDupSquad\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Squad:1\nK:Squad:2",
        "Name:BadDupOffspring\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Offspring:1\nK:Offspring:2",
        "Name:BadSquadX\nManaCost:1 U\nTypes:Creature\nPT:1/1\nK:Squad:X U\nSVar:X:Count$xPaid",
        "Name:BadSquadExile4\nManaCost:1 B\nTypes:Creature\nPT:1/1\nK:Squad:ExileFromGrave<4/Card>",
        "Name:BadEscapeX\nManaCost:1 G\nTypes:Creature\nPT:2/2\nK:Escape:X G ExileFromGrave<3/Card.Other>\nSVar:X:Count$xPaid",
        "Name:BadEscapeExileLand\nManaCost:2 R\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nK:Escape:2 R Exile<1/Land>",
        "Name:BadEscapeFilter\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Escape:2 W ExileFromGrave<2/Card.Other+Instant>",
        "Name:BadEscapeLand\nManaCost:no cost\nTypes:Land\nK:Escape:2 ExileFromGrave<2/Card.Other>",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave11a_fuse_and_disturb() {
    let fuse = read(
        "Name:Zap\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2\n\
         K:Fuse\nAlternateMode:Split\nALTERNATE\n\
         Name:Recall\nManaCost:1 U\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1",
    );
    assert!(fuse.fuse);
    assert!(!fuse.back.as_ref().expect("split back").fuse);

    let disturb = read(
        "Name:GraveWatcher\nManaCost:1 U\nTypes:Creature Human\nPT:1/1\n\
         K:Disturb:2 U\nAlternateMode:DoubleFaced\nALTERNATE\n\
         Name:SpectralWatcher\nColors:blue\nTypes:Creature Spirit\nPT:2/2\nK:Flying\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Graveyard | ReplaceWith$ ExileSelf\n\
         SVar:ExileSelf:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
    );
    assert_eq!(disturb.disturb.as_deref(), Some("{2}{U}"));
    assert_eq!(
        disturb.back.as_ref().expect("dfc back").disturb.as_deref(),
        Some("{2}{U}")
    );

    for bad in [
        "Name:SoloFuse\nManaCost:R\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nK:Fuse",
        "Name:ModalFuse\nManaCost:R\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nK:Fuse\nAlternateMode:Modal\nALTERNATE\nName:Back\nManaCost:U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1",
        "Name:AftermathFuse\nManaCost:R\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nK:Fuse\nAlternateMode:Split\nALTERNATE\nName:Back\nManaCost:U\nTypes:Sorcery\nK:Aftermath\nA:SP$ Draw | NumCards$ 1",
        "Name:CreatureFuse\nManaCost:R\nTypes:Instant\nA:SP$ Draw | NumCards$ 1\nK:Fuse\nAlternateMode:Split\nALTERNATE\nName:Back\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2",
        "Name:SoloDisturb\nManaCost:1 U\nTypes:Creature\nPT:1/1\nK:Disturb:2 U",
        "Name:NoExileDisturb\nManaCost:1 U\nTypes:Creature\nPT:1/1\nK:Disturb:2 U\nAlternateMode:DoubleFaced\nALTERNATE\nName:Back\nColors:blue\nTypes:Creature Spirit\nPT:2/2\nK:Flying",
        "Name:NonManaDisturb\nManaCost:1 U\nTypes:Creature\nPT:1/1\nK:Disturb:1 U PayLife<2>\nAlternateMode:DoubleFaced\nALTERNATE\nName:Back\nColors:blue\nTypes:Creature Spirit\nPT:2/2\nR:Event$ Moved | ValidCard$ Card.Self | Destination$ Graveyard | ReplaceWith$ ExileSelf\nSVar:ExileSelf:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave11a_graveyard_presents_eq1_and_combat_windows() {
    let descend = read(
        "Name:DescendSpy\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Permanent.YouOwn | PresentZone$ Graveyard | PresentCompare$ GE4",
    );
    assert!(
        descend.statics.contains("TypeSet::ARTIFACT.union(TypeSet::BATTLE)")
            && descend.abilities[0].contains(
                "AmountAtLeast(Amount::CountOf { filter: &CHECK1, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, 4)"
            ),
        "{}\n{:?}",
        descend.statics,
        descend.abilities
    );

    let above = read(
        "Name:GraveClimber\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Graveyard | IsPresent$ Creature.YouOwn+Above | PresentZone$ Graveyard | PresentCompare$ GE3 | Execute$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Defined$ Self | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        above.abilities[0].contains("Condition::GraveyardCardsAbove(&Filter::CREATURE, 3)"),
        "{:?}",
        above.abilities
    );

    let empty_gy = read(
        "Name:EmptyVault\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Card.YouOwn | PresentZone$ Graveyard | PresentCompare$ EQ0",
    );
    assert!(
        empty_gy.abilities[0].contains("Condition::Not(&Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, 1))"),
        "{:?}",
        empty_gy.abilities
    );

    let lone = read(
        "Name:LoneTitan\nManaCost:3 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | IsPresent$ Creature.YouCtrl | PresentCompare$ EQ1 | Execute$ Gain\n\
         SVar:Gain:DB$ GainLife | LifeAmount$ 2",
    );
    assert!(
        lone.abilities[0].contains("Condition::All(&[Condition::ControlCount(&Filter::YOUR_CREATURE, 1), Condition::ControlCountAtMost(&Filter::YOUR_CREATURE, 1)])"),
        "{:?}",
        lone.abilities
    );

    let solo_artifact = read(
        "Name:SingleRelic\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Artifact | PresentCompare$ EQ1",
    );
    assert!(
        solo_artifact.abilities[0].contains("Condition::All(&[Condition::BattlefieldCount(&Filter::ARTIFACT, 1), Condition::BattlefieldCountAtMost(&Filter::ARTIFACT, 1)])"),
        "{:?}",
        solo_artifact.abilities
    );

    let exact_one_self_ctr = read(
        "Name:ChargeGate\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Card.Self+counters_GE2_CHARGE | PresentCompare$ EQ1",
    );
    assert!(
        exact_one_self_ctr.abilities[0]
            .contains("Condition::CountersOnSelf(CounterKind::Charge, 2)"),
        "{:?}",
        exact_one_self_ctr.abilities
    );

    let combat_windows = read(
        "Name:CombatSentry\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | ActivationPhases$ Upkeep->Declare Attackers | ActivationFirstCombat$ True\n\
         A:AB$ GainLife | Cost$ T | LifeAmount$ 1 | ActivationPhases$ Upkeep->Declare Blockers | ActivationFirstCombat$ True",
    );
    assert!(
        combat_windows.abilities[0].contains("Condition::BeforeStep(StepKind::DeclareBlockers)")
            && combat_windows.abilities[1]
                .contains("Condition::BeforeStep(StepKind::CombatDamage)"),
        "{:?}",
        combat_windows.abilities
    );

    for bad in [
        "Name:DirectlyAbove\nManaCost:B\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Creature.YouOwn+DirectlyAbove | PresentZone$ Graveyard",
        "Name:NoYouOwnGy\nManaCost:B\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Creature | PresentZone$ Graveyard",
        "Name:MixedAbove\nManaCost:B\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Creature.YouOwn+Above,Artifact.YouOwn | PresentZone$ Graveyard",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave12a_keywords_and_attacks_alone() {
    for (kw, expected) in [
        ("Wither", "KeywordSet::WITHER"),
        ("Horsemanship", "KeywordSet::HORSEMANSHIP"),
        ("Skulk", "KeywordSet::SKULK"),
        ("Enlist", "KeywordSet::ENLIST"),
    ] {
        let body = read(&format!(
            "Name:TestCreature\nManaCost:2\nTypes:Creature\nPT:2/2\nK:{kw}"
        ));
        assert_eq!(body.keywords, vec![expected], "{kw}");
        assert!(body.abilities.is_empty(), "{kw}: {:?}", body.abilities);
    }

    let dup_enlist =
        read("Name:DoubleEnlist\nManaCost:2 W\nTypes:Creature\nPT:2/2\nK:Enlist\nK:Enlist");
    assert_eq!(dup_enlist.keywords, vec!["KeywordSet::ENLIST"]);
    assert_eq!(
        dup_enlist.abilities,
        vec!["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::ENLIST))"]
    );

    let solo = read(
        "Name:LoneCharger\nManaCost:1 R\nTypes:Creature\nPT:3/2\nK:CARDNAME can only attack alone.",
    );
    assert_eq!(
        solo.abilities,
        vec!["static_ability!(Filter::This, Modifier::AttacksAloneOnly)"]
    );
}

#[test]
fn wave12a_behold_mutate_and_commander_text() {
    let behold = read(
        "Name:BeholdScout\nManaCost:2 W\nTypes:Creature Goblin\nPT:2/2\n\
         K:AlternateAdditionalCost:Behold<1/Goblin>:2",
    );
    assert_eq!(
        behold.behold_or_pay.as_deref(),
        Some("Some(BeholdOrPay { filter: &BEHOLD1, cost: mana!(\"{2}\") })")
    );
    assert!(
        behold.statics.contains("BEHOLD1") && behold.statics.contains("GOBLIN"),
        "{}",
        behold.statics
    );

    let mutator = read("Name:MutateBeast\nManaCost:3 G\nTypes:Creature\nPT:3/3\nK:Mutate:1 G");
    assert_eq!(mutator.mutate.as_deref(), Some("Some(mana!(\"{1}{G}\"))"));

    for ok in [
        "Name:WalkerText\nManaCost:2 U\nTypes:Legendary Planeswalker\nLoyalty:3\nK:Flying\nText:CARDNAME can be your commander.\nOracle:",
        "Name:WalkerOracle\nManaCost:2 U\nTypes:Legendary Planeswalker\nLoyalty:3\nK:Flying\nText:CARDNAME can be your commander.\nOracle:Flying\\nWalkerOracle can be your commander.",
        "Name:WalkerKw\nManaCost:2 U\nTypes:Legendary Planeswalker\nLoyalty:3\nK:Flying\nK:CARDNAME can be your commander.\nOracle:Flying\\nWalkerKw can be your commander.",
    ] {
        read(ok);
    }

    for bad in [
        "Name:BadCount\nManaCost:1 W\nTypes:Creature\nPT:2/2\nK:AlternateAdditionalCost:Behold<2/Goblin>:2",
        "Name:BadReveal\nManaCost:1 W\nTypes:Creature\nPT:2/2\nK:AlternateAdditionalCost:Reveal<1/Goblin>:2",
        "Name:BadTap\nManaCost:1 W\nTypes:Creature\nPT:2/2\nK:AlternateAdditionalCost:Behold<1/Goblin>:T",
        "Name:BadDup\nManaCost:1 W\nTypes:Creature\nPT:2/2\nK:AlternateAdditionalCost:Behold<1/Goblin>:2\nK:AlternateAdditionalCost:Behold<1/Goblin>:3",
        "Name:BadLand\nTypes:Land\nK:AlternateAdditionalCost:Behold<1/Goblin>:2",
        "Name:MutateSpell\nManaCost:1 G\nTypes:Instant\nK:Mutate:G\nA:SP$ GainLife | LifeAmount$ 2",
        "Name:MutateLand\nTypes:Land\nK:Mutate:G",
        "Name:MutateTap\nManaCost:1 G\nTypes:Creature\nPT:2/2\nK:Mutate:T",
        "Name:MutateDup\nManaCost:1 G\nTypes:Creature\nPT:2/2\nK:Mutate:G\nK:Mutate:1 G",
        "Name:BadCommander\nManaCost:2 U\nTypes:Legendary Planeswalker\nLoyalty:3\nK:Flying\nText:CARDNAME can be your commander.\nOracle:+1: Draw a card.",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }

    let prepared = parse(
        "Name:PrepFront\nManaCost:1 U\nTypes:Creature Wizard\nPT:2/1\n\
         K:ETBReplacement:Other:DBPrep\nSVar:DBPrep:DB$ AlterAttribute | Attributes$ Prepared\n\
         AlternateMode:Prepare\nALTERNATE\nName:PrepBack\nCopyFaceFrom:PrepBack",
    );
    assert_eq!(
        refusal_reason(&prepared, &cats(), None).as_deref(),
        Some("unresolved `CopyFaceFrom` face")
    );
}

#[test]
fn wave12a_opponents_life_lost_svars() {
    let opps = |cmp: &str| {
        read(&format!(
            "Name:BloodSeer\nManaCost:1 B\nTypes:Creature\nPT:2/1\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ X | SVarCompare$ {cmp}\n\
             SVar:X:Count$LifeOppsLostThisTurn"
        ))
    };
    assert!(
        opps("EQ0").abilities[0].contains("Condition::Not(&Condition::OpponentLostLifeThisTurn)")
    );
    assert!(opps("GE1").abilities[0].contains("Condition::OpponentLostLifeThisTurn"));
    assert!(
        opps("GE3").abilities[0]
            .contains("Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, 3)")
    );
    let eq1_opps = opps("EQ1");
    assert!(
        eq1_opps.abilities[0].contains(
            "Condition::All(&[Condition::OpponentLostLifeThisTurn, Condition::Not(&CHECK1"
        ),
        "{}",
        eq1_opps.abilities[0]
    );
    assert!(
        eq1_opps
            .statics
            .contains("Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, 2)")
    );
    let le2_opps = opps("LE2");
    assert!(
        le2_opps.abilities[0].contains("Condition::Not(&CHECK1"),
        "{}",
        le2_opps.abilities[0]
    );
    assert!(
        le2_opps
            .statics
            .contains("Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, 3)")
    );

    let limit_one = |cmp: &str| {
        format!(
            "Name:LimitSeer\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ X | SVarCompare$ {cmp}\n\
             SVar:X:Count$LifeOppsLostThisTurn/LimitMax.1"
        )
    };
    assert!(read(&limit_one("GE1")).abilities[0].contains("Condition::OpponentLostLifeThisTurn"));
    assert!(
        read(&limit_one("EQ0")).abilities[0]
            .contains("Condition::Not(&Condition::OpponentLostLifeThisTurn)")
    );
    assert!(refused(&limit_one("GE2")));
}

#[test]
fn wave12a_bounded_amount_and_count_svars() {
    let entered = |cmp: &str| {
        read(&format!(
            "Name:GateWatch\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
             A:AB$ GainLife | Cost$ T | LifeAmount$ 1 | CheckSVar$ E | SVarCompare$ {cmp}\n\
             SVar:E:Count$ThisTurnEntered_Battlefield_Creature.YouCtrl"
        ))
    };
    assert!(
        entered("EQ0")
            .statics
            .contains("Amount::CreaturesEnteredUnderYourControlThisTurn, 1")
    );
    assert!(
        entered("LE2")
            .statics
            .contains("Amount::CreaturesEnteredUnderYourControlThisTurn, 3")
    );
    assert!(
        entered("EQ1").abilities[0].contains(
            "Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, 1)"
        )
    );

    let gained = |cmp: &str| {
        read(&format!(
            "Name:VitalSeer\nManaCost:1 W\nTypes:Creature\nPT:1/2\n\
             A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ G | SVarCompare$ {cmp}\n\
             SVar:G:Count$LifeYouGainedThisTurn"
        ))
    };
    assert!(
        gained("EQ0")
            .statics
            .contains("Amount::LifeGainedThisTurn(PlayerRel::You), 1")
    );
    assert!(
        gained("EQ2").abilities[0]
            .contains("Condition::AmountAtLeast(Amount::LifeGainedThisTurn(PlayerRel::You), 2)")
    );

    let exact_life = read(
        "Name:FiveLife\nManaCost:1 W\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ L | SVarCompare$ EQ5\n\
         SVar:L:Count$YourLifeTotal",
    );
    assert!(exact_life.abilities[0].contains(
        "Condition::All(&[Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), 5), Condition::LifeAtMost(5)])"
    ));

    let exact_ctrl = read(
        "Name:PairCtrl\nManaCost:2 G\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ C | SVarCompare$ EQ2\n\
         SVar:C:Count$Valid Creature.YouCtrl",
    );
    assert!(
        exact_ctrl.abilities[0].contains("Condition::ControlCount(&Filter::YOUR_CREATURE, 2)")
            && exact_ctrl.abilities[0]
                .contains("Condition::ControlCountAtMost(&Filter::YOUR_CREATURE, 2)"),
        "{}",
        exact_ctrl.abilities[0]
    );

    let exact_bf = read(
        "Name:PairAll\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ B | SVarCompare$ EQ2\n\
         SVar:B:Count$Valid Creature",
    );
    assert!(
        exact_bf.abilities[0].contains("Condition::BattlefieldCount(&Filter::CREATURE, 2)")
            && exact_bf.abilities[0]
                .contains("Condition::BattlefieldCountAtMost(&Filter::CREATURE, 2)"),
        "{}",
        exact_bf.abilities[0]
    );
}

#[test]
fn wave13a_plot_and_casualty_keywords() {
    let plotted = read(
        "Name:PlotMage\nManaCost:2 U\nTypes:Creature Wizard\nPT:2/3\n\
         K:Plot:1 U\nK:Flying",
    );
    assert_eq!(plotted.plot.as_deref(), Some("Some(mana!(\"{1}{U}\"))"));

    let cas = read(
        "Name:CasBolt\nManaCost:1 R\nTypes:Instant\n\
         K:Casualty:2\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2",
    );
    assert_eq!(cas.casualty.as_deref(), Some("Some(2)"));

    for bad in [
        "Name:PlotLand\nTypes:Land\nK:Plot:1",
        "Name:PlotDup\nManaCost:1 R\nTypes:Sorcery\nK:Plot:R\nK:Plot:1 R\nA:SP$ Draw | NumCards$ 1",
        "Name:PlotPayLife\nManaCost:1 B\nTypes:Sorcery\nK:Plot:1 B PayLife<2>\nA:SP$ Draw | NumCards$ 1",
        "Name:CasZero\nManaCost:R\nTypes:Instant\nK:Casualty:0\nA:SP$ Draw | NumCards$ 1",
        "Name:CasX\nManaCost:1 B R\nTypes:Planeswalker\nLoyalty:3\nK:Casualty:X:NonLegendary$ True",
        "Name:CasDup\nManaCost:R\nTypes:Instant\nK:Casualty:1\nK:Casualty:2\nA:SP$ Draw | NumCards$ 1",
        "Name:CasLand\nTypes:Land\nK:Casualty:1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave13a_backup_variants_and_refusals() {
    let kw_only = read(
        "Name:SkyBacker\nManaCost:3 W\nTypes:Creature\nPT:3/3\n\
         K:Flying\nK:First Strike\nK:Backup:2:G\nSVar:G:DB$ Pump | KW$ Flying & First Strike",
    );
    assert_eq!(
        kw_only.abilities,
        vec![
            "triggered!(Trigger::ETB, &[Effect::Backup { count: 2, keywords: KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE), grant_abilities: &[] }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let full = read(
        "Name:PackLeader\nManaCost:4 G\nTypes:Creature\nPT:3/3\n\
         K:Trample\nK:Backup:1:G\nK:Backup:1:G\n\
         SVar:G:DB$ Animate | Keywords$ Trample | Abilities$ Ab | Triggers$ Tr | staticAbilities$ St | sVars$ AE\n\
         SVar:Ab:AB$ Draw | Cost$ 1 Sac<1/CARDNAME> | NumCards$ 1\n\
         SVar:Tr:Mode$ Attacks | ValidCard$ Card.Self | Execute$ ExGain\n\
         SVar:ExGain:DB$ GainLife | LifeAmount$ 1\n\
         SVar:St:Mode$ CantBlockBy | ValidAttacker$ Card.Self | ValidBlocker$ Creature.powerLE2\n\
         SVar:AE:SVar:HasAttackEffect:TRUE",
    );
    assert_eq!(full.abilities.len(), 2);
    assert!(full.abilities[0].contains("grant_abilities: &BACKUP_GRANTED2"));
    assert_eq!(full.abilities[0], full.abilities[1]);
    assert_eq!(
        full.statics.matches("static BACKUP_GRANTED2:").count(),
        1,
        "{}",
        full.statics
    );
    assert!(full.statics.contains("[AbilityDef; 3]"), "{}", full.statics);
    assert!(full.statics.contains("PowerAtMost(2)"), "{}", full.statics);

    for bad in [
        "Name:BadBackupSpell\nManaCost:1 W\nTypes:Sorcery\nK:Backup:1:G\nSVar:G:DB$ Pump | KW$ Flying",
        "Name:BadBackupZero\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Backup:0:G\nSVar:G:DB$ Pump | KW$ Flying",
        "Name:BadBackupMiss\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Backup:1:Missing",
        "Name:BadBackupDur\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Backup:1:G\nSVar:G:DB$ Animate | Keywords$ Flying | Duration$ Permanent",
        "Name:BadBackupSVar\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Backup:1:G\nSVar:G:DB$ Animate | Keywords$ Flying | sVars$ Foo\nSVar:Foo:Count$YourLifeTotal",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave13a_pruned_statics_non_mana_counter_x_and_atoms() {
    let each_opp = read(
        "Name:SweepOpp\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Artifact.OppCtrl,Enchantment.OppCtrl | \
         TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        each_opp.abilities[0].contains("ObjectOfEachOpponent(&Filter::ARTIFACT_OR_ENCHANTMENT)")
    );
    assert!(each_opp.statics.is_empty(), "{}", each_opp.statics);

    let each_opp_nonconst = read(
        "Name:SweepStrong\nManaCost:3 B G\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.powerGE3+OppCtrl | \
         TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        each_opp_nonconst.abilities[0].contains("ObjectOfEachOpponent(&TARGET2)"),
        "{:?}",
        each_opp_nonconst.abilities
    );
    assert!(
        !each_opp_nonconst.statics.contains("static TARGET1:"),
        "{}",
        each_opp_nonconst.statics
    );
    assert!(
        each_opp_nonconst.statics.contains("static TARGET2:"),
        "{}",
        each_opp_nonconst.statics
    );

    let mana_x = read(
        "Name:ChargeToy\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T SubCounter<X/TIME> | Produced$ W | Amount$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(mana_x.abilities[0].contains("RemoveCounterSelfX { kind: CounterKind::Time }"));

    let stack_time = read(
        "Name:StackTime\nManaCost:2 W\nTypes:Artifact\n\
         A:AB$ ChangeZoneAll | Cost$ T SubCounter<X/TIME> Exile<1/CARDNAME> | \
         ChangeType$ Artifact.YouOwn | Origin$ Graveyard | Destination$ Battlefield\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(stack_time.abilities.len(), 1);
    assert!(
        stack_time.abilities[0].contains("RemoveCounterSelfX")
            && stack_time.abilities[0].contains("CounterKind::Time")
            && stack_time.abilities[0].contains("ExileSelf"),
        "expected RemoveCounterSelfX(Time) + ExileSelf, got {:?}",
        stack_time.abilities
    );
    for bad in [
        "Name:StackTimeTargeted\nManaCost:2 W\nTypes:Artifact\n\
         A:AB$ Destroy | ValidTgts$ Artifact | Cost$ T SubCounter<X/TIME> Exile<1/CARDNAME>\n\
         SVar:X:Count$xPaid",
        "Name:StackGain\nManaCost:2\nTypes:Artifact\n\
         A:AB$ GainLife | Cost$ SubCounter<X/CHARGE> | LifeAmount$ X\nSVar:X:Count$xPaid",
        "Name:StackCharm\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ SubCounter<X/CHARGE> | Choices$ D,G\n\
         SVar:D:DB$ Draw | NumCards$ 1\nSVar:G:DB$ GainLife | LifeAmount$ 1\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse non-mana counter X: {bad}");
    }

    let max_ctr = read(
        "Name:ChargeGate\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ C | SVarCompare$ LE2\n\
         SVar:C:Count$CardCounters.CHARGE",
    );
    assert!(
        max_ctr
            .statics
            .contains("Condition::AmountAtLeast(Amount::CountersOnSource(CounterKind::Charge), 3)")
    );

    assert_eq!(
        filter("Creature.withHorsemanship"),
        "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::HORSEMANSHIP)])"
    );
    assert_eq!(
        filter("Creature.withoutSkulk"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasKeyword(KeywordSet::SKULK))])"
    );
    assert_eq!(
        filter("Card.withDevoid"),
        "Filter::HasKeyword(KeywordSet::DEVOID)"
    );
    assert_eq!(
        filter("Creature.withPhasing"),
        "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::PHASING)])"
    );
    assert_eq!(
        filter("Creature.withEnlist"),
        "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::ENLIST)])"
    );
}

#[test]
fn wave14a_modular_soulbond_renown_and_refusals() {
    let mod2 =
        read("Name:ModBot\nManaCost:2\nTypes:Artifact Creature Construct\nPT:0/0\nK:Modular:2");
    assert_eq!(mod2.keywords, vec!["KeywordSet::MODULAR"]);
    assert_eq!(mod2.enter_modifiers, vec!["EnterModifier::modular(2)"]);

    let mod_dup = read(
        "Name:TwinMod\nManaCost:4\nTypes:Artifact Creature Construct\nPT:0/0\nK:Modular:1\nK:Modular:3",
    );
    assert_eq!(mod_dup.keywords, vec!["KeywordSet::MODULAR"]);
    assert_eq!(
        mod_dup.enter_modifiers,
        vec!["EnterModifier::modular(1)", "EnterModifier::modular(3)"]
    );
    assert!(mod_dup.abilities.contains(
        &"static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::MODULAR))".to_string()
    ));

    let sb_dup =
        read("Name:BondPair\nManaCost:2 W\nTypes:Creature\nPT:2/2\nK:Soulbond\nK:Soulbond");
    assert_eq!(sb_dup.keywords, vec!["KeywordSet::SOULBOND"]);
    assert!(sb_dup.abilities.contains(
        &"static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::SOULBOND))".to_string()
    ));

    let ren_dup = read("Name:Champ\nManaCost:1 W\nTypes:Creature\nPT:2/1\nK:Renown:1\nK:Renown:3");
    assert_eq!(ren_dup.keywords, vec!["KeywordSet::RENOWN"]);
    assert_eq!(
        ren_dup.abilities,
        vec![
            "triggered!(Trigger::DealsCombatDamageToPlayer(&Filter::This), &[Effect::BecomeRenowned(1)], condition = Some(Condition::Not(&Condition::IsRenowned)))",
            "triggered!(Trigger::DealsCombatDamageToPlayer(&Filter::This), &[Effect::BecomeRenowned(3)], condition = Some(Condition::Not(&Condition::IsRenowned)))",
        ]
    );

    assert_eq!(
        filter("Creature.withModular"),
        "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::MODULAR)])"
    );
    assert_eq!(
        filter("Creature.withSoulbond"),
        "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SOULBOND)])"
    );
    assert_eq!(
        filter("Creature.withoutRenown"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasKeyword(KeywordSet::RENOWN))])"
    );

    for bad in [
        "Name:ModSun\nManaCost:3\nTypes:Artifact Creature\nPT:0/0\nK:Modular:Sunburst",
        "Name:ModBare\nManaCost:1\nTypes:Artifact Creature\nPT:0/0\nK:Modular",
        "Name:ModSpell\nManaCost:1\nTypes:Instant\nK:Modular:1\nA:SP$ Draw | NumCards$ 1",
        "Name:RenZero\nManaCost:W\nTypes:Creature\nPT:1/1\nK:Renown:0",
        "Name:RenBare\nManaCost:W\nTypes:Creature\nPT:1/1\nK:Renown",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave14a_forage_damage_you_and_x_object_costs() {
    let ab = read(
        "Name:Forager\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ GainLife | Cost$ 1 Forage | LifeAmount$ 2\n\
         A:AB$ Draw | Cost$ T DamageYou<3> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ Discard<X/Creature/creature cards> | NumCards$ X\n\
         A:AB$ GainLife | Cost$ ExileFromGrave<X/Card/cards> | LifeAmount$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(ab.abilities[0].contains("cost!(\"{1}\", Forage)"));
    assert!(ab.abilities[1].contains("cost!(TapSelf, DealDamageToYou(3))"));
    assert!(ab.abilities[2].contains("cost!(DiscardX(&Filter::CREATURE))"));
    assert!(ab.abilities[3].contains("cost!(ExileFromGraveyardX(&Filter::Any))"));

    let sp_forage = read(
        "Name:BiteForage\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ 1 B Forage | NumCards$ 2",
    );
    assert_eq!(
        sp_forage.mandatory_additional_costs,
        vec!["CostPart::Forage"]
    );

    let sp_dmg = read(
        "Name:PainDraw\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ B DamageYou<2> | NumCards$ 2",
    );
    assert_eq!(
        sp_dmg.mandatory_additional_costs,
        vec!["CostPart::DealDamageToYou(2)"]
    );

    let sp_x = read(
        "Name:DreamsX\nManaCost:G G\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ G G Discard<X/Card/cards> ExileFromGrave<X/Creature/creatures> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(
        sp_x.mandatory_additional_costs,
        vec![
            "CostPart::DiscardX(&Filter::Any)",
            "CostPart::ExileFromGraveyardX(&Filter::CREATURE)",
        ]
    );

    let trig_forage = read(
        "Name:TrigForage\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ ExF\n\
         SVar:ExF:AB$ Draw | Cost$ Forage | NumCards$ 1",
    );
    assert!(
        trig_forage.abilities[0]
            .contains("PlayerMayPayCostThen { player: PlayerRel::You, cost: &CostPart::Forage")
    );

    let trig_dmg = read(
        "Name:TrigPain\nManaCost:1 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ ExP\n\
         SVar:ExP:AB$ Draw | Cost$ DamageYou<2> | NumCards$ 1",
    );
    assert!(trig_dmg.abilities[0].contains(
        "PlayerMayPayCostThen { player: PlayerRel::You, cost: &CostPart::DealDamageToYou(2)"
    ));

    for bad in [
        "Name:DmgZero\nManaCost:R\nTypes:Artifact\nA:AB$ Draw | Cost$ DamageYou<0> | NumCards$ 1",
        "Name:RandX\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Draw | Cost$ 1 R Discard<X/Random> | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:NoXSVar\nManaCost:1 B\nTypes:Sorcery\nA:SP$ Draw | Cost$ 1 B Discard<X/Card> | NumCards$ 1",
        "Name:OracleXIs\nManaCost:1 B\nTypes:Artifact\nOracle:Discard X cards, where X is twice something.\n\
         A:AB$ Draw | Cost$ Discard<X/Card> | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:FbDiscardX\nManaCost:1 R\nTypes:Sorcery\nK:Flashback:1 R Discard<X/Card>\n\
         A:SP$ Draw | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:EscExileX\nManaCost:1 B\nTypes:Creature\nPT:2/2\nK:Escape:2 B ExileFromGrave<X/Card>\nSVar:X:Count$xPaid",
        "Name:TrigDiscX\nManaCost:1 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ Ex\n\
         SVar:Ex:DB$ Draw | Cost$ Discard<X/Card> | NumCards$ 1\nSVar:X:Count$xPaid",
        "Name:KickForage\nManaCost:1 G\nTypes:Sorcery\nK:Kicker:Forage\nA:SP$ Draw | NumCards$ 1",
        "Name:CtrlX\nManaCost:1 B\nTypes:Sorcery\nA:SP$ Draw | Cost$ 1 B ExileFromGrave<X/Card.YouCtrl> | NumCards$ X\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

fn wave14a_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Germ".into(),
            "Hero".into(),
            "Phyrexian".into(),
            "Rebel".into(),
            "Spirit".into(),
        ],
        artifact: vec!["Equipment".into()],
        enchantment: vec!["Class".into()],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

#[test]
fn wave14a_engines_bushido_soulshift_reconfigure_and_attach_tokens() {
    let eng = read("Name:Racer\nManaCost:2 R\nTypes:Creature\nPT:3/2\nK:Start your engines");
    assert_eq!(eng.abilities, vec!["AbilityDef::StartYourEngines"]);

    let bush = read("Name:Samurai\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Bushido:2\nK:Bushido:1");
    assert_eq!(
        bush.abilities,
        vec![
            "triggered!(Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE), &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(2), toughness: Amount::Fixed(2), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])",
            "triggered!(Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE), &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(1), toughness: Amount::Fixed(1), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])",
        ]
    );

    let full_cats = wave14a_cats();
    let ss = transcode(
        &parse("Name:SpiritGuide\nManaCost:3 G\nTypes:Creature Spirit\nPT:2/2\nK:Soulshift:3"),
        &full_cats,
        None,
    )
    .expect("soulshift");
    assert!(
        ss.abilities[0].contains("Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&TARGET1, PlayerRel::You) }")
            && ss.statics.contains("Filter::HasSubtype(subtypes::creature::SPIRIT)")
            && ss.statics.contains("Filter::CmcAtMost(3)"),
        "{:?}\n{}",
        ss.abilities,
        ss.statics
    );

    let reconf = read(
        "Name:GearBot\nManaCost:2\nTypes:Artifact Creature Equipment\nPT:2/1\nK:Reconfigure:2:PayEnergy<3>",
    );
    assert_eq!(
        reconf.abilities,
        vec![
            "static_ability!(Filter::This, Modifier::RemoveType(TypeSet::CREATURE), condition = Some(Condition::SourceMatches(&Filter::IsAttached)))",
            "activated!(cost!(\"{2}\"), &[Effect::ReconfigureAttach], target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)), timing = ActivationTiming::SorcerySpeed)",
            "activated!(cost!(\"{2}\"), &[Effect::DetachSource], timing = ActivationTiming::SorcerySpeed, condition = Some(Condition::SourceMatches(&Filter::IsAttached)))",
            "activated!(cost!(PayEnergy(3)), &[Effect::ReconfigureAttach], target = Some(TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL)), timing = ActivationTiming::SorcerySpeed)",
            "activated!(cost!(PayEnergy(3)), &[Effect::DetachSource], timing = ActivationTiming::SorcerySpeed, condition = Some(Condition::SourceMatches(&Filter::IsAttached)))",
        ]
    );

    let attach_tokens = TokenLookup::held(&[
        (
            "b_0_0_phyrexian_germ",
            "Name:Phyrexian Germ Token\nColors:black\nTypes:Creature Phyrexian Germ\nPT:0/0",
        ),
        (
            "r_2_2_rebel",
            "Name:Rebel\nColors:red\nTypes:Creature Rebel\nPT:2/2",
        ),
        ("c_1_1_hero", "Name:Hero\nTypes:Creature Hero\nPT:1/1"),
    ]);
    for (kw, stem, want_tok) in [
        (
            "Living Weapon",
            "b_0_0_phyrexian_germ",
            "&generated_tokens::PHYREXIAN_GERM_0_0_BLACK",
        ),
        (
            "For Mirrodin",
            "r_2_2_rebel",
            "&generated_tokens::REBEL_2_2_RED",
        ),
        ("Job select", "c_1_1_hero", "&generated_tokens::HERO_1_1"),
    ] {
        let script = parse(&format!(
            "Name:Blade\nManaCost:2\nTypes:Artifact Equipment\nK:{kw}"
        ));
        let stems = token_stems(&script);
        assert!(stems.iter().any(|s| s == stem), "{kw}: {stems:?}");
        let body = transcode(&script, &full_cats, Some(&attach_tokens)).expect("attach token");
        assert!(
            body.abilities[0].contains(&format!(
                "Effect::CreateTokenThenAttachSelf {{ token: {want_tok} }}"
            )),
            "{kw}: {:?}",
            body.abilities
        );
    }

    for bad in [
        "Name:BushZero\nManaCost:W\nTypes:Creature\nPT:1/1\nK:Bushido:0",
        "Name:ShiftX\nManaCost:2 G\nTypes:Creature\nPT:1/1\nK:Soulshift:X",
        "Name:ReconfEmpty\nManaCost:2\nTypes:Artifact Equipment\nK:Reconfigure:",
        "Name:RetraceLand\nTypes:Land\nK:Retrace",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave14a_class_levels_and_overload() {
    let full_cats = wave14a_cats();
    let class_parsed = parse(
        "Name:StudyHall\nManaCost:1 U\nTypes:Enchantment Class\n\
         S:Mode$ Continuous | Affected$ You | SetMaxHandSize$ Unlimited\n\
         K:Class:2:2 U:AddTrigger$ Lvl2Trig | AddStaticAbility$ L2Pump\n\
         K:Class:3:3 U:AddTrigger$ WatchDraw & Lvl3Trig | AddReplacementEffect$ NoUntap\n\
         SVar:Lvl2Trig:Mode$ ClassLevelGained | ClassLevel$ 2 | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ ExDraw\n\
         SVar:ExDraw:DB$ Draw | NumCards$ 2\n\
         SVar:Lvl3Trig:Mode$ ClassLevelGained | ClassLevel$ 3 | ValidCard$ Card.Self | Execute$ ExDestroy\n\
         SVar:ExDestroy:DB$ Destroy | ValidTgts$ Artifact.YouDontCtrl\n\
         SVar:L2Pump:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1\n\
         SVar:WatchDraw:Mode$ Drawn | ValidCard$ Card.YouCtrl | TriggerZones$ Battlefield | Execute$ ExLife\n\
         SVar:ExLife:DB$ GainLife | LifeAmount$ 1\n\
         SVar:NoUntap:Event$ Untap | ActiveZones$ Battlefield | ValidStepTurnToController$ You | ValidCard$ Creature.YouDontCtrl | Layer$ CantHappen",
    );
    let valid_class = transcode(&class_parsed, &full_cats, None).unwrap_or_else(|| {
        panic!(
            "class card refused: {:?}",
            refusal_reason(&class_parsed, &full_cats, None)
        )
    });
    assert!(
        valid_class
            .abilities
            .iter()
            .any(|a| a.contains("Effect::SetClassLevel(2)")
                && a.contains("ActivationTiming::SorcerySpeed")
                && a.contains("Condition::ClassLevelExactly(1)"))
    );
    assert!(
        valid_class
            .abilities
            .iter()
            .any(|a| a == "triggered!(Trigger::ClassLevelGained(2), &[Effect::draw(2)])")
    );
    assert!(
        valid_class
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::ModifyPT(1, 1)")
                && a.contains("Condition::ClassLevelAtLeast(2)"))
    );
    assert!(
        valid_class
            .abilities
            .iter()
            .any(|a| a.contains("Effect::SetClassLevel(3)")
                && a.contains("ActivationTiming::SorcerySpeed")
                && a.contains("Condition::ClassLevelExactly(2)"))
    );
    assert!(valid_class.abilities.iter().any(|a| {
        a.contains("GrantTriggeredDefinition(&GRANTED_TRIGGER_1)")
            && a.contains("Condition::ClassLevelAtLeast(3)")
    }));
    assert!(valid_class.abilities.iter().any(|a| {
        a.starts_with("triggered!(Trigger::ClassLevelGained(3),")
            && a.contains("targets = Some(TargetReq::one(TargetSpec::Object(")
    }));
    assert!(
        valid_class
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::DoesNotUntap")
                && a.contains("Condition::ClassLevelAtLeast(3)"))
    );

    let ov_destroy = read(
        "Name:ShatterAll\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Destroy | ValidTgts$ Artifact.YouDontCtrl\nK:Overload:4 R",
    );
    assert_eq!(
        ov_destroy.abilities,
        vec![
            "AbilityDef::ModalSpell { choose: ModeCount::ONE, modes: &[mode!(&[Effect::destroy(TargetSpec::Object(&TARGET1))], targets = Some(TargetReq::one(TargetSpec::Object(&TARGET1)))), mode!(&[Effect::destroy_all(&TARGET1)], cost_override = Some(mana!(\"{4}{R}\")))] }"
        ]
    );
    let ov_pump = read(
        "Name:MassBuff\nManaCost:W\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2 | KW$ Vigilance\nK:Overload:4 W",
    );
    assert!(ov_pump.abilities[0].contains("Effect::PumpFilter { filter: &Filter::YOUR_CREATURE, controlled_by: None, power: Amount::Fixed(2), toughness: Amount::Fixed(2), keywords: KeywordSet::VIGILANCE, duration: Duration::UntilEndOfTurn }"));
    assert!(ov_pump.abilities[0].contains("cost_override = Some(mana!(\"{4}{W}\"))"));

    let ov_x = read(
        "Name:SpasmX\nManaCost:X R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature.YouDontCtrl | NumDmg$ X\n\
         K:Overload:X X R R\nSVar:X:Count$xPaid",
    );
    assert!(ov_x.abilities[0].contains("Effect::DealDamageEach { amount: Amount::X"));
    assert!(ov_x.abilities[0].contains("cost_override = Some(mana!(\"{X}{X}{R}{R}\"))"));

    for bad in [
        "Name:NoClassSub\nManaCost:1 U\nTypes:Enchantment\nK:Class:2:1 U:AddStaticAbility$ St\nSVar:St:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1",
        "Name:SkipLvl\nManaCost:1 U\nTypes:Enchantment Class\nK:Class:3:1 U:AddStaticAbility$ St\nSVar:St:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1",
        "Name:WrongLevelTrigger\nManaCost:1 U\nTypes:Enchantment Class\nK:Class:2:1 U:AddTrigger$ Trig\nSVar:Trig:Mode$ ClassLevelGained | ClassLevel$ 3 | ValidCard$ Card.Self | Execute$ ExDraw\nSVar:ExDraw:DB$ Draw | NumCards$ 2",
        "Name:OvNonSpell\nManaCost:2\nTypes:Artifact\nK:Overload:4",
        "Name:OvDup\nManaCost:R\nTypes:Instant\nA:SP$ Tap | ValidTgts$ Creature\nK:Overload:3 R\nK:Overload:4 R",
        "Name:OvDisc\nManaCost:1 B\nTypes:Sorcery\nA:SP$ Discard | ValidTgts$ Player | NumCards$ 1 | Mode$ TgtChoose\nK:Overload:3 B",
        "Name:OvBadX\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2\nK:Overload:X R\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave15a_impending_lowering() {
    let imp = read(
        "Name:ImpendingSentinel\nManaCost:4 W W\nTypes:Enchantment Creature Avatar\nPT:5/5\n\
         K:Impending:4:2 W W",
    );
    assert_eq!(
        imp.impending.as_deref(),
        Some("Some((4, mana!(\"{2}{W}{W}\")))")
    );

    for bad_imp in [
        "Name:ImpZero\nManaCost:2 G\nTypes:Creature Beast\nPT:3/3\nK:Impending:0:1 G",
        "Name:ImpX\nManaCost:3 R\nTypes:Creature Giant\nPT:4/4\nK:Impending:2:X R\nSVar:X:Count$xPaid",
        "Name:ImpNonMana\nManaCost:3 B\nTypes:Creature Demon\nPT:4/4\nK:Impending:3:1 B PayLife<2>",
        "Name:ImpDup\nManaCost:3 U\nTypes:Creature Kraken\nPT:5/5\nK:Impending:3:1 U\nK:Impending:2:2 U",
        "Name:ImpSpell\nManaCost:2 R\nTypes:Instant\nK:Impending:2:R\nA:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad_imp), "must refuse invalid Impending: {bad_imp}");
    }
}

#[test]
fn wave15a_gift_lowering() {
    for (gift_ability, expected) in [
        (
            "DB$ Draw | Defined$ Promised | GiftDescription$ a card",
            "Some(GiftKind::Card)",
        ),
        (
            "DB$ Token | TokenScript$ c_a_food_sac | TokenOwner$ Promised | GiftDescription$ a Food",
            "Some(GiftKind::Food)",
        ),
        (
            "DB$ Token | TokenScript$ u_1_1_fish | TokenTapped$ True | TokenOwner$ Promised | LockTokenScript$ True | GiftDescription$ a tapped Fish",
            "Some(GiftKind::TappedFish)",
        ),
        (
            "DB$ Token | TokenScript$ c_a_treasure_sac | TokenOwner$ Promised | GiftDescription$ a Treasure",
            "Some(GiftKind::Treasure)",
        ),
        (
            "DB$ Token | TokenScript$ u_8_8_octopus | TokenOwner$ Promised | LockTokenScript$ True | GiftDescription$ an Octopus",
            "Some(GiftKind::Octopus)",
        ),
        (
            "DB$ AddTurn | Defined$ Promised | NumTurns$ 1 | GiftDescription$ an extra turn",
            "Some(GiftKind::ExtraTurn)",
        ),
    ] {
        let body = read(&format!(
            "Name:GiftBearer\nManaCost:2 U\nTypes:Creature Otter\nPT:2/2\n\
             K:Gift\nSVar:GiftAbility:{gift_ability}"
        ));
        assert_eq!(body.gift.as_deref(), Some(expected), "{gift_ability}");
    }

    for bad_gift in [
        "Name:GiftNoSvar\nManaCost:1 W\nTypes:Instant\nK:Gift\nA:SP$ GainLife | LifeAmount$ 2",
        "Name:GiftLand\nTypes:Land\nK:Gift\nSVar:GiftAbility:DB$ Draw | Defined$ Promised | GiftDescription$ a card",
        "Name:GiftDup\nManaCost:1 U\nTypes:Creature\nPT:1/1\nK:Gift\nK:Gift\nSVar:GiftAbility:DB$ Draw | Defined$ Promised | GiftDescription$ a card",
        "Name:GiftMakeCard\nManaCost:2 U\nTypes:Instant\nK:Gift\nSVar:GiftAbility:DB$ MakeCard | Name$ Arcane Tome | Defined$ Promised | Zone$ Battlefield | GiftDescription$ an Arcane Tome\nA:SP$ Draw | NumCards$ 1",
        "Name:GiftUntappedFish\nManaCost:1 U\nTypes:Instant\nK:Gift\nSVar:GiftAbility:DB$ Token | TokenScript$ u_1_1_fish | TokenOwner$ Promised | GiftDescription$ a tapped Fish\nA:SP$ Draw | NumCards$ 1",
        "Name:GiftExtraKey\nManaCost:1 U\nTypes:Instant\nK:Gift\nSVar:GiftAbility:DB$ Draw | Defined$ Promised | ExtraKey$ Weird | GiftDescription$ a card\nA:SP$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad_gift), "must refuse invalid Gift: {bad_gift}");
    }
}

#[test]
fn wave15a_conditional_etb_counters_and_nickname_grave_exile() {
    let raid = read(
        "Name:RaidEtb\nManaCost:2 R\nTypes:Creature Warrior\nPT:2/2\n\
         K:etbCounter:P1P1:1:CheckSVar$ RaidTest:Raid desc\n\
         SVar:RaidTest:Count$AttackersDeclared",
    );
    assert_eq!(
        raid.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(1), condition: Condition::AttackedWithAtLeast(1) }"
        ]
    );

    let revolt = read(
        "Name:RevoltEtb\nManaCost:1 G\nTypes:Creature Elf\nPT:2/1\n\
         K:etbCounter:P1P1:2:Revolt$ True:Revolt desc",
    );
    assert_eq!(
        revolt.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(2), condition: Condition::Revolt }"
        ]
    );

    let from_hand = read(
        "Name:HandEtb\nManaCost:4 W W\nTypes:Creature Spirit\nPT:4/4\n\
         K:etbCounter:P1P1:1:CheckSVar$ FromHand:desc\n\
         SVar:FromHand:Count$wasCastFromYourHandByYou.1.0",
    );
    assert!(
        from_hand.enter_modifiers[0].contains("WithCountersIf")
            && from_hand.enter_modifiers[0].contains("Condition::SourceWasCastByYou")
            && from_hand.enter_modifiers[0].contains("Filter::CastFromHand"),
        "{:?}",
        from_hand.enter_modifiers
    );

    let not_hand = read(
        "Name:NotHandEtb\nManaCost:2 B\nTypes:Creature Horror\nPT:3/2\n\
         K:etbCounter:P1P1:1:ValidCard$ Card.Self+!wasCastFromYourHandByYou:desc",
    );
    assert!(
        not_hand.enter_modifiers[0].contains("Condition::Not(")
            && not_hand.enter_modifiers[0].contains("Filter::CastFromHand"),
        "{:?}",
        not_hand.enter_modifiers
    );

    let cast_by_you = read(
        "Name:CastYouEtb\nManaCost:2 U\nTypes:Creature Wizard\nPT:2/2\n\
         K:etbCounter:P1P1:1:ValidCard$ Card.Self+wasCastByYou:desc",
    );
    assert_eq!(
        cast_by_you.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(1), condition: Condition::SourceWasCastByYou }"
        ]
    );

    let no_mana = read(
        "Name:PlotBonus\nManaCost:2 G\nTypes:Creature Centaur\nPT:3/3\n\
         K:etbCounter:P1P1:2:CheckSVar$ X | SVarCompare$ EQ0:desc\n\
         SVar:X:Count$CastTotalManaSpent",
    );
    assert_eq!(
        no_mana.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(2), condition: Condition::Not(&Condition::AmountAtLeast(Amount::ManaSpentToCast, 1)) }"
        ]
    );
}

#[test]
fn wave15a_conditional_etb_counter_clauses_and_nickname_grave_exile() {
    let inline_morbid = read(
        "Name:MorbidEtb\nManaCost:3 B\nTypes:Creature Demon\nPT:3/3\n\
         K:etbCounter:P1P1:1:CheckSVar$ Count$Morbid.1.0:desc",
    );
    assert_eq!(
        inline_morbid.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(1), condition: Condition::CreaturesDiedThisTurnAtLeast(1) }"
        ]
    );

    let is_present = read(
        "Name:BigFriendEtb\nManaCost:3 G\nTypes:Creature Beast\nPT:3/3\n\
         K:etbCounter:P1P1:1:IsPresent$ Permanent.YouCtrl+cmcGE4:desc",
    );
    assert!(
        is_present.enter_modifiers[0].contains("WithCountersIf")
            && is_present.enter_modifiers[0].contains("Condition::ControlCount("),
        "{:?}",
        is_present.enter_modifiers
    );

    let cmp_morbid = read(
        "Name:CmpMorbidEtb\nManaCost:2 B\nTypes:Creature Shade\nPT:1/1\n\
         K:etbCounter:P1P1:2:CheckSVar$ X:desc\n\
         SVar:X:Count$Compare Y GE1.1.0\nSVar:Y:Count$Morbid.1.0",
    );
    assert_eq!(
        cmp_morbid.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(2), condition: Condition::CreaturesDiedThisTurnAtLeast(1) }"
        ]
    );

    let nick_exile = read(
        "Name:NickGrave\nManaCost:2 B\nTypes:Creature Zombie\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 B ExileFromGrave<1/NICKNAME> | ActivationZone$ Graveyard | NumCards$ 1",
    );
    assert!(
        nick_exile.abilities[0].contains("ExileSelf"),
        "{:?}",
        nick_exile.abilities
    );

    for bad_etb in [
        "Name:BadAdamantEtb\nManaCost:3 W\nTypes:Creature Knight\nPT:2/5\nK:etbCounter:P1P1:1:Adamant$ Purple:desc",
        "Name:SelfCtrPresent\nManaCost:2 G\nTypes:Creature\nPT:2/2\nK:etbCounter:P1P1:1:IsPresent$ Card.Self+counters_GE1_P1P1:desc",
        "Name:DragonPresence\nManaCost:3 R\nTypes:Creature\nPT:3/3\nK:etbCounter:P1P1:1:CheckSVar$ Count$Presence_Dragon.1.0:desc",
    ] {
        assert!(
            refused(bad_etb),
            "must refuse unsupported etbCounter: {bad_etb}"
        );
    }
}

#[test]
fn wave16a_warp_craft_and_descend_inference() {
    let warp_x = read(
        "Name:WarpWalker\nManaCost:2 U\nTypes:Creature Spirit\nPT:2/2\n\
         K:Warp:X U\nSVar:X:Count$xPaid",
    );
    assert_eq!(warp_x.warp.as_deref(), Some("Some(cost!(\"{X}{U}\"))"));
    assert_eq!(warp_x.keywords, ["KeywordSet::WARP"]);

    let craft_fixed = read(
        "Name:CraftRelic\nManaCost:2 R\nTypes:Artifact\n\
         K:Craft:4 ExileCtrlOrGrave<1/Artifact.Other>:other artifacts:them\n\
         AlternateMode:DoubleFaced\nALTERNATE\n\
         Name:CraftBack\nColors:red\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1",
    );
    assert!(craft_fixed.abilities[0].contains("ReturnCraftedTransformed"));
    assert!(craft_fixed.abilities[0].contains("ExileSelf"));
    assert!(craft_fixed.abilities[0].contains("CraftMaterials"));
    assert!(craft_fixed.abilities[0].contains("min: 1, max: 1"));

    let craft_multi = read(
        "Name:CraftIdol\nManaCost:2\nTypes:Artifact\n\
         K:Craft:2 XMin1 ExileCtrlOrGrave<X/Permanent.Other>\n\
         AlternateMode:DoubleFaced\nALTERNATE\n\
         Name:CraftIdolBack\nColors:red\nTypes:Artifact\n",
    );
    assert!(craft_multi.abilities[0].contains("min: 1, max: 255"));
    assert!(
        craft_multi
            .statics
            .contains("Filter::HasType(TypeSet::ARTIFACT")
            && craft_multi.statics.contains("TypeSet::CREATURE"),
        "`Permanent.Other` must retain permanent card types: {}",
        craft_multi.statics
    );

    let craft_grave = read(
        "Name:CraftTome\nManaCost:1 R\nTypes:Artifact\n\
         K:Craft:3 XMin4 ExileFromGrave<X/Instant.Red+Other;Sorcery.Red+Other> ExileCtrlOrGrave<1/Creature.Other>\n\
         AlternateMode:DoubleFaced\nALTERNATE\n\
         Name:CraftTomeBack\nColors:red\nTypes:Artifact\n",
    );
    assert!(
        craft_grave.statics.contains("ZoneRef::Graveyard")
            && craft_grave.abilities[0].contains("min: 4, max: 255")
            && craft_grave.abilities[0].contains("min: 1, max: 1"),
        "craft grave = {:?}",
        craft_grave.statics
    );

    let d4 = read(
        "Name:DescendSpy4\nManaCost:2 B\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Permanent.YouOwn | \
         PresentZone$ Graveyard | SpellDescription$ Descend 4 — Draw a card.",
    );
    let d8 = read(
        "Name:DescendSpy8\nManaCost:3 B\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Permanent.YouOwn | \
         PresentZone$ Graveyard | SpellDescription$ Descend 8 — Draw a card.",
    );
    let d_one = read(
        "Name:OneCardGrave\nManaCost:1 U\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Permanent.YouOwn | \
         PresentZone$ Graveyard | SpellDescription$ Draw a card if a permanent card is in your graveyard.",
    );
    assert!(
        d4.abilities[0].contains("AmountAtLeast(") && d4.abilities[0].contains(", 4)"),
        "{:?}",
        d4.abilities
    );
    assert!(
        d8.abilities[0].contains("AmountAtLeast(") && d8.abilities[0].contains(", 8)"),
        "{:?}",
        d8.abilities
    );
    assert!(
        d_one.abilities[0].contains("AmountAtLeast(") && d_one.abilities[0].contains(", 1)"),
        "{:?}",
        d_one.abilities
    );

    for bad in [
        "Name:DupWarp\nManaCost:2 W\nTypes:Creature Knight\nPT:2/2\nK:Warp:1 W\nK:Warp:2 W",
        "Name:LandWarp\nTypes:Land\nK:Warp:1",
        "Name:SpellWarp\nManaCost:1 U\nTypes:Instant\nK:Warp:U\nA:SP$ Draw | NumCards$ 1",
        "Name:TapWarp\nManaCost:1 W\nTypes:Creature\nPT:1/1\nK:Warp:T",
        "Name:SoloCraft\nManaCost:2\nTypes:Artifact\nK:Craft:2 ExileCtrlOrGrave<1/Artifact.Other>",
        "Name:NoXMinCraft\nManaCost:2\nTypes:Artifact\nK:Craft:2 ExileCtrlOrGrave<X/Artifact.Other>\nAlternateMode:DoubleFaced\nALTERNATE\nName:Back\nColors:blue\nTypes:Artifact",
        "Name:AmbigDescend\nManaCost:2 B\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Permanent.YouOwn | \
         PresentZone$ Graveyard | SpellDescription$ Threshold — Draw if three or more.",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave16a_spree_charms_and_refusals() {
    let spree = read(
        "Name:SpreeCharm\nManaCost:R\nTypes:Instant\nK:Spree\n\
         A:SP$ Charm | Choices$ M1,M2,M3,M4 | MinCharmNum$ 1 | CharmNum$ 4\n\
         SVar:M1:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | ModeCost$ W\n\
         SVar:M2:DB$ Destroy | ValidTgts$ Artifact | ModeCost$ 1 R\n\
         SVar:M3:DB$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ M3Fight | ModeCost$ 2 R\n\
         SVar:M3Fight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature.OppCtrl\n\
         SVar:M4:DB$ Draw | NumCards$ 1 | ModeCost$ 1",
    );
    assert_eq!(spree.keywords, ["KeywordSet::SPREE"]);
    assert!(
        spree.abilities[0].contains("additional_cost = Some(mana!(\"{W}\"))")
            && spree.abilities[0].contains("additional_cost = Some(mana!(\"{1}{R}\"))")
            && spree.abilities[0].contains("additional_cost = Some(mana!(\"{2}{R}\"))")
            && spree.abilities[0].contains("additional_cost = Some(mana!(\"{1}\"))")
            && spree.abilities[0].contains("second_targets = Some("),
        "{:?}",
        spree.abilities
    );

    for bad in [
        "Name:NoKSpree\nManaCost:R\nTypes:Instant\n\
         A:SP$ Charm | Choices$ M1,M2 | MinCharmNum$ 1 | CharmNum$ 2\n\
         SVar:M1:DB$ Draw | NumCards$ 1 | ModeCost$ 1\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2 | ModeCost$ 1",
        "Name:SpreeDefaultOne\nManaCost:R\nTypes:Instant\nK:Spree\n\
         A:SP$ Charm | Choices$ M1,M2\n\
         SVar:M1:DB$ Draw | NumCards$ 1 | ModeCost$ 1\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2 | ModeCost$ 1",
        "Name:TieredOne\nManaCost:R\nTypes:Instant\nK:Spree\n\
         A:SP$ Charm | MinCharmNum$ 1 | CharmNum$ 1 | Choices$ M1,M2\n\
         SVar:M1:DB$ Draw | NumCards$ 1 | ModeCost$ 1\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2 | ModeCost$ 1",
        "Name:MissingModeCost\nManaCost:R\nTypes:Instant\nK:Spree\n\
         A:SP$ Charm | Choices$ M1,M2 | MinCharmNum$ 1 | CharmNum$ 2\n\
         SVar:M1:DB$ Draw | NumCards$ 1 | ModeCost$ 1\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2",
        "Name:VarModeCost\nManaCost:R\nTypes:Instant\nK:Spree\n\
         A:SP$ Charm | Choices$ M1,M2 | MinCharmNum$ 1 | CharmNum$ 2\n\
         SVar:M1:DB$ Draw | NumCards$ 1 | ModeCost$ X\n\
         SVar:M2:DB$ GainLife | LifeAmount$ 2 | ModeCost$ 1\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave17a_retrace_monstrosity_cost_plumbing_and_refusals() {
    let retrace_sorcery = read(
        "Name:BogChant\nManaCost:B B\nTypes:Sorcery\nK:Retrace\n\
         A:SP$ Draw | NumCards$ 1",
    );
    assert_eq!(retrace_sorcery.keywords, ["KeywordSet::RETRACE"]);

    let retrace_instant = read(
        "Name:SparkJab\nManaCost:R\nTypes:Instant\nK:Retrace\n\
         A:SP$ DealDamage | NumDmg$ 1 | ValidTgts$ Any",
    );
    assert_eq!(retrace_instant.keywords, ["KeywordSet::RETRACE"]);

    let retrace_perm = read("Name:GraveRelic\nManaCost:2 G\nTypes:Enchantment\nK:Retrace");
    assert_eq!(retrace_perm.keywords, ["KeywordSet::RETRACE"]);

    let xx_cost = read(
        "Name:GiantHydra\nManaCost:4 G\nTypes:Creature Hydra\nPT:4/4\n\
         A:AB$ PutCounter | Cost$ X X G | Defined$ Self | CounterType$ P1P1 | CounterNum$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        xx_cost.abilities[0].contains(r#"cost!("{X}{X}{G}")"#),
        "{:?}",
        xx_cost.abilities
    );

    let snow_cost = read(
        "Name:FrostGiant\nManaCost:5 R\nTypes:Creature Giant\nPT:5/5\n\
         A:AB$ PutCounter | Cost$ 4 S S | Defined$ Self | CounterType$ P1P1 | CounterNum$ 3",
    );
    assert!(
        snow_cost.abilities[0].contains(r#"cost!("{4}{S}{S}")"#),
        "{:?}",
        snow_cost.abilities
    );

    let grave_reduce = read(
        "Name:CryptBehemoth\nManaCost:4 B B\nTypes:Creature Demon\nPT:5/5\n\
         A:AB$ PutCounter | Cost$ 6 B B | Defined$ Self | CounterType$ P1P1 | CounterNum$ 4 | ReduceCost$ X\n\
         SVar:X:Count$ValidGraveyard Creature.YouCtrl",
    );
    assert!(
        grave_reduce.abilities[0].contains("CostReduction::PerCount")
            && grave_reduce.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::You)"),
        "{:?}",
        grave_reduce.abilities
    );

    let opp_reduce = read(
        "Name:ColossusFoe\nManaCost:4 G G\nTypes:Creature Colossus\nPT:6/6\n\
         A:AB$ PutCounter | Cost$ 6 G | Defined$ Self | CounterType$ P1P1 | CounterNum$ 4 | ReduceCost$ X\n\
         SVar:X:Count$Valid Creature.OppCtrl+powerGE4",
    );
    assert!(
        opp_reduce.abilities[0].contains("CostReduction::PerCount"),
        "{:?}",
        opp_reduce.abilities
    );

    for bad in [
        "Name:RetraceLand\nTypes:Land\nK:Retrace",
        "Name:TypelessRetrace\nK:Retrace",
        "Name:DupRetrace\nManaCost:R\nTypes:Sorcery\nK:Retrace\nK:Retrace\nA:SP$ Draw | NumCards$ 1",
        "Name:ParamRetrace\nManaCost:R\nTypes:Sorcery\nK:Retrace:2\nA:SP$ Draw | NumCards$ 1",
        "Name:LureBeast\nManaCost:2 G\nTypes:Creature Beast\nPT:3/3\nK:CARDNAME must be blocked if able.",
        "Name:SoloBeast\nManaCost:2 W\nTypes:Creature Beast\nPT:3/3\nK:CARDNAME can't attack alone.",
        "Name:TwoBlockBeast\nManaCost:3 G\nTypes:Creature Beast\nPT:4/4\nK:CARDNAME must be blocked by two or more creatures if able.",
        "Name:FlashExtra\nManaCost:1 U\nTypes:Creature Bird\nPT:2/2\nK:MayFlashSac:2",
        "Name:SelPartner\nManaCost:2 W\nTypes:Legendary Creature Human\nPT:2/2\nK:Partner:Character select",
        "Name:HexBlack\nManaCost:1 U\nTypes:Creature Drake\nPT:2/2\nK:Hexproof:Black",
        "Name:UncomparedCount\nManaCost:2 G\nTypes:Creature Beast\nPT:2/2\n\
         K:etbCounter:P1P1:1:CheckSVar$ L\nSVar:L:Count$Valid Creature.YouCtrl",
        "Name:UnknownCounterKind\nManaCost:2 U\nTypes:Artifact\nK:etbCounter:OMEN:1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave18a_planeswalker_loyalty_lowering() {
    // Full planeswalker: +N, 0 (AddCounter<0> and SubCounter<0>), -N with
    // up_to_one target, mana-adding loyalty ability (CR 605.1a: NOT mana_ability!),
    // lowercase `Planeswalker$ true`, and `Ultimate$ True` emblem.
    let walker = read(
        "Name:AetherSage\nManaCost:2 U R\nTypes:Legendary Planeswalker Sage\nLoyalty:4\n\
         A:AB$ Draw | Cost$ AddCounter<2/LOYALTY> | Planeswalker$ True | NumCards$ 1\n\
         A:AB$ GainLife | Cost$ AddCounter<0/LOYALTY> | Planeswalker$ True | LifeAmount$ 2\n\
         A:AB$ Mana | Cost$ SubCounter<0/LOYALTY> | Planeswalker$ true | Produced$ R | Amount$ 2\n\
         A:AB$ ChangeZone | Cost$ SubCounter<3/LOYALTY> | Planeswalker$ True | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1\n\
         A:AB$ Effect | Cost$ SubCounter<7/LOYALTY> | Planeswalker$ True | Ultimate$ True | Name$ Emblem — AetherSage | StaticAbilities$ NoMax | Duration$ Permanent\n\
         SVar:NoMax:Mode$ Continuous | Affected$ You | SetMaxHandSize$ Unlimited | EffectZone$ Command",
    );
    assert_eq!(walker.abilities.len(), 5, "{:?}", walker.abilities);
    assert_eq!(walker.abilities[0], "loyalty!(2, &[Effect::draw(1)])");
    assert_eq!(walker.abilities[1], "loyalty!(0, &[Effect::gain_life(2)])");
    assert_eq!(
        walker.abilities[2],
        "loyalty!(0, &[Effect::mana(ManaColor::Red, 2)])"
    );
    assert!(
        walker.abilities[3].starts_with("loyalty!(-3, &[Effect::bounce(")
            && walker.abilities[3].contains("targets = Some(TargetReq::up_to_one("),
        "{}",
        walker.abilities[3]
    );
    assert!(
        walker.abilities[4].starts_with("loyalty!(-7, &[Effect::CreateEmblem {")
            && walker.abilities[4].contains("Modifier::NoMaxHandSize"),
        "{}",
        walker.abilities[4]
    );

    // Single-ability planeswalker with `{T}:` in Oracle text must NEVER have
    // `TapSelf` inferred onto its loyalty cost.
    let no_tap_infer = read(
        "Name:LoneWalker\nManaCost:3 G\nTypes:Legendary Planeswalker Lone\nLoyalty:3\n\
         A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | NumCards$ 1\n\
         Oracle:+1: Draw a card. (Not {T}: draw.)",
    );
    assert_eq!(no_tap_infer.abilities, ["loyalty!(1, &[Effect::draw(1)])"]);

    // Second target requirement on a loyalty ability.
    let two_target_walker = read(
        "Name:TwinWalker\nManaCost:1 G U\nTypes:Legendary Planeswalker Twin\nLoyalty:4\n\
         A:AB$ Pump | Cost$ SubCounter<5/LOYALTY> | Planeswalker$ True | \
         ValidTgts$ Creature | NumAtt$ +2 | SubAbility$ Next\n\
         SVar:Next:DB$ Pump | ValidTgts$ Creature.YouCtrl | NumDef$ +5",
    );
    assert!(
        two_target_walker.abilities[0].starts_with("loyalty!(-5, &[Effect::PumpTarget {")
            && two_target_walker.abilities[0].contains("targets = Some(TargetReq::one(")
            && two_target_walker.abilities[0].contains("second_targets = Some(TargetReq::one("),
        "{}",
        two_target_walker.abilities[0]
    );
}

#[test]
fn wave18a_planeswalker_loyalty_cost_refusals() {
    for (script, expected) in [
        (
            "Name:BadX\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ SubCounter<X/LOYALTY> | Planeswalker$ True | NumCards$ X\n\
             SVar:X:Count$xPaid",
            "a variable-X loyalty cost",
        ),
        (
            "Name:BadMixed\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> 1 | Planeswalker$ True | NumCards$ 1",
            "a non-loyalty cost `AddCounter<1/LOYALTY> 1` on a loyalty ability",
        ),
        (
            "Name:BadNonLoyaltyCounter\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/P1P1> | Planeswalker$ True | NumCards$ 1",
            "a non-loyalty cost `AddCounter<1/P1P1>` on a loyalty ability",
        ),
        (
            "Name:BadZeroCost\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ 0 | Planeswalker$ True | NumCards$ 1",
            "a non-loyalty cost `0` on a loyalty ability",
        ),
        (
            "Name:BadOverflow\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ SubCounter<128/LOYALTY> | Planeswalker$ True | NumCards$ 1",
            "a loyalty cost count `128`",
        ),
        (
            "Name:BadNoCost\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Planeswalker$ True | NumCards$ 1",
            "a loyalty ability with no `Cost$`",
        ),
        (
            "Name:BadUltOnly\nTypes:Artifact\n\
             A:AB$ Draw | Cost$ T | Ultimate$ True | NumCards$ 1",
            "`Ultimate$` without `Planeswalker$ True`",
        ),
        (
            "Name:BadPwFalse\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ False | NumCards$ 1",
            "`Planeswalker$ False`",
        ),
        (
            "Name:BadUltFalse\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ SubCounter<5/LOYALTY> | Planeswalker$ True | Ultimate$ False | NumCards$ 1",
            "`Ultimate$ False`",
        ),
    ] {
        let parsed = parse(script);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {script}"
        );
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(expected),
            "wrong refusal reason for: {script}"
        );
    }
}

#[test]
fn wave18a_planeswalker_loyalty_modifier_refusals() {
    for (script, expected) in [
        (
            "Name:BadSpellPw\nManaCost:U\nTypes:Sorcery\n\
             A:SP$ Draw | Planeswalker$ True | NumCards$ 1",
            "`Planeswalker$` on a spell",
        ),
        (
            "Name:BadLimit\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | ActivationLimit$ 2 | NumCards$ 1",
            "`ActivationLimit$` on a loyalty ability",
        ),
        (
            "Name:BadZone\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | ActivationZone$ Graveyard | NumCards$ 1",
            "a non-battlefield `ActivationZone$` on a loyalty ability",
        ),
        (
            "Name:BadActivator\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Activator$ Player | NumCards$ 1",
            "`Activator$ Player` on a loyalty ability",
        ),
        (
            "Name:BadSorcery\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | SorcerySpeed$ True | NumCards$ 1",
            "`SorcerySpeed$` on a loyalty ability",
        ),
        (
            "Name:BadAltCost\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | AlternateCost$ SubCounter<1/LOYALTY> | Planeswalker$ True | NumCards$ 1",
            "`AlternateCost$` on a loyalty ability",
        ),
        (
            "Name:BadCond\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Condition$ Threshold | NumCards$ 1",
            "an activation condition on a loyalty ability",
        ),
        (
            "Name:BadGrantPw\nManaCost:2 W\nTypes:Enchantment\n\
             S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddAbility$ GrantPw\n\
             SVar:GrantPw:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | NumCards$ 1",
            "a granted loyalty ability",
        ),
        (
            "Name:BadModalPw\nTypes:Legendary Planeswalker\nLoyalty:3\n\
             A:AB$ Charm | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Choices$ M1,M2\n\
             SVar:M1:DB$ Draw | NumCards$ 1\n\
             SVar:M2:DB$ GainLife | LifeAmount$ 2",
            "unclaimed parameter `Charm.Planeswalker`",
        ),
    ] {
        let parsed = parse(script);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {script}"
        );
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(expected),
            "wrong refusal reason for: {script}"
        );
    }
}

#[test]
fn wave18a_put_counter_on_other_and_exile_same_grave() {
    let blight_act = read(
        "Name:BlightPing\nManaCost:2 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ DealDamage | Cost$ 2 R T AddCounter<1/M1M1/Creature.YouCtrl/a creature you control> | \
         ValidTgts$ Any | NumDmg$ 2",
    );
    assert!(
        blight_act.abilities[0].contains(
            "PutCounterOnOther { filter: &Filter::YOUR_CREATURE, kind: CounterKind::M1M1, n: 1 }"
        ),
        "{:?}",
        blight_act.abilities
    );

    let self_named_counter = read(
        "Name:SelfCounter\nManaCost:1 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ AddCounter<1/M1M1/CARDNAME> | NumCards$ 1",
    );
    assert!(
        self_named_counter.abilities[0]
            .contains("PutCounterSelf { kind: CounterKind::M1M1, n: 1 }"),
        "{:?}",
        self_named_counter.abilities
    );

    let night_soil = read(
        "Name:SoilEnch\nManaCost:G G\nTypes:Enchantment\n\
         A:AB$ GainLife | Cost$ 1 ExileSameGrave<2/Creature> | LifeAmount$ 1",
    );
    assert!(
        night_soil.abilities[0]
            .contains("ExileFromSingleGraveyard { filter: &Filter::CREATURE, count: 2 }"),
        "{:?}",
        night_soil.abilities
    );

    for bad in [
        "Name:OppCounter\nTypes:Artifact\nA:AB$ Draw | Cost$ AddCounter<1/P1P1/Creature.OppCtrl> | NumCards$ 1",
        "Name:AnyCounter\nTypes:Artifact\nA:AB$ Draw | Cost$ AddCounter<1/P1P1/Creature> | NumCards$ 1",
        "Name:SpellOtherCounter\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Cost$ B AddCounter<1/M1M1/Creature.YouCtrl> | NumCards$ 1",
        "Name:ZeroOtherCounter\nTypes:Artifact\nA:AB$ Draw | Cost$ AddCounter<0/M1M1/Creature.YouCtrl> | NumCards$ 1",
        "Name:OwnSameGrave\nTypes:Artifact\nA:AB$ Draw | Cost$ ExileSameGrave<2/Creature.YouOwn> | NumCards$ 1",
        "Name:ZeroSameGrave\nTypes:Artifact\nA:AB$ Draw | Cost$ ExileSameGrave<0/Creature> | NumCards$ 1",
        "Name:SpellSameGrave\nManaCost:G\nTypes:Sorcery\nA:SP$ Draw | Cost$ G ExileSameGrave<2/Creature> | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave18a_activation_svar_and_present_conditions() {
    // `CheckSVar$` with `Count$ThisTurnEntered_Battlefield_Creature.YouCtrl`
    // must fall through to `svar_condition_clause` instead of failing subtype lookup.
    let etb_check = read(
        "Name:EtbTapper\nManaCost:1 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ Tap | Cost$ T | ValidTgts$ Creature | CheckSVar$ X\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Creature.YouCtrl",
    );
    assert!(
        etb_check.abilities[0].contains(
            "condition = Some(Condition::AmountAtLeast(Amount::CreaturesEnteredUnderYourControlThisTurn, 1))"
        ),
        "{:?}",
        etb_check.abilities
    );

    // Inline `Count$CommittedCrimeThisTurn.1.0`, `PlayerCountOpponents$HasPropertywasDealtDamageThisTurn`,
    // `IsPresent$ Card.YouCtrl+IsRingbearer`, and `Condition$ Morbid`/`Raid`.
    let crime_ab = read(
        "Name:CrimeScout\nManaCost:1 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ Count$CommittedCrimeThisTurn.1.0",
    );
    assert!(
        crime_ab.abilities[0]
            .contains("condition = Some(Condition::CommittedCrimeThisTurn(PlayerRel::You))"),
        "{:?}",
        crime_ab.abilities
    );

    let bloodthirst_ab = read(
        "Name:BloodScout\nManaCost:1 R\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ X\n\
         SVar:X:PlayerCountOpponents$HasPropertywasDealtDamageThisTurn",
    );
    assert!(
        bloodthirst_ab.abilities[0]
            .contains("condition = Some(Condition::OpponentDealtDamageThisTurn)"),
        "{:?}",
        bloodthirst_ab.abilities
    );

    let ring_ab = read(
        "Name:RingScout\nManaCost:1 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | IsPresent$ Card.YouCtrl+IsRingbearer | PresentCompare$ EQ0",
    );
    assert!(
        ring_ab.abilities[0]
            .contains("condition = Some(Condition::Not(&Condition::HasRingBearer))"),
        "{:?}",
        ring_ab.abilities
    );

    let morbid_ab = read(
        "Name:MorbidScout\nManaCost:1 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | Condition$ Morbid",
    );
    assert!(
        morbid_ab.abilities[0]
            .contains("condition = Some(Condition::CreaturesDiedThisTurnAtLeast(1))"),
        "{:?}",
        morbid_ab.abilities
    );
}

#[test]
fn wave19a_exhaust_boast_game_limit_and_svar_activation_limit() {
    let exhaust = read(
        "Name:ExhaustBot\nManaCost:2\nTypes:Artifact Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 2 T | NumCards$ 1 | Exhaust$ True",
    );
    assert_eq!(
        exhaust.abilities,
        [
            "activated!(cost!(\"{2}\", TapSelf), &[Effect::draw(1)], limit = ActivationLimit::Exhaust)"
        ]
    );

    let game_once = read(
        "Name:OnceBot\nManaCost:1\nTypes:Creature\nPT:1/1\n\
         A:AB$ PutCounter | Cost$ 3 | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2 | GameActivationLimit$ 1",
    );
    assert!(
        game_once.abilities[0].contains("limit = ActivationLimit::Exhaust"),
        "{:?}",
        game_once.abilities
    );

    let boast = read(
        "Name:BoastWarrior\nManaCost:W\nTypes:Creature\nPT:2/1\n\
         A:AB$ Tap | Cost$ 1 W | ValidTgts$ Creature | Boast$ True",
    );
    assert!(
        boast.abilities[0].contains("limit = ActivationLimit::Boast"),
        "{:?}",
        boast.abilities
    );

    let per_turn_amount = read(
        "Name:SnowLimit\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ B | NumCards$ 1 | ActivationLimit$ X\n\
         SVar:X:Count$Valid Land.Snow+YouCtrl",
    );
    assert!(
        per_turn_amount.abilities[0]
            .contains("limit = ActivationLimit::PerTurnAmount(Amount::CountOf {"),
        "{:?}",
        per_turn_amount.abilities
    );

    for bad in [
        "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | Exhaust$ True",
        "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | GameActivationLimit$ 1",
        "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | Boast$ True",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | Exhaust$ False",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | GameActivationLimit$ 2",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | Boast$ False",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | Exhaust$ True | Boast$ True",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | ActivationLimit$ 1 | GameActivationLimit$ 1",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | ActivationLimit$ X\nSVar:X:Count$xPaid",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Exhaust$ True | NumCards$ 1",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Boast$ True | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave19a_charms_with_cost_min_num_limits_and_additional_description() {
    let sp_cost = read(
        "Name:SacCharm\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Charm | Cost$ 1 B Sac<1/Creature> | MinCharmNum$ 1 | CharmNum$ 2 | Choices$ M1,M2\n\
         SVar:M1:DB$ Draw | NumCards$ 2\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Sacrificed$CardPower",
    );
    assert_eq!(
        sp_cost.mandatory_additional_costs,
        ["CostPart::Sacrifice(&Filter::YOUR_CREATURE)"]
    );
    assert!(
        sp_cost.abilities[0].contains("Amount::SacrificedPower"),
        "{:?}",
        sp_cost.abilities
    );

    let ab_modal = read(
        "Name:ModalRelic\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ 2 T | Choices$ M1,M2 | MinCharmNum$ 1 | SorcerySpeed$ True | GameActivationLimit$ 1 | AdditionalDescription$ . Activate only as a sorcery and only once.\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 3",
    );
    assert!(
        ab_modal.abilities[0].contains("timing = ActivationTiming::SorcerySpeed")
            && ab_modal.abilities[0].contains("limit = ActivationLimit::Exhaust"),
        "{:?}",
        ab_modal.abilities
    );

    let ab_cond_desc = read(
        "Name:HatchRelic\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ Sac<1/CARDNAME> | Choices$ M1,M2 | IsPresent$ Card.Self+counters_GE2_CHARGE | AdditionalDescription$ . Activate only if there are two or more charge counters on CARDNAME.\n\
         SVar:M1:DB$ Draw | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    );
    assert!(
        ab_cond_desc.abilities[0].contains("condition = Some("),
        "{:?}",
        ab_cond_desc.abilities
    );

    for bad in [
        "Name:X\nManaCost:1 R\nTypes:Instant\nA:SP$ Charm | Choices$ M1,M2 | AdditionalDescription$ . If you control a commander as you cast this spell, you may choose both.\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
        "Name:X\nManaCost:1 R\nTypes:Instant\nA:SP$ Charm | Cost$ XMin1 X R | Choices$ M1,M2\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
        "Name:X\nTypes:Artifact\nA:AB$ Charm | Cost$ 1 | Choices$ M1,M2 | AdditionalDescription$ . Activate only as a sorcery.\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
        "Name:X\nTypes:Artifact\nA:AB$ Charm | Cost$ 1 | Choices$ M1,M2 | MinCharmNum$ 2\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
        "Name:X\nTypes:Artifact\nA:AB$ Charm | Cost$ 1 | Choices$ M1,M2 | Condition$ Threshold | AdditionalDescription$ . Activate only if you have threshold and only once.\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave19a_activators_hand_or_command_and_activation_x_min() {
    for (param, expected_zone) in [
        (
            "Activator$ Player.Opponent",
            "ActivationZone::BattlefieldOpponent",
        ),
        (
            "Activator$ Player.Owner",
            "ActivationZone::BattlefieldOwner",
        ),
        (
            "Activator$ Player.EnchantedController",
            "ActivationZone::BattlefieldAttachedController",
        ),
        (
            "ActivationZone$ Hand,Command",
            "ActivationZone::HandOrCommand",
        ),
        (
            "ActivationZone$ Command,Hand",
            "ActivationZone::HandOrCommand",
        ),
    ] {
        let body = read(&format!(
            "Name:ZoneTest\nManaCost:2\nTypes:Artifact\n\
             A:AB$ Draw | Cost$ 2 | NumCards$ 1 | {param}"
        ));
        assert!(
            body.abilities[0].contains(&format!("zone = {expected_zone}")),
            "{param} => {:?}",
            body.abilities
        );
    }

    let x_min = read(
        "Name:MinX\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ XMin1 X G | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(
        x_min.abilities,
        [
            "activated!(cost!(\"{X}{G}\"), &[Effect::DrawCards { amount: Amount::X }], x_value = ActivationX::AtLeast(1))"
        ]
    );

    let modal_x_min = read(
        "Name:ModalX\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ XMin1 X | Choices$ M1,M2 | AdditionalDescription$ . X can't be 0.\n\
         SVar:M1:DB$ Draw | NumCards$ X\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        modal_x_min.abilities[0].contains("x_value = ActivationX::AtLeast(1)"),
        "{:?}",
        modal_x_min.abilities
    );

    for bad in [
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ G | Activator$ Player.Opponent",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | Activator$ Player.Owner | ActivationZone$ Graveyard",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Activator$ Player.Opponent | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ XMin1 2 | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ XMin0 X | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ XMin1 XMin2 X | NumCards$ X\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave19a_devotion_mana_any_different_and_sub_counter_self() {
    let devotion = read(
        "Name:GreenAcolyte\nManaCost:3 G\nTypes:Creature\nPT:1/4\n\
         A:AB$ Mana | Cost$ T | Produced$ G | Amount$ X\n\
         SVar:X:Count$Devotion.Green",
    );
    assert_eq!(
        devotion.abilities,
        ["mana_ability!(&[Effect::mana_dynamic(ManaColor::Green, Amount::DevotionToManaColor)])"]
    );

    let beacon = read(
        "Name:BeaconLand\nTypes:Land\n\
         A:AB$ Mana | Cost$ 1 T | Produced$ Combo AnyDifferent | Amount$ 2 | RestrictValid$ Spell.Planeswalker",
    );
    assert!(
        beacon.abilities[0].contains(
            "Effect::mana_different_colors(ALL_MANA_COLORS, Amount::Fixed(2)).restricted("
        ),
        "{:?}",
        beacon.abilities
    );

    let self_sub = read(
        "Name:OilRelic\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ SubCounter<2/OIL/CARDNAME/this artifact> | NumCards$ 1",
    );
    assert_eq!(
        self_sub.abilities,
        ["activated!(cost!(RemoveCounterSelf { kind: counters::OIL, n: 2 }), &[Effect::draw(1)])"]
    );

    let nick_add = read(
        "Name:NickRelic\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ AddCounter<1/CHARGE/NICKNAME> | NumCards$ 1",
    );
    assert_eq!(
        nick_add.abilities,
        [
            "activated!(cost!(PutCounterSelf { kind: CounterKind::Charge, n: 1 }), &[Effect::draw(1)])"
        ]
    );

    let pw_card = read(
        "Name:WalkerCard\nManaCost:3 W\nTypes:Planeswalker\nLoyalty:4\n\
         A:AB$ Draw | Cost$ AddCounter<1/LOYALTY/CARDNAME> | Planeswalker$ True | NumCards$ 1",
    );
    assert_eq!(pw_card.abilities, ["loyalty!(1, &[Effect::draw(1)])"]);

    for bad in [
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | Amount$ X\nSVar:X:Count$Devotion.Red",
        "Name:X\nTypes:Creature\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ C | Amount$ X\nSVar:X:Count$Devotion.Green",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo AnyDifferent | Amount$ 6",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo AnyDifferent | Amount$ 2 | Each$ 2",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo Chosen AnyDifferent | Amount$ 2",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/LOYALTY/CARDNAME> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/OIL/Creature.OppCtrl> | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20a_mana_reflected_is_and_produce_extensions() {
    let exiled = read(
        "Name:ImprintMox\nManaCost:0\nTypes:Artifact\n\
         A:AB$ ManaReflected | Cost$ T | Valid$ Defined.ExiledWith | ColorOrType$ Color | ReflectProperty$ Is",
    );
    assert_eq!(
        exiled.abilities,
        ["mana_ability!(&[Effect::mana_exiled_colors()])"]
    );

    let self_colors = read(
        "Name:SelfMana\nManaCost:2\nTypes:Creature\nPT:1/1\n\
         A:AB$ ManaReflected | Cost$ T | Valid$ Defined.Self | ColorOrType$ Color | ReflectProperty$ Is",
    );
    assert!(
        self_colors.abilities[0].contains(
            "ManaSource::ColorsAmong { filter: &Filter::This, zone: ZoneSel::Battlefield }"
        ),
        "{:?}",
        self_colors.abilities
    );

    let leg = read(
        "Name:LegendMox\nManaCost:0\nTypes:Artifact\n\
         A:AB$ ManaReflected | Cost$ T | Valid$ Permanent.Legendary+YouCtrl | ColorOrType$ Color | ReflectProperty$ Is",
    );
    assert!(
        leg.abilities[0]
            .contains("ManaSource::ColorsAmong { filter: &MANA1, zone: ZoneSel::Battlefield }"),
        "{:?}",
        leg.abilities
    );

    let gy = read(
        "Name:GraveMox\nManaCost:2\nTypes:Artifact\n\
         A:AB$ ManaReflected | Cost$ T | Valid$ Defined.ValidGraveyard Card.YouOwn | ColorOrType$ Color | ReflectProperty$ Is",
    );
    assert!(
        gy.abilities[0].contains(
            "ManaSource::ColorsAmong { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::You) }"
        ),
        "{:?}",
        gy.abilities
    );

    let gate_type = read(
        "Name:GatePool\nTypes:Land\n\
         A:AB$ ManaReflected | Cost$ T | ColorOrType$ Type | Valid$ Gate.YouCtrl | ReflectProperty$ Produce",
    );
    assert!(
        gate_type.abilities[0].contains(
            "Effect::mana_from_lands(&Filter::HasSubtype(subtypes::land::GATE), true, true)"
        ),
        "{:?}",
        gate_type.abilities
    );

    let pain_pool = read(
        "Name:PainPool\nTypes:Land\n\
         A:AB$ ManaReflected | Cost$ T PayLife<1> | ColorOrType$ Type | Valid$ Land.YouCtrl | ReflectProperty$ Produce",
    );
    assert_eq!(
        pain_pool.abilities,
        [
            "mana_ability!(cost!(TapSelf, PayLife(1)), &[Effect::mana_from_lands(&Filter::ControlledByYou, true, true)])"
        ]
    );

    for bad in [
        "Name:X\nTypes:Artifact\nA:AB$ ManaReflected | Cost$ T | Valid$ Permanent.YouCtrl | ColorOrType$ Type | ReflectProperty$ Is",
        "Name:X\nTypes:Artifact\nA:AB$ ManaReflected | Cost$ T | Valid$ Defined.Untapped | ColorOrType$ Color | ReflectProperty$ Is",
        "Name:X\nTypes:Artifact\nA:AB$ ManaReflected | Cost$ T | Valid$ Defined.ValidGraveyard Card.OppOwn | ColorOrType$ Color | ReflectProperty$ Is",
        "Name:X\nTypes:Artifact\nA:AB$ ManaReflected | Cost$ T | Valid$ Permanent.YouCtrl | ColorOrType$ Color | ReflectProperty$ Is | Amount$ 2",
        "Name:X\nTypes:Artifact\nA:AB$ ManaReflected | Cost$ T | Valid$ Permanent.YouCtrl | ColorOrType$ Color | ReflectProperty$ Is | Unknown$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20a_mana_spend_riders_and_triggers_when_spent() {
    let uncounterable = read(
        "Name:HeroPlaza\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Legendary | AddsNoCounter$ True",
    );
    assert!(
        uncounterable.abilities[0].contains(".restricted(&SPEND1, SpendRider::Uncounterable)")
            && uncounterable
                .statics
                .contains("Filter::HasSupertype(SupertypeSet::LEGENDARY)"),
        "{:?} / {}",
        uncounterable.abilities,
        uncounterable.statics
    );

    let haste_perm = read(
        "Name:BanditHall\nTypes:Land\n\
         A:AB$ Mana | Cost$ T PayLife<3> | Produced$ C | AddsKeywords$ Haste | AddsKeywordsValid$ Spell.Creature",
    );
    assert_eq!(
        haste_perm.abilities,
        [
            "mana_ability!(cost!(TapSelf, PayLife(3)), &[Effect::mana(ManaColor::Colorless, 1).when_spent(&Filter::CREATURE, SpendRider::Haste)])"
        ]
    );

    let haste_eot = read(
        "Name:GloryArena\nTypes:Land\n\
         A:AB$ Mana | Cost$ R T Exert<1/CARDNAME/this land> | Produced$ R R | AddsKeywords$ Haste | AddsKeywordsValid$ Spell.Creature | AddsKeywordsUntil$ UntilEOT",
    );
    assert_eq!(
        haste_eot.abilities,
        [
            "mana_ability!(cost!(\"{R}\", TapSelf, ExertSelf), &[Effect::mana(ManaColor::Red, 2).when_spent(&Filter::CREATURE, SpendRider::HasteUntilEndOfTurn)])"
        ]
    );

    let ctr = read(
        "Name:CounterLand\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | AddsCounters$ Card.Creature_P1P1_1",
    );
    assert_eq!(
        ctr.abilities,
        [
            "mana_ability!(&[Effect::mana_of_any_color().when_spent(&Filter::CREATURE, SpendRider::EnterCounters(1))])"
        ]
    );

    let cmd_ctr = read(
        "Name:PalaceLand\nTypes:Land\n\
         A:AB$ Mana | Cost$ 1 T | Produced$ Combo ColorIdentity | AddsCounters$ Card.YouOwn+IsCommander_P1P1_ManaAddsCounterNum",
    );
    assert!(
        cmd_ctr.abilities[0].contains(".when_spent(&SPEND1, SpendRider::CommanderEnterCounters)"),
        "{:?}",
        cmd_ctr.abilities
    );

    let path_scry = read(
        "Name:AncestryPath\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo ColorIdentity | TriggersWhenSpent$ TrigScry\n\
         SVar:TrigScry:Mode$ SpellCast | ValidCard$ Creature.sharesCreatureTypeWith Commander | Execute$ DBScry\n\
         SVar:DBScry:DB$ Scry | ScryNum$ 1",
    );
    assert!(
        path_scry.abilities[0].contains(".when_spent(&SPEND1, SpendRider::Scry(1))")
            && path_scry
                .statics
                .contains("Filter::SharesSubtypeWithCommander"),
        "{:?} / {}",
        path_scry.abilities,
        path_scry.statics
    );

    let cmd_scry = read(
        "Name:HallStudy\nTypes:Land\n\
         A:AB$ Mana | Cost$ 1 T | Produced$ Any | TriggersWhenSpent$ TrigSpent\n\
         SVar:TrigSpent:Mode$ SpellCast | ValidCard$ Card.IsCommander+YouOwn | ValidActivatingPlayer$ You | Execute$ TrigScry\n\
         SVar:TrigScry:DB$ Scry | ScryNum$ X\n\
         SVar:X:TriggeredCard$CommanderCastFromCommandZone",
    );
    assert!(
        cmd_scry.abilities[0].contains(".when_spent(&SPEND1, SpendRider::CommanderScry)"),
        "{:?}",
        cmd_scry.abilities
    );

    let copy_mana = read(
        "Name:CopyLens\nManaCost:5\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ R | TriggersWhenSpent$ TrigCopy\n\
         SVar:TrigCopy:Mode$ SpellCast | ValidCard$ Instant.Red,Sorcery.Red | Execute$ TrigCopyMain\n\
         SVar:TrigCopyMain:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
    );
    assert!(
        copy_mana.abilities[0].contains(".when_spent(&SPEND1, SpendRider::Copy)"),
        "{:?}",
        copy_mana.abilities
    );

    for bad in [
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | AddsNoCounter$ True",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ G | AddsKeywords$ Riot | AddsKeywordsValid$ Spell.Creature",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ G | RestrictValid$ Spell.Creature | AddsKeywords$ Haste | AddsKeywordsValid$ Spell.Creature",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | AddsCounters$ Card.Creature_M1M1_1",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ G | TriggersWhenSpent$ T\nSVar:T:Mode$ SpellCast | ValidCard$ Creature | Execute$ G\nSVar:G:DB$ GainLife | LifeAmount$ 2",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ U | TriggersWhenSpent$ T\nSVar:T:Mode$ SpellAbilityCast | ValidSA$ Spell,Activated | Execute$ C\nSVar:C:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20a_mana_amount_for_chosen_color_identity_and_explicit_reasons() {
    let for_dyn = read(
        "Name:EachMana\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Mana | Defined$ Player | Produced$ G | Amount$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        for_dyn.abilities[0].contains("Effect::AddManaAmountFor { who: PlayerRel::EachPlayer, color: ManaColor::Green, amount: Amount::CountOf {"),
        "{:?}",
        for_dyn.abilities
    );

    let chosen_two = read(
        "Name:DoubleChosen\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Chosen | Amount$ 2",
    );
    assert!(
        chosen_two.abilities[0].contains("ManaSource::Chosen, amount: Amount::Fixed(2)"),
        "{:?}",
        chosen_two.abilities
    );

    let chosen_devotion = read(
        "Name:DevotionChosen\nManaCost:4\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ Chosen | Amount$ X\n\
         SVar:X:Count$Devotion.Chosen",
    );
    assert!(
        chosen_devotion.abilities[0]
            .contains("ManaSource::Chosen, amount: Amount::DevotionToManaColor"),
        "{:?}",
        chosen_devotion.abilities
    );

    let cmd_two = read(
        "Name:DoubleTower\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo ColorIdentity | Amount$ 2",
    );
    assert!(
        cmd_two.abilities[0].contains(
            "ManaSource::CommanderIdentity, amount: Amount::Fixed(2), distribution: ManaDistribution::AnyCombination"
        ),
        "{:?}",
        cmd_two.abilities
    );

    // Every refusal below must record an explicit non-empty reason (no silent None).
    for bad in [
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W Chosen | Amount$ 2",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | IsPresent$ Card.Self+counters_GE2_NOSUCHCOUNTER",
        "Name:X\nManaCost:2 U\nTypes:Instant\nA:SP$ Charm | Choices$ M1,M2 | CharmNum$ X | CanRepeatModes$ True\nSVar:X:Count$xPaid\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record an explicit refusal reason: {bad}"
        );
    }
}

#[test]
fn wave20a_nickname_self_costs_multi_return_exile_xmin_param_and_life_total_drop() {
    let nick_costs = read(
        "Name:NickBot\nManaCost:2\nTypes:Artifact Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ Sac<1/NICKNAME> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ Exile<1/NICKNAME/this creature> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ Exert<1/NICKNAME> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ Return<1/NICKNAME> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ ExileFromTop<2/Card.YouOwn> | NumCards$ 1",
    );
    assert_eq!(
        nick_costs.abilities,
        [
            "activated!(cost!(SacrificeSelf), &[Effect::draw(1)])",
            "activated!(cost!(ExileSelf), &[Effect::draw(1)])",
            "activated!(cost!(ExertSelf), &[Effect::draw(1)])",
            "activated!(cost!(ReturnSelfToHand), &[Effect::draw(1)])",
            "activated!(cost!(ExileTopOfLibrary(2)), &[Effect::draw(1)])",
        ]
    );

    let multi_ret_exile = read(
        "Name:MultiCost\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ Return<2/Land> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ Exile<2/Artifact> | NumCards$ 1",
    );
    assert_eq!(
        multi_ret_exile.abilities,
        [
            "activated!(cost!(ReturnToHand(&Filter::YOUR_LAND), ReturnToHand(&Filter::YOUR_LAND)), &[Effect::draw(1)])",
            "activated!(cost!(ExilePermanent(&Filter::YOUR_ARTIFACT), ExilePermanent(&Filter::YOUR_ARTIFACT)), &[Effect::draw(1)])",
        ]
    );

    let xmin_param = read(
        "Name:ParamXMin\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ X | XMin$ 1 | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(
        xmin_param.abilities,
        [
            "activated!(cost!(\"{X}\"), &[Effect::DrawCards { amount: Amount::X }], x_value = ActivationX::AtLeast(1))"
        ]
    );

    let life_and_gy = read(
        "Name:DropKeyFix\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | LifeTotal$ You | LifeAmount$ GE30\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | IsPresent$ Card | PresentZone$ Graveyard | PresentPlayer$ You | PresentCompare$ GE5",
    );
    assert!(
        life_and_gy.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), 30)")
            && life_and_gy.abilities[1].contains(
                "Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::Any, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, 5)"
            ),
        "{:?}",
        life_and_gy.abilities
    );

    for bad in [
        "Name:X\nTypes:Instant\nA:SP$ Draw | Cost$ X | XMin$ 1 | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | XMin$ 1 | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ XMin1 X | XMin$ 1 | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 2 | XMin$ 1 | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ X | XMin$ 0 | NumCards$ X\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave21a_remove_any_counter_fixed_and_variable_x() {
    let fixed = read(
        "Name:CounterSpender\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/CARDNAME/this creature> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ RemoveAnyCounter<2/Any/NICKNAME> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ 1 RemoveAnyCounter<3/Any/Permanent> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ RemoveAnyCounter<2/P1P1/Creature> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ RemoveAnyCounter<3/Any/Artifact.Other;Creature.Other;Planeswalker.Other/other permanents> | NumCards$ 1",
    );
    assert_eq!(
        fixed.abilities[0],
        "activated!(cost!(RemoveAnyCounter { count: 1, kind: None, filter: &Filter::This }), &[Effect::draw(1)])"
    );
    assert_eq!(
        fixed.abilities[1],
        "activated!(cost!(RemoveAnyCounter { count: 2, kind: None, filter: &Filter::This }), &[Effect::draw(1)])"
    );
    assert_eq!(
        fixed.abilities[2],
        "activated!(cost!(\"{1}\", RemoveAnyCounter { count: 3, kind: None, filter: &Filter::Any }), &[Effect::draw(1)])"
    );
    assert_eq!(
        fixed.abilities[3],
        "activated!(cost!(RemoveAnyCounter { count: 2, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }), &[Effect::draw(1)])"
    );
    assert!(
        fixed.abilities[4].contains("RemoveAnyCounter { count: 3, kind: None, filter: &COST1 }"),
        "{:?}",
        fixed.abilities[4]
    );

    let var_x = read(
        "Name:CounterCannon\nManaCost:3\nTypes:Artifact\n\
         A:AB$ DealDamage | Cost$ T RemoveAnyCounter<X/P1P1/Creature> | ValidTgts$ Any | NumDmg$ X\n\
         A:AB$ Draw | Cost$ RemoveAnyCounter<X/LOYALTY/Planeswalker> | NumCards$ X\n\
         A:AB$ Mana | Cost$ RemoveAnyCounter<X1+/P1P1/Artifact> | XMin$ 1 | Produced$ C | Amount$ X\n\
         A:AB$ Draw | Cost$ XMin1 1 G RemoveAnyCounter<X1+/P1P1/Creature> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        var_x.abilities[0].contains(
            "RemoveAnyCounterX { kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }"
        ),
        "{:?}",
        var_x.abilities[0]
    );
    assert!(
        var_x.abilities[1].contains(
            "RemoveAnyCounterX { kind: Some(CounterKind::Loyalty), filter: &Filter::PLANESWALKER }"
        ),
        "{:?}",
        var_x.abilities[1]
    );
    assert_eq!(
        var_x.abilities[2],
        "mana_ability!(cost!(RemoveAnyCounterX { kind: Some(CounterKind::P1P1), filter: &Filter::ARTIFACT }), &[Effect::mana_dynamic(ManaColor::Colorless, Amount::X)], x_value = ActivationX::AtLeast(1))"
    );
    assert!(
        var_x.abilities[3].contains("cost!(\"{1}{G}\", RemoveAnyCounterX { kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE })")
            && var_x.abilities[3].contains("x_value = ActivationX::AtLeast(1)"),
        "{:?}",
        var_x.abilities[3]
    );

    let sub_any = read(
        "Name:AnySub\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ SubCounter<1/Any/CARDNAME> | NumCards$ 1\n\
         A:AB$ Draw | Cost$ SubCounter<X/Any/Artifact.YouCtrl;Creature.YouCtrl/an artifact or creature you control> | NumCards$ X\n\
         A:AB$ Mana | Cost$ T SubCounter<X1+/DREAM/NICKNAME> | XMin$ 1 | Produced$ U | Amount$ X\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(
        sub_any.abilities[0],
        "activated!(cost!(RemoveAnyCounter { count: 1, kind: None, filter: &Filter::This }), &[Effect::draw(1)])"
    );
    assert_eq!(
        sub_any.abilities[1],
        "activated!(cost!(RemoveAnyCounterX { kind: None, filter: &Filter::ARTIFACT_OR_CREATURE }), &[Effect::DrawCards { amount: Amount::X }])"
    );
    assert_eq!(
        sub_any.abilities[2],
        "mana_ability!(cost!(TapSelf, RemoveCounterSelfX { kind: counters::DREAM }), &[Effect::mana_dynamic(ManaColor::Blue, Amount::X)], x_value = ActivationX::AtLeast(1))"
    );
}

#[test]
fn wave21a_remove_any_counter_refusals() {
    for bad in [
        // Loyalty removal from self stays restricted to Planeswalker$ True.
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/LOYALTY/CARDNAME> | NumCards$ 1",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/LOYALTY/Card.Self> | NumCards$ 1",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ SubCounter<1/LOYALTY/Card.Self> | NumCards$ 1",
        // Invalid counts, non-controller/non-battlefield filters, unannounced X.
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<0/Any/Permanent> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<256/Any/Permanent> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/Creature.OppCtrl> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/Card.YouOwn+inZoneExile+suspended> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<X/P1P1/Creature> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<0/P1P1/Creature.YouCtrl> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<256/P1P1/Creature.YouCtrl> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1/Creature.OppCtrl> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1/Creature.YouCtrl/desc/Exile> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<All/AIM/OriginalHost/Host> | NumCards$ 1",
        // Conflicting XMin vs X1+, or self counter removal outside battlefield.
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ XMin2 RemoveAnyCounter<X1+/P1P1/Creature> | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<X1+/P1P1/Creature> | XMin$ 2 | NumCards$ X\nSVar:X:Count$xPaid",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/CARDNAME> | ActivationZone$ Graveyard | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/CARDNAME> | ActivationZone$ Hand | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1/Card.Self> | ActivationZone$ Graveyard | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1> | ActivationZone$ Hand | NumCards$ 1",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record an explicit refusal reason: {bad}"
        );
    }
}

#[test]
fn wave21a_monarch_initiative_and_completed_dungeon_conditions() {
    let named = read(
        "Name:CourtKeeper\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | Condition$ Monarch\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | Condition$ Initiative\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | Condition$ CompletedDungeon\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckDefinedPlayer$ You.isMonarch\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckDefinedPlayer$ You.hasInitiative\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckDefinedPlayer$ Opponent.isMonarch\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckDefinedPlayer$ You.committedCrimeThisTurn",
    );
    assert_eq!(
        named.abilities,
        [
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::IsMonarch(PlayerRel::You)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::HasInitiative(PlayerRel::You)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::CompletedADungeon(PlayerRel::You)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::IsMonarch(PlayerRel::You)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::HasInitiative(PlayerRel::You)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::IsMonarch(PlayerRel::EachOpponent)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::CommittedCrimeThisTurn(PlayerRel::You)))",
        ]
    );

    let svar_conds = read(
        "Name:CrownAndDelve\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ M | SVarCompare$ EQ1\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ AnyM | SVarCompare$ EQ0\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ OppI | SVarCompare$ GE1\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ D | SVarCompare$ GE1\n\
         A:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ D | SVarCompare$ EQ0\n\
         SVar:M:Count$Monarch.1.0\n\
         SVar:AnyM:PlayerCountPlayers$HasPropertyisMonarch\n\
         SVar:OppI:PlayerCountOpponents$HasPropertyhasInitiative\n\
         SVar:D:DungeonsCompleted$Amount",
    );
    assert_eq!(
        svar_conds.abilities,
        [
            "activated!(cost!(\"{1}\"), &[Effect::draw(1)], condition = Some(Condition::IsMonarch(PlayerRel::You)))",
            "activated!(cost!(\"{1}\"), &[Effect::draw(1)], condition = Some(Condition::Not(&Condition::IsMonarch(PlayerRel::EachPlayer))))",
            "activated!(cost!(\"{1}\"), &[Effect::draw(1)], condition = Some(Condition::HasInitiative(PlayerRel::EachOpponent)))",
            "activated!(cost!(\"{1}\"), &[Effect::draw(1)], condition = Some(Condition::CompletedADungeon(PlayerRel::You)))",
            "activated!(cost!(\"{1}\"), &[Effect::draw(1)], condition = Some(Condition::Not(&Condition::CompletedADungeon(PlayerRel::You))))",
        ]
    );

    for bad in [
        // Exact/multi dungeon counts cannot be lowered to boolean CompletedADungeon.
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ D | SVarCompare$ EQ1\nSVar:D:DungeonsCompleted$Amount",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckSVar$ D | SVarCompare$ GE2\nSVar:D:DungeonsCompleted$Amount",
        // Unknown CheckDefinedPlayer must not be silently dropped.
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | CheckDefinedPlayer$ You.withMostLife",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ 1 | NumCards$ 1 | Condition$ Monarch | CheckDefinedPlayer$ You.withMostLife",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave22a_subcounter_non_any_costs_and_refusals() {
    let body = read(
        "Name:CounterChanneler\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Pump | Cost$ BG SubCounter<1/M1M1/Creature/a creature you control> | NumAtt$ +3 | NumDef$ +3\n\
         A:AB$ Draw | Cost$ 2 U SubCounter<1/P1P1/Creature.YouCtrl/a creature you control> | NumCards$ 1\n\
         A:AB$ DealDamage | Cost$ 1 R SubCounter<1/CHARGE/Permanent/permanent you control/Battlefield> | ValidTgts$ Any | NumDmg$ 2\n\
         A:AB$ Draw | Cost$ SubCounter<X/P1P1/Creature.YouCtrl> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{B/G}\", RemoveAnyCounter { count: 1, kind: Some(CounterKind::M1M1), filter: &Filter::CREATURE }), &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(3), toughness: Amount::Fixed(3), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])",
            "activated!(cost!(\"{2}{U}\", RemoveAnyCounter { count: 1, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }), &[Effect::draw(1)])",
            "activated!(cost!(\"{1}{R}\", RemoveAnyCounter { count: 1, kind: Some(CounterKind::Charge), filter: &Filter::Any }), &[Effect::DealDamage { amount: Amount::Fixed(2), target: TargetSpec::AnyTarget }], target = Some(TargetSpec::AnyTarget))",
            "activated!(cost!(RemoveAnyCounterX { kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }), &[Effect::DrawCards { amount: Amount::X }])",
        ]
    );

    for bad in [
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1/Creature.OppCtrl/an opponent's creature> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/TIME/Permanent.inZoneBattlefield;Card.suspended/a permanent or suspended card/Battlefield,Exile> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1/Creature.YouCtrl/desc/Exile> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/Any/Permanent/desc/Battlefield,Exile> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ RemoveAnyCounter<1/Any/Permanent/desc/Exile> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<0/P1P1/Creature.YouCtrl> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<256/P1P1/Creature.YouCtrl> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<All/AIM/OriginalHost/Host> | NumCards$ 1",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Draw | Cost$ SubCounter<1/LOYALTY/Card.Self> | NumCards$ 1",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ SubCounter<1/P1P1/Creature.YouCtrl/a/Battlefield/extra> | NumCards$ 1",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record refusal reason: {bad}"
        );
    }
}

#[test]
fn wave22a_mana_priority_only_zones_multi_restrict_and_graveyard_battlefield_count() {
    let led = read(
        "Name:PriorityDiamond\nManaCost:0\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ Sac<1/CARDNAME> Discard<0/Hand> | Produced$ Any | Amount$ 3 | InstantSpeed$ True",
    );
    assert_eq!(
        led.abilities,
        [
            "mana_ability!(cost!(SacrificeSelf, DiscardHand), &[Effect::mana_choice_dynamic(ALL_MANA_COLORS, Amount::Fixed(3))], timing = ActivationTiming::PriorityOnly)"
        ]
    );

    let zone_mana = read(
        "Name:ZoneGuide\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         A:AB$ Mana | Cost$ ExileFromHand<1/CARDNAME> | Produced$ G | ActivationZone$ Hand\n\
         A:AB$ Mana | Cost$ 1 ExileFromGrave<1/CARDNAME> | Produced$ Any | ActivationZone$ Graveyard",
    );
    assert_eq!(
        zone_mana.abilities,
        [
            "mana_ability!(cost!(ExileSelf), &[Effect::mana(ManaColor::Green, 1)], zone = ActivationZone::Hand)",
            "mana_ability!(cost!(\"{1}\", ExileSelf), &[Effect::mana_of_any_color()], zone = ActivationZone::Graveyard)",
        ]
    );

    let multi_restrict = read(
        "Name:TriSpellRock\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ T | Produced$ U B R | RestrictValid$ Spell.Instant,Spell.Sorcery\n\
         A:AB$ Mana | Cost$ T | Produced$ C U | RestrictValid$ Activated\n\
         A:AB$ Mana | Cost$ T | Produced$ W U | RestrictValid$ Spell.Creature,Activated.Artifact",
    );
    assert_eq!(
        multi_restrict.abilities,
        [
            "mana_ability!(&[Effect::mana(ManaColor::Blue, 1).restricted(&Filter::INSTANT_OR_SORCERY, SpendRider::None), Effect::mana(ManaColor::Black, 1).restricted(&Filter::INSTANT_OR_SORCERY, SpendRider::None), Effect::mana(ManaColor::Red, 1).restricted(&Filter::INSTANT_OR_SORCERY, SpendRider::None)])",
            "mana_ability!(&[Effect::mana(ManaColor::Colorless, 1).restricted_for(&Filter::Any, SpendRider::None, ManaUse::Abilities), Effect::mana(ManaColor::Blue, 1).restricted_for(&Filter::Any, SpendRider::None, ManaUse::Abilities)])",
            "mana_ability!(&[Effect::mana(ManaColor::White, 1).restricted_for(&Filter::CREATURE, SpendRider::None, ManaUse::SpellsOrAbilities).for_abilities_matching(&Filter::ARTIFACT), Effect::mana(ManaColor::Blue, 1).restricted_for(&Filter::CREATURE, SpendRider::None, ManaUse::SpellsOrAbilities).for_abilities_matching(&Filter::ARTIFACT)])",
        ]
    );

    let cave_count = read(
        "Name:CaveTally\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$ValidGraveyard,Battlefield Creature.YouCtrl",
    );
    assert!(
        cave_count.abilities[0].contains("Amount::Sum")
            && cave_count.abilities[0].contains("ZoneSel::Battlefield")
            && cave_count.abilities[0].contains("ZoneSel::GraveyardOf(PlayerRel::You)"),
        "{:?}",
        cave_count.abilities
    );

    for bad in [
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ T | NumCards$ 1 | InstantSpeed$ True",
        "Name:X\nTypes:Artifact\nA:AB$ Mana | Cost$ T | Produced$ G | SorcerySpeed$ True | InstantSpeed$ True",
        "Name:X\nTypes:Planeswalker\nLoyalty:3\nA:AB$ Mana | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Produced$ G | InstantSpeed$ True",
        "Name:X\nManaCost:U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | InstantSpeed$ True",
        "Name:X\nTypes:Artifact\nA:AB$ Mana | Cost$ T | Produced$ G | InstantSpeed$ False",
        "Name:X\nTypes:Artifact\nA:AB$ Mana | Cost$ T | Produced$ G | ActivationZone$ Command",
        "Name:X\nTypes:Artifact\nA:AB$ Mana | Cost$ T | Produced$ G U | AddsKeywords$ Haste | AddsKeywordsValid$ Spell.Creature",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record refusal reason: {bad}"
        );
    }
}

#[test]
fn wave22a_present_hand_library_desert_and_disjunctive() {
    let hand_checks = read(
        "Name:HandOracle\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card.YouOwn | PresentCompare$ EQ7\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card.YouCtrl | PresentCompare$ EQ0\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card.YouCtrl | PresentCompare$ LE3\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card.YouOwn | PresentCompare$ GE7\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card | PresentPlayer$ You\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Land.YouOwn | PresentCompare$ EQ0",
    );
    assert_eq!(
        hand_checks.abilities,
        [
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::HandSizeExactly(7)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::HandSizeAtMost(0)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::HandSizeAtMost(3)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), 7)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), 1)))",
            "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::Not(&Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::LAND, zone: ZoneSel::HandYou }, 1))))",
        ]
    );

    let library_checks = read(
        "Name:LibraryOracle\nManaCost:3 U U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | IsPresent$ Card.YouCtrl | PresentZone$ Library | PresentCompare$ GE200 | Execute$ Win\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | IsPresent$ Land.YouCtrl+nonBasic | PresentZone$ Library | PresentCompare$ EQ0 | Execute$ Ping\n\
         SVar:Win:DB$ WinsGame | Defined$ You\n\
         SVar:Ping:DB$ Draw | NumCards$ 1",
    );
    assert!(
        library_checks.abilities[0]
            .contains("Condition::AmountAtLeast(Amount::LibrarySize(PlayerRel::You), 200)"),
        "{:?}",
        library_checks.abilities
    );
    assert!(
        library_checks.abilities[1].contains("ZoneSel::LibraryYou"),
        "{:?}",
        library_checks.abilities
    );

    let desert_and_verge = read(
        "Name:DesertSanctuary\nTypes:Land\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Battlefield,Graveyard | IsPresent$ Land | PresentPlayer$ You\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Battlefield,Graveyard | IsPresent$ Land.YouCtrl | PresentCompare$ GE2\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo G W | IsPresent$ Card.Self+ThisTurnEntered,Land.Basic+YouCtrl\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | ActivationPhases$ Upkeep->Main1,Main2->Cleanup\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ Gy | SVarCompare$ LT7\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | CheckSVar$ Lib | SVarCompare$ GE50\n\
         SVar:Gy:Count$ValidGraveyard Card.YouOwn\n\
         SVar:Lib:Count$CardsInYourLibrary",
    );
    assert!(
        desert_and_verge.abilities[0]
            .contains("Condition::Any(&[Condition::ControlCount(&Filter::LAND, 1), Condition::AmountAtLeast(Amount::CountOf { filter: &Filter::LAND, zone: ZoneSel::GraveyardOf(PlayerRel::You) }, 1)])"),
        "{:?}",
        desert_and_verge.abilities[0]
    );
    assert!(
        desert_and_verge.abilities[1].contains("Amount::Sum")
            && desert_and_verge.abilities[1].contains("ZoneSel::Battlefield")
            && desert_and_verge.abilities[1].contains("ZoneSel::GraveyardOf(PlayerRel::You)"),
        "{:?}",
        desert_and_verge.abilities[1]
    );
    assert!(
        desert_and_verge.abilities[2].contains("Condition::Any(&[Condition::SourceMatches(")
            && desert_and_verge.abilities[2].contains("Condition::ControlCount("),
        "{:?}",
        desert_and_verge.abilities[2]
    );
    assert_eq!(
        desert_and_verge.abilities[3],
        "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::Not(&Condition::DuringCombat)))"
    );
    assert_eq!(
        desert_and_verge.abilities[4],
        "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::Not(&Condition::GraveyardCountAtLeast(7))))"
    );
    assert_eq!(
        desert_and_verge.abilities[5],
        "activated!(Cost::TAP, &[Effect::draw(1)], condition = Some(Condition::AmountAtLeast(Amount::LibrarySize(PlayerRel::You), 50)))"
    );

    for bad in [
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card.DefenderCtrl",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Hand | IsPresent$ Card.ControlledBy TriggeredPlayer",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Library | IsPresent$ Card.StrictlySelf",
        "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ T | NumCards$ 1 | PresentZone$ Battlefield,Graveyard | IsPresent$ Land.OppCtrl",
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ G | IsPresent$ Card.Self+ThisTurnEntered,Land.Basic+YouCtrl | PresentCompare$ GE2",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record refusal reason: {bad}"
        );
    }
}

#[test]
fn wave22a_spell_turn_and_phase_restrictions() {
    let spell_windows = read(
        "Name:CombatWindow\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | ActivationPhases$ BeginCombat->EndCombat",
    );
    assert_eq!(
        spell_windows.abilities,
        ["spell!(&[Effect::draw(1)], condition = Some(Condition::DuringCombat))"]
    );

    let spell_turn_and_step = read(
        "Name:EndStepStudy\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | PlayerTurn$ True | ActivationPhases$ End of Turn",
    );
    assert_eq!(
        spell_turn_and_step.abilities,
        [
            "spell!(&[Effect::draw(1)], condition = Some(Condition::All(&[Condition::YourTurn, Condition::DuringStep(StepKind::End)])))"
        ]
    );

    let spell_before_blockers = read(
        "Name:BeforeBlockers\nManaCost:R\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | ActivationPhases$ BeginCombat->Declare Attackers",
    );
    assert_eq!(
        spell_before_blockers.abilities,
        [
            "spell!(&[Effect::draw(1)], condition = Some(Condition::Any(&[Condition::DuringStep(StepKind::CombatBegin), Condition::DuringStep(StepKind::DeclareAttackers)])))"
        ]
    );

    let spell_after_combat = read(
        "Name:AfterCombatSpell\nManaCost:U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | OpponentTurn$ True | ActivationPhases$ Main2->Cleanup",
    );
    assert_eq!(
        spell_after_combat.abilities,
        [
            "spell!(&[Effect::draw(1)], condition = Some(Condition::All(&[Condition::OpponentsTurn, Condition::AfterCombat])))"
        ]
    );

    for bad in [
        "Name:X\nManaCost:U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | IsPresent$ Island.YouCtrl",
        "Name:X\nManaCost:U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | Activation$ Threshold",
        "Name:X\nManaCost:U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | CheckSVar$ X\nSVar:X:Count$Valid Creature.YouCtrl",
        "Name:X\nManaCost:U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | PlayerTurn$ True | OpponentTurn$ True",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record refusal reason: {bad}"
        );
    }
}

#[test]
fn wave22a_explicit_refusal_reasons_no_silent_none() {
    for bad in [
        // Divided damage with unsupported shape must record a refusal reason.
        "Name:X\nManaCost:2 R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | DividedAsYouChoose$ 3 | SubAbility$ Extra\nSVar:Extra:DB$ Draw | NumCards$ 1",
        // Unreadable A:SP$ / A:AB$ / Charm mode chains must record a refusal reason.
        "Name:X\nManaCost:1 U\nTypes:Instant\nA:SP$ GainControl | Defined$ Self | NewController$ Opponent",
        "Name:X\nManaCost:1 U\nTypes:Artifact\nA:AB$ Mana | Cost$ T | Produced$ Any | SubAbility$ Extra\nSVar:Extra:DB$ GainControl | Defined$ Self | NewController$ Opponent",
        "Name:X\nManaCost:1 U\nTypes:Instant\nA:SP$ Charm | Choices$ M1,M2\nSVar:M1:DB$ Draw | NumCards$ 1\nSVar:M2:DB$ GainControl | Defined$ Self | NewController$ Opponent",
    ] {
        let parsed = parse(bad);
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "must refuse: {bad}"
        );
        assert!(
            refusal_reason(&parsed, &cats(), None).is_some(),
            "must record refusal reason (no silent None): {bad}"
        );
    }
}
