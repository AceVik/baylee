//! Reading keywords, targets, triggers and the "may" a line carries.

use super::*;

/// Current CR 605.1a permits damage riders but excludes library movement.
#[test]
fn mana_classification_distinguishes_damage_from_library_riders() {
    // A painland: mana, and a rider that names a player without
    // targeting one.
    let pain = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo W U | SubAbility$ DBDmg | SpellDescription$ Add {W} or {U}.\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ You | NumDmg$ 1",
    );
    assert_eq!(
        pain.abilities,
        [
            "mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue]), \
          Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::You) }])"
        ]
    );

    // Chromatic Sphere: mana and a draw, off a cost that is not a bare
    // tap — so the long form of the macro, with the cost written out.
    let sphere = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ Mana | Cost$ 1 T Sac<1/CARDNAME> | Produced$ Any | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert_eq!(
        sphere.abilities,
        ["activated!(cost!(\"{1}\", TapSelf, SacrificeSelf), \
          &[Effect::mana_of_any_color(), Effect::draw(1)])"]
    );

    // And the other side of the line, which is where CR 605.1a draws
    // it: an ability that **targets** is not a mana ability however much
    // mana it makes, so its mana goes on the stack like anything else.
    let targeted = read(
        "Name:X\nTypes:Creature Elf\nPT:1/2\n\
         A:AB$ Mana | Cost$ T | Produced$ G | ValidTgts$ Creature | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap",
    );
    assert_eq!(
        targeted.abilities,
        [
            "activated!(Cost::TAP, &[Effect::mana(ManaColor::Green, 1), Effect::TapTarget], \
          target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );
}

#[test]
fn random_discard_reads_x_and_refuses_a_discard_it_cannot_name() {
    let generated = read(
        "Name:Test
ManaCost:X B
Types:Sorcery
A:SP$ Discard | ValidTgts$ Player | NumCards$ X | Mode$ Random
SVar:X:Count$xPaid",
    );
    assert!(
        generated
            .abilities
            .iter()
            .any(|a| a.contains("Effect::DiscardRandom")
                && a.contains("Amount::X")
                && a.contains("PlayerRel::Chosen")),
        "{generated:?}"
    );
    // `Mode$ TgtChoose` is read since the discarding player's own choice
    // has a rule (`a_chosen_discard_on_your_turn_only`).
    for extra in ["", " | Mode$ Random | RevealNumber$ 2"] {
        assert!(refused(&format!(
            "Name:Test\nManaCost:B\nTypes:Sorcery\nA:SP$ Discard | ValidTgts$ Player | NumCards$ 1{extra}"
        )));
    }
}

/// `K:ETBReplacement:Other:<svar>` is a **pointer**, and the rule is what
/// it points at.
///
/// Reading the keyword alone would say "as this enters, something", which
/// is why the exclusion and the colour the tap makes are both asserted
/// here: the two halves of these lands only work as a pair, and a card
/// that chose a colour nothing read would be a land that taps for
/// nothing.
#[test]
fn an_as_enters_colour_choice_is_read_together_with_what_taps_for_it() {
    let plain = read(
        "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
         SVar:CC:DB$ ChooseColor | Defined$ You | AILogic$ MostProminentInComputerDeck | SpellDescription$ As CARDNAME enters, choose a color.\n\
         A:AB$ Mana | Cost$ T | Produced$ Chosen | SpellDescription$ Add one mana of the chosen color.",
    );
    assert_eq!(plain.enter_modifiers, ["EnterModifier::ChooseColor"]);
    assert_eq!(plain.abilities, ["mana_ability!(&[Effect::mana_chosen()])"]);

    // Thriving Heath: the colour it may not be told to make is the one
    // it always makes anyway.
    let except = read(
        "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
         SVar:CC:DB$ ChooseColor | Defined$ You | Exclude$ white | SpellDescription$ As CARDNAME enters, choose a color other than white.\n\
         A:AB$ Mana | Cost$ T | Produced$ Combo W Chosen | SpellDescription$ Add {W} or one mana of the chosen color.",
    );
    assert_eq!(
        except.enter_modifiers,
        ["EnterModifier::ChooseColorExcept(ManaColor::White)"]
    );
    assert_eq!(
        except.abilities,
        ["mana_ability!(&[Effect::mana_chosen_or(&[ManaColor::White])])"]
    );

    // A clone that must copy is not the "may" `CopyOnEnter` asks.
    assert!(refused(
        "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\nK:ETBReplacement:Copy:CC\n\
         SVar:CC:DB$ Clone | Defined$ You"
    ));
    // Clone and Copy Artifact: the choice is made before it enters, so
    // `Other` names nothing, and the except-clause is a copy mod.
    let clone = read(
        "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\n\
         K:ETBReplacement:Copy:CC:Optional\n\
         SVar:CC:DB$ Clone | Choices$ Creature.Other | SpellDescription$ x",
    );
    assert_eq!(
        clone.abilities,
        ["AbilityDef::CopyOnEnter { target: TargetSpec::Object(&Filter::CREATURE), mods: &[] }"]
    );
    let artifact = read(
        "Name:X\nTypes:Artifact\n\
         K:ETBReplacement:Copy:CC:Optional\n\
         SVar:CC:DB$ Clone | Choices$ Artifact.Other | AddTypes$ Enchantment",
    );
    assert!(
        artifact.abilities[0].contains("mods: &[CopyMod::AddType(TypeSet::ENCHANTMENT)]"),
        "{:?}",
        artifact.abilities
    );
    // Vesuvan Doppelganger's colour and granted trigger are refused.
    assert!(refused(
        "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\n\
         K:ETBReplacement:Copy:CC:Optional\n\
         SVar:CC:DB$ Clone | Choices$ Creature.Other | SetColor$ Blue"
    ));
    // "As this enters, choose a color" is its controller's choice, and a
    // card handing it to somebody else is a different sentence.
    assert!(refused(
        "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
         SVar:CC:DB$ ChooseColor | Defined$ Opponent"
    ));
    // A word that is not one of the five.
    assert!(refused(
        "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
         SVar:CC:DB$ ChooseColor | Defined$ You | Exclude$ chartreuse"
    ));
    // A parameter no rule here claims.
    assert!(refused(
        "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
         SVar:CC:DB$ ChooseColor | Defined$ You | Amount$ 2"
    ));
    // And the pointer has to point somewhere.
    assert!(refused(
        "Name:X\nTypes:Land\nK:ETBReplacement:Other:Missing"
    ));
}

/// A card's token scripts are found wherever they are written, and each
/// is named once.
///
/// Both halves matter. The commonest shape in the corpus puts the effect
/// in an `SVar` and only the trigger on the rules line, so a scan of the
/// rules alone finds nothing for most of the cards that make tokens —
/// `SVar:TrigToken:DB$ Token` appears 1037 times against 402 `A:AB$
/// Token`. And a card that makes the same token twice is one token: the
/// list feeds an append-only ledger, where a repeat would be a second id
/// for one permanent.
#[test]
fn a_card_names_each_of_its_token_scripts_once() {
    let script = parse(
        "Name:Two Sides\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
         Execute$ TrigToken | TriggerDescription$ x\n\
         A:AB$ Token | Cost$ 2 | TokenScript$ r_1_1_goblin | TokenAmount$ 2\n\
         SVar:TrigToken:DB$ Token | TokenScript$ b_2_2_zombie | TokenOwner$ You\n\
         SVar:Other:DB$ Token | TokenScript$ r_1_1_goblin\n",
    );
    assert_eq!(token_stems(&script), ["r_1_1_goblin", "b_2_2_zombie"]);
    // A card that names none says so, rather than saying nothing at all.
    assert!(token_stems(&parse("Name:Plain\nK:Flying\n")).is_empty());
}

/// Every entry in `Tx::NAMED` is reachable from a valid-string the corpus
/// actually prints — a table row nothing produces is a claim no run
/// checks. The pairing itself is proved elsewhere and more strongly: the
/// pool dump is byte-identical across this substitution, which it could
/// not be if a constant held the clauses in another order.
#[test]
fn a_filter_the_dsl_already_names_is_written_as_that_name() {
    assert_eq!(filter("Creature.YouCtrl"), "Filter::YOUR_CREATURE");
    assert_eq!(filter("Creature.OppCtrl"), "Filter::OPPONENT_CREATURE");
    assert_eq!(filter("Creature.Other"), "Filter::ANOTHER_CREATURE");
    assert_eq!(filter("Creature.nonToken"), "Filter::NONTOKEN_CREATURE");
    assert_eq!(filter("Creature.Legendary"), "Filter::LEGENDARY_CREATURE");
    assert_eq!(filter("Creature.attacking"), "Filter::ATTACKING_CREATURE");
    assert_eq!(filter("Land.YouCtrl"), "Filter::YOUR_LAND");
    assert_eq!(filter("Artifact.YouCtrl"), "Filter::YOUR_ARTIFACT");
    assert_eq!(filter("Land.nonBasic"), "Filter::NONBASIC_LAND");
    assert_eq!(
        filter("Artifact,Enchantment"),
        "Filter::ARTIFACT_OR_ENCHANTMENT"
    );
    assert_eq!(filter("Artifact,Creature"), "Filter::ARTIFACT_OR_CREATURE");
    assert_eq!(
        filter("Artifact,Creature,Enchantment"),
        "Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT"
    );
    assert_eq!(
        filter("Creature,Planeswalker"),
        "Filter::CREATURE_OR_PLANESWALKER"
    );
    assert_eq!(filter("Instant,Sorcery"), "Filter::INSTANT_OR_SORCERY");
    // The counter-test: a filter with no constant is still written out,
    // and one the DSL spells in the other order is left alone rather
    // than quietly reordered.
    assert_eq!(
        filter("Creature.tapped"),
        "Filter::And(&[Filter::CREATURE, Filter::Tapped])"
    );
    assert_eq!(
        filter("Land.Basic"),
        "Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC)])"
    );
    assert_eq!(
        filter("Land.Basic+YouCtrl"),
        "Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC), \
         Filter::ControlledByYou])",
        "three flat clauses are not the nested YOUR_BASIC_LAND"
    );
    // The two the corpus writes both ways round. Neither reordering is
    // this reader's to make, so the rarer spelling is written out and
    // "another creature you control" reaches no name at all — see the
    // measurement on `NAMED`.
    assert_eq!(
        filter("Creature,Artifact"),
        "Filter::Or(&[Filter::CREATURE, Filter::ARTIFACT])"
    );
    assert_eq!(
        filter("Creature.Other+YouCtrl"),
        "Filter::And(&[Filter::CREATURE, Filter::Another, Filter::ControlledByYou])",
        "the commoner spelling builds the order ANOTHER_CREATURE_YOU_CONTROL does not have"
    );
}

/// A filter that is already a name is not given a second one. Fifteen
/// generated cards carried a `static` that was one bare constant, eight
/// of them `static TARGET1: Filter = Filter::CREATURE;`, which is the
/// duplication the hoist exists to prevent.
#[test]
fn a_filter_that_is_already_a_name_is_not_hoisted_into_a_static() {
    let body = read(
        "Name:X\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | SpellDescription$ deals 2 damage.",
    );
    assert!(body.statics.is_empty(), "{}", body.statics);
    assert!(
        body.abilities[0].contains("TargetSpec::Object(&Filter::CREATURE)"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn a_damage_spell_keeps_its_target_and_its_amount() {
    let body = read(
        "Name:Shock the Bear\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SpellDescription$ deals 3 damage.\n\
         Oracle:Shock the Bear deals 3 damage to target creature.",
    );
    assert_eq!(body.abilities.len(), 1);
    assert!(body.abilities[0].starts_with("spell!("));
    assert!(body.abilities[0].contains("Effect::DealDamage { amount: Amount::Fixed(3)"));
    assert!(body.abilities[0].contains("targets = Some(TargetReq::one("));
}

#[test]
fn keywords_become_bits_and_abilities_stay_abilities() {
    let body = read(
        "Name:Birds of Paradise\nManaCost:G\nTypes:Creature Bird\nPT:0/1\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | SpellDescription$ Add one mana of any color.\n\
         K:Flying\nOracle:Flying",
    );
    assert_eq!(body.keywords, ["KeywordSet::FLYING"]);
    assert_eq!(
        body.abilities,
        ["mana_ability!(&[Effect::mana_of_any_color()])"]
    );
}

#[test]
fn shadow_is_a_keyword_bit_rather_than_unblockable() {
    let body = read("Name:Shadow Test\nTypes:Creature Rogue\nPT:1/1\nK:Shadow\nOracle:Shadow");
    assert_eq!(body.keywords, ["KeywordSet::SHADOW"]);
    assert!(body.abilities.is_empty());
}

/// Banding is a keyword the engine reads (CR 702.22), on a body and
/// granted by a pump (Helm of Chatzuk); "bands with other" names a
/// quality and stays refused.
#[test]
fn banding_is_read_and_bands_with_other_is_not() {
    let body = read("Name:X\nTypes:Creature Human\nPT:1/1\nK:Banding\nOracle:Banding");
    assert_eq!(body.keywords, ["KeywordSet::BANDING"]);
    let body = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ Pump | Cost$ 1 T | ValidTgts$ Creature | KW$ Banding | SpellDescription$ …",
    );
    assert!(
        body.abilities
            .iter()
            .any(|a| a.contains("keywords: KeywordSet::BANDING")),
        "{:?}",
        body.abilities
    );
    let script = parse("Name:X\nTypes:Creature Human\nPT:1/1\nK:Bands with Other:Legendary");
    assert!(transcode(&script, &cats(), None).is_none());
}

/// "Attacks each combat if able" (CR 508.1d) and "can't attack unless
/// defending player controls an Island" (CR 508.1c) are statics the
/// engine reads; a requirement naming what to attack, and an `unless`
/// that is not "controls", stay refused.
#[test]
fn attack_requirements_and_island_restrictions_are_read() {
    let body = read(
        "Name:X\nManaCost:4\nTypes:Artifact Creature Juggernaut\nPT:5/3\n\
         S:Mode$ MustAttack | ValidCreature$ Card.Self | Description$ CARDNAME attacks each combat if able.\n",
    );
    assert_eq!(
        body.abilities,
        ["static_ability!(Filter::This, Modifier::AttacksEachCombat)"]
    );
    let body = read(
        "Name:X\nManaCost:5 U\nTypes:Creature Serpent\nPT:5/5\n\
         S:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ controlsIsland | \
         Description$ CARDNAME can't attack unless defending player controls an Island.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.starts_with(
            "static_ability!(Filter::This, Modifier::CantAttackUnlessDefenderControls(&"
        ),
        "{text}"
    );
    assert!(
        format!("{text}\n{}", body.statics).contains("ISLAND"),
        "the Island is what it asks for: {text}\n{}",
        body.statics
    );
    for refused_line in [
        "S:Mode$ MustAttack | ValidCreature$ Card.Self | MustAttack$ CardOwner",
        "S:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ isMonarch",
        "S:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ !controlsLand.untapped",
        "S:Mode$ CantAttack | ValidCard$ Card.Self",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Creature Serpent\nPT:5/5\n{refused_line}\n"
            )),
            "{refused_line}"
        );
    }
}

/// "Can block an additional creature each combat" (CR 509.1a) and
/// Lure's "all creatures able to block enchanted creature do so"
/// (CR 509.1c) are statics the engine reads; an amount that is not a
/// number, and any other hidden keyword, stay refused.
#[test]
fn block_limits_and_lures_are_read() {
    let body = read(
        "Name:X\nManaCost:4 R\nTypes:Creature Giant\nPT:4/4\nK:Trample\n\
         S:Mode$ Continuous | Affected$ Card.Self | CanBlockAmount$ 1 | \
         Description$ CARDNAME can block an additional creature each combat.\n",
    );
    assert_eq!(
        body.abilities,
        ["static_ability!(Filter::This, Modifier::CanBlockAdditional(1))"]
    );
    let body = read(
        "Name:X\nManaCost:1 G G\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | \
         AddHiddenKeyword$ All creatures able to block CARDNAME do so. | \
         Description$ All creatures able to block enchanted creature do so.\n",
    );
    assert!(
        body.abilities
            .iter()
            .any(|a| a.ends_with(", Modifier::MustBeBlockedByAllAble)")
                && a.contains("Filter::AttachedToBySource")),
        "{:?}",
        body.abilities
    );
    for refused_line in [
        "S:Mode$ Continuous | Affected$ Card.Self | CanBlockAmount$ X",
        "S:Mode$ Continuous | Affected$ Card.Self | AddHiddenKeyword$ CARDNAME can block only creatures with flying.",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Creature Giant\nPT:4/4\n{refused_line}\nSVar:X:Count$Valid Creature\n"
            )),
            "{refused_line}"
        );
    }
}

/// "When you control no Islands, sacrifice this" is a state trigger
/// (CR 603.8): the clause is the trigger's own condition, named above the
/// literal, and never an intervening "if". A clause the reader cannot
/// read, a zone it does not collect from, and a check at resolution stay
/// refused.
#[test]
fn a_state_trigger_is_read_as_its_own_condition() {
    let body = read(
        "Name:X\nManaCost:5 U\nTypes:Creature Serpent\nPT:5/5\n\
         T:Mode$ Always | TriggerZones$ Battlefield | IsPresent$ Island.YouCtrl | \
         PresentCompare$ EQ0 | Execute$ TrigSac | TriggerDescription$ When you control no Islands, sacrifice CARDNAME.\n\
         SVar:TrigSac:DB$ Sacrifice\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.starts_with("triggered!(Trigger::State(&STATE") && text.contains("SacrificeSelf"),
        "{text}"
    );
    assert!(
        !text.contains("condition ="),
        "not an intervening if: {text}"
    );
    assert!(
        body.statics
            .contains(": Condition = Condition::ControlCountAtMost(&")
            && body.statics.contains(", 0);"),
        "no Islands under your control: {}",
        body.statics
    );
    for refused_line in [
        "T:Mode$ Always | TriggerZones$ Battlefield | Execute$ TrigSac",
        "T:Mode$ Always | TriggerZones$ Graveyard | IsPresent$ Island.YouCtrl | PresentCompare$ EQ0 | Execute$ TrigSac",
        "T:Mode$ Always | TriggerZones$ Battlefield | IsPresent$ Island.YouCtrl | PresentCompare$ EQ0 | ResolvingCheck$ IsPresent | Execute$ TrigSac",
        "T:Mode$ Always | TriggerZones$ Battlefield | LifeTotal$ You | LifeAmount$ LE0 | Execute$ TrigSac",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Creature Serpent\nPT:5/5\n{refused_line}\nSVar:TrigSac:DB$ Sacrifice\n"
            )),
            "{refused_line}"
        );
    }
}

#[test]
fn a_two_color_mana_ability_is_a_choice_and_an_amount_is_a_count() {
    let body = read(
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W U\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 2",
    );
    assert_eq!(
        body.abilities,
        [
            "mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])",
            "mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)])",
        ]
    );
}

#[test]
fn a_trigger_resolves_the_svar_it_executes() {
    let body = read(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigGain | TriggerDescription$ gain 2 life.\n\
         SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
    );
    assert_eq!(
        body.abilities,
        ["triggered!(Trigger::ETB, &[Effect::gain_life(2)])"]
    );
}

/// "Whenever an opponent casts a spell": the two keys join into one
/// filter, and the three shapes that must not be read are refused by
/// name.
///
/// The zone is the one that matters. A `SpellCast` line with no
/// `TriggerZones$` is almost always "when you cast **this** spell",
/// which fires from the stack — 98 of the corpus's 114 zoneless lines
/// name `Card.Self` — and this engine collects triggers off the
/// battlefield. Read as an ordinary trigger it is a card whose ability
/// can never fire, so both halves of that sentence are refused
/// separately: the missing zone, and the self-reference under any zone.
#[test]
fn a_spell_cast_trigger_joins_what_was_cast_with_who_cast_it() {
    // Rhystic Study's own line, minus the `UnlessCost$` its `SVar`
    // writes — which is a different rule's business.
    let body = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent \
         | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ draw.\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert_eq!(
        body.abilities,
        ["triggered!(Trigger::SpellCast(&Filter::ControlledByOpponent), &[Effect::draw(1)])"]
    );

    // Every alternative takes the controller atom, not just the first.
    let two = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You \
         | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ draw.\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        two.statics.contains(
            "Filter::Or(&[Filter::And(&[Filter::HasType(TypeSet::INSTANT), \
             Filter::ControlledByYou]), Filter::And(&[Filter::HasType(TypeSet::SORCERY), \
             Filter::ControlledByYou])])"
        ),
        "{}",
        two.statics
    );

    for (line, why) in [
        (
            "T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ You \
             | Execute$ TrigDraw | TriggerDescription$ draw.",
            "a `SpellCast` trigger with no `TriggerZones$`",
        ),
        (
            "T:Mode$ SpellCast | ValidCard$ Card.Self | TriggerZones$ Battlefield \
             | Execute$ TrigDraw | TriggerDescription$ draw.",
            "a `SpellCast` trigger on the card itself",
        ),
        (
            "T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Player.EnchantedBy \
             | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ draw.",
            "a spell cast by `Player.EnchantedBy`",
        ),
    ] {
        let script = parse(&format!(
            "Name:X\nTypes:Enchantment\n{line}\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
        ));
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(why),
            "{line}"
        );
    }
}

/// "When this enters, you may …" is one decision over the whole clause.
///
/// CR 603.5: an optional triggered ability goes on the stack whatever
/// its controller intends, and the choice is made as it resolves — which
/// is `Effect::MayDo`, asked of the resolving ability's controller. The
/// three shapes that must not become one are refused instead:
///
/// * a decider who is not the controller, which no effect here can ask;
/// * a `may` whose clause is "pay this cost, then …" — the reference
///   writes that as a `Cost$` on the executed sub-ability, and 183 of
///   the 1442 `OptionalDecider$ You` triggers do. Read as a bare `MayDo`
///   it would be a free effect under `Coverage::Implemented`, for
///   hundreds of cards at once, so the counter-case is pinned here and
///   not merely inferred from where `Cost$` is claimed;
/// * the same key on a **sub-ability** (83 in the corpus), where
///   declining skips the rest of the chain rather than one step, and so
///   is not this shape at all.
#[test]
fn a_may_on_a_trigger_wraps_the_whole_clause() {
    let body = read(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | OptionalDecider$ You | Execute$ TrigGain | TriggerDescription$ you may gain 2 life.\n\
         SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
    );
    assert_eq!(
        body.abilities,
        ["triggered!(Trigger::ETB, &[Effect::MayDo { effects: &[Effect::gain_life(2)] }])"]
    );

    // A target is chosen when the ability goes on the stack (CR 603.3d)
    // and the `may` is answered as it resolves, so the target stays
    // outside the wrap.
    let targeted = read(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | OptionalDecider$ You | Execute$ TrigLose | TriggerDescription$ you may drain.\n\
         SVar:TrigLose:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ 1",
    );
    assert_eq!(
        targeted.abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::MayDo { effects: &[Effect::LoseLife { \
             amount: Amount::Fixed(1), target: PlayerRel::Chosen }] }], \
             targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
        ]
    );

    for (lines, why) in [
        (
            "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
             | ValidCard$ Card.Self | OptionalDecider$ TriggeredCardController \
             | Execute$ TrigGain | TriggerDescription$ x.\n\
             SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
            "a `may` decided by `TriggeredCardController`",
        ),
        (
            // Ruin Processor's shape: "you may put a card an opponent
            // owns from exile into that player's graveyard. If you
            // do, you gain 5 life."
            "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
             | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ TrigGain \
             | TriggerDescription$ x.\n\
             SVar:TrigGain:AB$ GainLife | Cost$ Sac<1/Creature.YouCtrl/a creature> \
             | LifeAmount$ 5",
            "unclaimed parameter `GainLife.Cost`",
        ),
        (
            "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
             | ValidCard$ Card.Self | Execute$ TrigGain | TriggerDescription$ x.\n\
             SVar:TrigGain:DB$ GainLife | LifeAmount$ 2 | OptionalDecider$ You",
            "unclaimed parameter `GainLife.OptionalDecider`",
        ),
    ] {
        let script = parse(&format!("Name:X\nTypes:Creature Goblin\nPT:1/1\n{lines}"));
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(why),
            "{lines}"
        );
    }
}

/// Crystal Rod: "Whenever a player casts a blue spell, you may pay {1}.
/// If you do, you gain 1 life." The price is the "may", so it replaces
/// the `MayDo` rather than sitting inside it. A coloured price is
/// Farmstead's `{W}{W}`, printed exactly (`PlayerMayPayManaThen`); a
/// price that is not mana is a payment neither takes, and stays refused.
#[test]
fn a_may_with_a_generic_price_is_a_payment_that_buys_the_clause() {
    let body = read(
        "Name:X\nManaCost:1\nTypes:Artifact\n\
         T:Mode$ SpellCast | ValidCard$ Card.Blue | TriggerZones$ Battlefield \
         | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ x.\n\
         SVar:TrigGainLife:AB$ GainLife | Cost$ 1 | Defined$ You | LifeAmount$ 1",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains(
            "Effect::PlayerMayPayThen { player: PlayerRel::You, mana: Amount::Fixed(1), \
             effects: &[Effect::gain_life(1)] }"
        ),
        "{text}"
    );
    assert!(!text.contains("MayDo"), "one question, not two: {text}");

    let script = parse(
        "Name:X\nManaCost:1\nTypes:Artifact\n\
         T:Mode$ SpellCast | ValidCard$ Card.Blue | TriggerZones$ Battlefield \
         | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ x.\n\
         SVar:TrigGainLife:AB$ GainLife | Cost$ W W | Defined$ You | LifeAmount$ 1",
    );
    let text = transcode(&script, &cats(), None)
        .expect("a coloured price is read")
        .abilities
        .join("\n");
    assert!(
        text.contains(
            "Effect::PlayerMayPayManaThen { player: PlayerRel::You, cost: mana!(\"{W}{W}\"), \
             effects: &[Effect::gain_life(1)] }"
        ),
        "{text}"
    );

    let script = parse(
        "Name:X\nManaCost:1\nTypes:Artifact\n\
         T:Mode$ SpellCast | ValidCard$ Card.Blue | TriggerZones$ Battlefield \
         | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ x.\n\
         SVar:TrigGainLife:AB$ GainLife | Cost$ PayLife<1> | Defined$ You | LifeAmount$ 1",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("unclaimed parameter `GainLife.Cost`")
    );
}

/// "That player" is the trigger's to say: whose step began for a
/// `Phase` trigger (Copper Tablet), the moved card's controller for a
/// `ChangesZone` one (Dingus Egg), and the enchanted permanent's
/// controller where the upkeep is theirs (Cursed Land). The same words
/// on an activated ability name nothing this reader can see.
#[test]
fn that_player_is_the_one_the_trigger_names() {
    let tablet = read(
        "Name:X\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield \
         | Execute$ TrigDamage | TriggerDescription$ x.\n\
         SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1",
    );
    assert_eq!(
        tablet.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }, \
             &[Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::ActivePlayer) }])"
        ]
    );

    let egg = read(
        "Name:X\nManaCost:4\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Land \
         | TriggerZones$ Battlefield | Execute$ TrigDamage | TriggerDescription$ x.\n\
         SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 2",
    );
    let text = egg.abilities.join("\n");
    assert!(
        text.contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
        "{text}"
    );

    let cursed = read(
        "Name:X\nManaCost:2 B B\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player.EnchantedController \
         | TriggerZones$ Battlefield | Execute$ TrigDamage | TriggerDescription$ x.\n\
         SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1",
    );
    let text = cursed.abilities.join("\n");
    assert!(
        text.contains("whose: PlayerRel::ControllerOfAttached"),
        "{text}"
    );

    assert!(refused(
        "Name:X\nManaCost:2\nTypes:Artifact\n\
         A:AB$ DealDamage | Cost$ T | Defined$ TriggeredPlayer | NumDmg$ 1"
    ));
}

/// A card in a graveyard is its own kind of target: Regrowth and
/// Resurrection move from `Origin$ Graveyard`, and read as a permanent
/// on the battlefield they offered nothing to target. `YouCtrl` there is
/// "your graveyard".
#[test]
fn a_graveyard_card_is_targeted_in_its_graveyard() {
    let regrowth = read(
        "Name:X\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Card.YouCtrl",
    );
    let text = regrowth.abilities.join("\n");
    assert!(
        text.contains(
            "Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&Filter::Any, \
             PlayerRel::You) }"
        ),
        "{text}"
    );

    let resurrection = read(
        "Name:X\nManaCost:2 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield \
         | ValidTgts$ Creature.YouCtrl",
    );
    let text = resurrection.abilities.join("\n");
    assert!(
        text.contains(
            "Effect::GraveyardToBattlefield { target: TargetSpec::CardInGraveyard(\
             &Filter::CREATURE, PlayerRel::You), owner_control: false, counters: None }"
        ),
        "{text}"
    );
}

/// Braingeyser and Stream of Life: the X the caster announced, drawn or
/// gained by the player the spell targets. An `X` that counts
/// something is a different number and stays refused.
#[test]
fn an_announced_x_is_drawn_and_gained() {
    let geyser = read(
        "Name:X\nManaCost:X U U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X | ValidTgts$ Player\nSVar:X:Count$xPaid",
    );
    assert_eq!(
        geyser.abilities,
        [
            "spell!(&[Effect::DrawCardsFor { amount: Amount::X, who: PlayerRel::Chosen }], \
             targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
        ]
    );
    let stream = read(
        "Name:X\nManaCost:X G G\nTypes:Sorcery\n\
         A:SP$ GainLife | ValidTgts$ Player | LifeAmount$ X\nSVar:X:Count$xPaid",
    );
    assert_eq!(
        stream.abilities,
        [
            "spell!(&[Effect::GainLifeFor { amount: Amount::X, who: PlayerRel::Chosen }], \
             targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
        ]
    );
    // A count is a count and not the announced X: read as one.
    let counted = read(
        "Name:X\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ X\nSVar:X:Count$Valid Creature.YouCtrl",
    );
    assert!(
        counted.abilities[0].contains("Effect::GainLife { amount: Amount::CountOf"),
        "{:?}",
        counted.abilities
    );
    // And the announced X on a trigger is still refused.
    assert!(refused(
        "Name:X\nTypes:Creature\n\
         T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | \
         Execute$ G\nSVar:G:DB$ GainLife | LifeAmount$ X\nSVar:X:Count$xPaid"
    ));
}

/// Earthquake: "X damage to each creature without flying and each
/// player" — the permanents through `DealDamageEach`, the players
/// through `DealDamage`, and "without flying" as a keyword the engine
/// has a bit for.
#[test]
fn damage_to_each_is_the_permanents_and_the_players() {
    let quake = read(
        "Name:X\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DamageAll | ValidCards$ Creature.withoutFlying | ValidPlayers$ Player \
         | NumDmg$ X\nSVar:X:Count$xPaid",
    );
    let text = quake.abilities.join("\n");
    assert!(
        text.contains(
            "Effect::DealDamageEach { amount: Amount::X, filter: &EACH1 }, \
             Effect::DealDamage { amount: Amount::X, target: TargetSpec::Player(\
             PlayerRel::EachPlayer) }"
        ),
        "{text}"
    );
    assert!(
        quake
            .statics
            .contains("Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING))"),
        "{}",
        quake.statics
    );
    assert!(refused(
        "Name:X\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DamageAll | ValidCards$ Creature.withBushido | NumDmg$ 1"
    ));
}

#[test]
fn a_subability_chain_becomes_a_sequence_of_effects() {
    let body = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 2 | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | LifeAmount$ 2",
    );
    assert_eq!(
        body.abilities,
        [
            "spell!(&[Effect::draw(2), Effect::LoseLife { amount: Amount::Fixed(2), target: PlayerRel::You }])"
        ]
    );
}

/// "Target player loses 1 life" (Piranha Marsh): the effect carries no
/// `Defined$`, and reading that as `You` would drain the controller. The
/// chain targets a player, so the absent key means the chosen one.
#[test]
fn an_undefined_player_effect_means_the_targeted_player() {
    let body = read(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigLoseLife | TriggerDescription$ loses 1 life.\n\
         SVar:TrigLoseLife:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ 1 | TgtPrompt$ Select target player",
    );
    assert_eq!(
        body.abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::Chosen }], targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
        ]
    );
}

/// The other half of that rule: the absent key means the *line's* own
/// target, and a sub-ability inherits none. Last Caress — "target player
/// loses 1 life and you gain 1 life. Draw a card." — is a targeting
/// `LoseLife` and then a bare `GainLife` and a bare `Draw`, and read
/// against the chain's target it handed the life and the card to the
/// player it was draining.
#[test]
fn an_undefined_player_effect_after_the_targeting_line_means_you() {
    let body = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ 1 | SubAbility$ DBGainLife | SpellDescription$ drain.\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw",
    );
    assert_eq!(
        body.abilities,
        [
            "spell!(&[Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::Chosen }, \
             Effect::gain_life(1), Effect::draw(1)], \
             targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
        ]
    );
}

/// A checkland: a script writes the printed "enters tapped **unless** you
/// control a Swamp or a Mountain" inside out, as "tap it when the count
/// of those is zero". `EQ0` is that sentence and nothing else is.
#[test]
fn a_conditional_enters_tapped_becomes_tapped_unless() {
    let body = read(
        "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
         SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Basic+YouCtrl | ConditionCompare$ EQ0",
    );
    assert_eq!(
        body.enter_modifiers,
        ["EnterModifier::TappedUnless(&CHECK1)"]
    );
    assert!(
        body.statics
            .contains("Filter::HasSupertype(SupertypeSet::BASIC)"),
        "{}",
        body.statics
    );
}

/// "Unless you control *two* other lands" is a count, and the
/// arithmetic between the script and the card is the whole risk.
///
/// The reference says when the land comes down **tapped** and the card
/// prints when it does not, so `LT n` is `at_least = n` and `LE n` is
/// `at_least = n + 1`. Neither is argued from the key’s name: Rockfall
/// Vale writes `LT2` and prints "two or more other lands", and Canopy
/// Vista writes `LE1` against a hand-written `at_least: 2` in this very
/// pool. Off by one here is a land that enters untapped a turn early,
/// which no test downstream would catch.
///
/// `at_least: 1` is deliberately *not* emitted — that sentence is what
/// `TappedUnless` already says, and a second spelling of one sentence is
/// what the pool-wide lints exist to prevent.
#[test]
fn a_counted_enters_tapped_condition_becomes_a_count() {
    let two = read(
        "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
         SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Other+YouCtrl | ConditionCompare$ LT2",
    );
    assert_eq!(
        two.enter_modifiers,
        ["EnterModifier::TappedUnlessCount { filter: &CHECK1, at_least: 2 }"]
    );

    // Canopy Vista’s own line, and the pool’s own answer to it.
    let battle = read(
        "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
         SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Basic+YouCtrl | ConditionCompare$ LE1",
    );
    assert_eq!(
        battle.enter_modifiers,
        ["EnterModifier::TappedUnlessCount { filter: &CHECK1, at_least: 2 }"]
    );

    // Steam Vents: `PayLife<2>` is `TappedOrPayLife(2)`, which is how
    // that card is written by hand three directories away.
    let shock = read(
        "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
         SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | UnlessCost$ PayLife<2> | UnlessPayer$ You",
    );
    assert_eq!(shock.enter_modifiers, ["EnterModifier::TappedOrPayLife(2)"]);

    // Blackcleave Cliffs: "unless you control two or fewer other lands",
    // which the reference states as the tap — `GT2`.
    let fast = read(
        "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
         SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.YouCtrl | ConditionCompare$ GT2",
    );
    assert_eq!(
        fast.enter_modifiers,
        ["EnterModifier::TappedUnlessAtMost { filter: &Filter::YOUR_LAND, at_most: 2 }"]
    );

    // The manlands write one sentence two ways — Hall of Storm Giants
    // `GE2`, Den of the Bugbear `GT1` — and both are the same bound. A
    // reader that took the letter rather than the predicate would have
    // given one cycle two different cards.
    for tail in ["GE2", "GT1"] {
        let manland = read(&format!(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.YouCtrl | ConditionCompare$ {tail}"
        ));
        assert_eq!(
            manland.enter_modifiers,
            ["EnterModifier::TappedUnlessAtMost { filter: &Filter::YOUR_LAND, at_most: 1 }"],
            "{tail}"
        );
    }

    for (tail, why) in [
        // `GE0` is where the conversion would underflow, and it is not a
        // sentence: a land tapped whatever the board says needs the other
        // variant entirely. The corpus writes it nowhere, so this is the
        // boundary being refused rather than a card being lost.
        (
            "| ConditionPresent$ Land.YouCtrl | ConditionCompare$ GE0",
            "an enter-tapped condition `GE0` this rule cannot read",
        ),
        // Rustic Clachan reveals a Kithkin instead of paying life.
        (
            "| UnlessCost$ Reveal<1/Kithkin> | UnlessPayer$ You",
            "replacement `Moved` charging `Reveal<1/Kithkin>`",
        ),
        (
            "| UnlessCost$ PayLife<2> | UnlessPayer$ Opponent",
            "replacement `Moved` charging `Opponent`",
        ),
        // A computed condition whose SVar is not there to read.
        (
            "| ConditionCheckSVar$ X | ConditionSVarCompare$ LT2",
            "`ConditionCheckSVar$ X` names no SVar",
        ),
    ] {
        let script = parse(&format!(
            "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
         SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True {tail}"
        ));
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(why),
            "{tail}"
        );
    }
}
