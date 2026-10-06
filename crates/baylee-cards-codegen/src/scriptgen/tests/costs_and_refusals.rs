//! Reading amounts and costs, statics on the battlefield, and saying why a
//! script is refused.

use super::*;

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

    let elsewhere = parse(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ B | Amount$ X | SpellDescription$ Add.\n\
         SVar:X:Count$ValidGraveyard Creature.Black+YouCtrl\n",
    );
    assert!(transcode(&elsewhere, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&elsewhere, &cats(), None).as_deref(),
        Some("count `Count$ValidGraveyard Creature.Black+YouCtrl`"),
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

    // "Add two mana of different colors" is a third sentence again, and
    // the DSL has no room for "different" — so it is refused by the word
    // the corpus writes rather than by the line it sits on.
    let different = parse(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ 1 T | Produced$ Combo AnyDifferent | Amount$ 2 | \
         SpellDescription$ Add two mana of different colors.\n",
    );
    assert!(transcode(&different, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&different, &cats(), None).as_deref(),
        Some("`Mana` combination over `AnyDifferent`")
    );
}

/// `Cost$ UR T` is the filter cycle's price, and the letters run together.
///
/// Three things are asserted rather than one, because a rule that saw
/// "two letters" would get two of them wrong:
///
/// - the pair keeps the **printed order**, `{U/R}` and not `{R/U}`;
/// - `2W` and `WP` are two letters and neither is a colour pair — a
///   `{2/W}` costs two generic as its other half, a `{W/P}` is paid with
///   life — so both keep refusing **by name**;
/// - a doubled pair is not a symbol at all, and `ColorPair::new` asserts
///   it, so `WW` has to be refused here rather than turned into a panic
///   at the card's compile time. No reference script writes one.
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

    for token in ["2W", "WP", "WW", "WUB"] {
        let script = parse(&format!(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ {token} T | Produced$ U | SpellDescription$ Add {{U}}.\n"
        ));
        assert!(
            transcode(&script, &cats(), None).is_none(),
            "`{token}` is not a colour pair"
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
        ("ExileFromGrave<3/Card>", "a cost naming `3` objects"),
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

    let svars = BTreeMap::new();
    let cats = cats();
    let mut tx = Tx {
        svars: &svars,
        cats: &cats,
        tokens: None,
        has_x: false,
        on_a_spell: false,
        trigger_mode: None,
        block_line: None,
        block_half: None,
        in_delayed: false,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
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

/// A loyalty cost is not an ordinary counter cost. 413 scripts in the
/// corpus print `Cost$ AddCounter<n/LOYALTY>`, and reading one as a
/// `PutCounterSelf` would make a planeswalker's `+1` an ability anybody
/// may activate at instant speed as often as they like — CR 606.3 allows
/// it once a turn and only when a sorcery could be cast, and
/// `activated!` says neither.
///
/// Nothing in `cost_expr` knows that. What refuses the card is
/// `Planeswalker$ True`, which sits on every loyalty ability and is
/// claimed by no rule, so the refusal happens before the cost is read.
/// That makes this test a tripwire as much as a check: a rule that
/// claims the key later has to answer the loyalty question in the same
/// commit, or this fails.
#[test]
fn a_loyalty_cost_does_not_become_an_ordinary_counter_cost() {
    let script = "Name:X\nManaCost:2 W W\nTypes:Legendary Planeswalker Ajani\nLoyalty:4\n\
                  A:AB$ PutCounter | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | \
                  CounterType$ P1P1 | CounterNum$ 1 | ValidTgts$ Creature";
    assert!(refused(script));
    assert_eq!(
        refusal_reason(&parse(script), &cats(), None).as_deref(),
        Some("unclaimed parameter `PutCounter.Planeswalker`")
    );
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

    assert!(refused(
        "Name:X\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +0 | NumDef$ +0 | \
         KW$ HIDDEN CARDNAME can't block."
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
    // Setting only one half of P/T is a real card ("base power 4") that
    // `SetPT` cannot express.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | SetPower$ 4\n"
    ));
    // A condition is a rule of its own; one it cannot read refuses.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | \
         IsPresent$ Island.YouCtrl | PresentCompare$ EQ2\n"
    ));
    // A mode that is not Continuous is not this rule.
    assert!(refused(
        "Name:X\nTypes:Creature\nS:Mode$ CantBlock | ValidCard$ Card.Self\n"
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
    // Gaea's Might: domain, which the DSL cannot count. 207 scripts in
    // the reference pump by `X` and every one of them defines `SVar:X`;
    // only 44 define it as the number the player announced.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
         SVar:X:Count$Domain\n"
    ));
    // Oboro Envoy: a count of a **zone**, and negative, so the sign is
    // not what makes the difference. `Count$ValidHand` is one letter away
    // from the `Count$Valid ` the reader answers and is a different
    // question; a `strip_prefix` without its trailing space would take
    // this one and count the battlefield.
    assert!(refused(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
         SVar:X:Count$ValidHand Card.YouOwn\n"
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
         SVar:X:Count$Domain\n",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("pump amount `+X` = `Count$Domain`")
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
    let script = parse("Name:X\nTypes:Creature\nS:Mode$ CantBlock | ValidCard$ Card.Self");
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("static ability `S: Mode$ CantBlock`")
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
            "Name:X\nTypes:Creature\nA:AB$ Draw | Cost$ Discard<2/Card> | NumCards$ 1",
            "a cost naming `2` objects",
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
             T:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "trigger at step `Main1`",
        ),
        (
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Exile | \
             ValidCard$ Card.Self | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "trigger on a move from Battlefield to Exile",
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
             SVar:Second:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 1",
            "two different targets in one chain",
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
        // A return cost carries one permanent, so a count above one is
        // refused by that count rather than paid one short. Fourteen of
        // the corpus's 78 return costs print two, three or X.
        (
            "Name:X\nTypes:Creature\n\
             A:AB$ Untap | Cost$ Return<2/Forest> | ValidTgts$ Creature",
            "a cost naming `2` objects",
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
