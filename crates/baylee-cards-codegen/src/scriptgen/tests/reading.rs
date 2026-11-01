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

#[test]
fn instant_history_requires_the_exact_cast_type_count_and_player() {
    for (player, relation) in [
        ("Player", "EachPlayer"),
        ("You", "You"),
        ("Player.Opponent", "EachOpponent"),
    ] {
        let script = format!(
            "T:Mode$ SpellCast | ValidCard$ Instant | ValidActivatingPlayer$ {player} | ActivatorThisTurnCast$ GT1 | TriggerZones$ Battlefield | Execute$ Reward\nSVar:Reward:DB$ GainLife | Defined$ You | LifeAmount$ 1"
        );
        assert!(read(&script).abilities[0].contains(&format!(
            "Trigger::InstantCastAfterFirst {{ who: PlayerRel::{relation} }}"
        )));
        for (from, to) in [
            ("ValidCard$ Instant", "ValidCard$ Sorcery"),
            ("ValidCard$ Instant", "ValidCard$ Instant,Sorcery"),
            ("GT1", "GT2"),
            ("GT1", "GE1"),
            ("GT1", "X"),
            ("TriggerZones$ Battlefield", "TriggerZones$ Graveyard"),
        ] {
            assert!(refused(&script.replace(from, to)), "{from} -> {to}");
        }
        assert!(refused(
            &script.replace(" | Execute$", " | UnknownHistory$ True | Execute$")
        ));
    }
}

#[test]
fn entry_history_activation_reads_a_subtype_union_and_refuses_extra_predicates() {
    let script = "A:AB$ Surveil | Cost$ U T | Amount$ 2 | CheckSVar$ Entry\nSVar:Entry:Count$ThisTurnEntered_Battlefield_Goblin.YouCtrl,Wizard.YouCtrl";
    let body = read(script);
    assert!(body.abilities[0].contains("Condition::EnteredSubtypeUnderYourControlThisTurn(&["));
    assert!(body.abilities[0].contains("creature::GOBLIN"));
    assert!(body.abilities[0].contains("creature::WIZARD"));
    for (from, to) in [
        ("Goblin.YouCtrl", "Goblin.OppCtrl"),
        ("Goblin.YouCtrl", "Goblin.YouCtrl+tapped"),
        ("Goblin.YouCtrl", "UnknownSubtype.YouCtrl"),
        ("ThisTurnEntered", "LastTurnEntered"),
        ("Battlefield_", "Graveyard_"),
        ("AB$", "SP$"),
        ("CheckSVar$ Entry", "CheckSVar$ Missing"),
        ("CheckSVar$ Entry", "CheckSVar$ Entry | SVarCompare$ GE2"),
        ("CheckSVar$ Entry", "CheckSVar$ Entry | PlayerTurn$ True"),
    ] {
        assert!(refused(&script.replace(from, to)), "{from} -> {to}");
    }
}

#[test]
fn costless_mayhem_is_only_a_land_permission_and_preserves_other_refusals() {
    assert_eq!(
        read("Types:Land\nK:Mayhem").keywords,
        ["KeywordSet::MAYHEM"]
    );
    for script in [
        "K:Mayhem",
        "Types:Creature Goblin\nK:Mayhem",
        "Types:Sorcery\nK:Mayhem",
        "Types:Land\nK:Mayhem:2 U",
        "Types:Land\nK:Mayhem\nK:UnknownPermission",
    ] {
        assert!(refused(script), "{script}");
    }
}

#[test]
fn graveyard_self_entry_keeps_the_origin_and_rejects_other_objects() {
    let script = "Types:Land\nK:Mayhem\nT:Mode$ ChangesZone | Origin$ Graveyard | Destination$ Battlefield | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ Loss\nSVar:Loss:DB$ LoseLife | Defined$ You | LifeAmount$ 2";
    let body = read(script);
    assert_eq!(body.keywords, ["KeywordSet::MAYHEM"]);
    assert!(body.abilities[0].contains("Trigger::EnteredThisFrom(ZoneRef::Graveyard)"));
    assert!(body.abilities[0].contains("Effect::LoseLife"));
    for (from, to) in [
        ("Card.Self", "Card"),
        ("Card.Self", "Land.YouCtrl"),
        ("Card.Self", "Card.Self+tapped"),
        ("Origin$ Graveyard", "Origin$ Graveyard,Exile"),
        ("Origin$ Graveyard", "Origin$ Exile"),
        ("Destination$ Battlefield", "Destination$ Hand"),
        ("TriggerZones$ Battlefield", "TriggerZones$ Graveyard"),
        (" | Execute$", " | UnknownOriginRule$ True | Execute$"),
    ] {
        assert!(refused(&script.replace(from, to)), "{from} -> {to}");
    }
}

#[test]
fn entry_history_accepts_the_four_published_subtypes_without_a_creature_gate() {
    let mut catalog = cats();
    catalog
        .creature
        .extend(["Frog", "Bird", "Otter", "Rat"].map(str::to_string));
    catalog.normalize();
    let script = parse(
        "A:AB$ Surveil | Cost$ U T | Amount$ 2 | CheckSVar$ Entry\nSVar:Entry:Count$ThisTurnEntered_Battlefield_Frog.YouCtrl,Bird.YouCtrl,Otter.YouCtrl,Rat.YouCtrl",
    );
    let body = transcode(&script, &catalog, None).expect("the exact subtype union is readable");
    for subtype in ["FROG", "BIRD", "OTTER", "RAT"] {
        assert!(body.abilities[0].contains(&format!("creature::{subtype}")));
    }
    assert!(!body.abilities[0].contains("Filter::CREATURE"));
}

#[test]
fn landwalk_removal_and_keyword_prohibition_keep_their_different_meanings() {
    let walk = "A:AB$ Debuff | Cost$ T | ValidTgts$ Creature | AllSuffixKeywords$ walk";
    assert!(
        read(walk).abilities[0]
            .contains("Effect::ModifyTargets { modifier: Modifier::LoseAllLandwalk")
    );
    for (a, b) in [
        ("walk", "proof"),
        ("ValidTgts$ Creature", "ValidTgts$ Artifact"),
        (
            "AllSuffixKeywords$ walk",
            "AllSuffixKeywords$ walk | Duration$ Permanent",
        ),
    ] {
        assert!(refused(&walk.replace(a, b)));
    }
    let light = "A:AB$ AnimateAll | Cost$ 1 T | ValidCards$ Creature.OppCtrl | RemoveKeywords$ Hexproof & Shroud | CantHaveKeyword$ Hexproof & Shroud";
    assert!(read(light).abilities[0].contains("Modifier::CantHaveKeywords"));
    for (a, b) in [
        ("Creature.OppCtrl", "Creature.YouCtrl"),
        (
            "CantHaveKeyword$ Hexproof & Shroud",
            "CantHaveKeyword$ Shroud",
        ),
        (
            "RemoveKeywords$ Hexproof & Shroud",
            "RemoveKeywords$ Hexproof",
        ),
        (
            "CantHaveKeyword$ Hexproof & Shroud",
            "CantHaveKeyword$ Hexproof & Shroud | SetPower$ 4",
        ),
    ] {
        assert!(refused(&light.replace(a, b)));
    }
}

#[test]
fn chosen_source_all_damage_shield_is_repeatable_and_not_recipient_limited() {
    let script = "A:AB$ ChooseSource | Cost$ 4 W U T | Choices$ Card,Emblem | SubAbility$ Shield\nSVar:Shield:DB$ Effect | ReplacementEffects$ Prevent | ConditionDefined$ ChosenCard | ConditionPresent$ Card,Emblem | SubAbility$ Clean\nSVar:Prevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | Prevent$ True\nSVar:Clean:DB$ Cleanup | ClearChosenCard$ True";
    assert!(read(script).abilities[0].contains("Effect::ChooseDamageSourcePreventAll"));
    for (a, b) in [
        ("Prevent$ True", "Prevent$ False"),
        ("Prevent$ True", "Prevent$ True | ValidTarget$ You"),
        ("Prevent$ True", "Prevent$ True | CombatDamage$ True"),
        (
            "Card.ChosenCardStrict,Emblem.ChosenCard",
            "Card.ChosenCardStrict",
        ),
        ("ClearChosenCard$ True", "ClearChosenCard$ False"),
    ] {
        assert!(refused(&script.replace(a, b)));
    }
}

#[test]
fn graveyard_return_with_targeted_mana_cost_x_adds_cmc_at_most_x_filter() {
    let body = read(
        "Name:Stir the Grave\nManaCost:X B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl\n\
         SVar:X:Targeted$CardManaCost",
    );
    assert!(
        body.statics
            .contains("Filter::And(&[Filter::CREATURE, Filter::CmcAtMostX])"),
        "{}",
        body.statics
    );
    assert_eq!(
        body.abilities,
        [
            "spell!(&[Effect::GraveyardToBattlefield { target: TargetSpec::CardInGraveyard(&TARGET1, PlayerRel::You), owner_control: false, counters: None }], targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&TARGET1, PlayerRel::You))))"
        ]
    );
}

#[test]
fn player_target_with_target_opponent_oracle_narrows_to_any_opponent() {
    let body = read(
        "Name:Dread Rider\nManaCost:5 B\nTypes:Creature Spirit Knight\nPT:3/7\n\
         A:AB$ LoseLife | Cost$ 1 B T ExileFromGrave<1/Creature> | ValidTgts$ Player | LifeAmount$ 3\n\
         Oracle:{1}{B}, {T}, Exile a creature card from your graveyard: Target opponent loses 3 life.",
    );
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{1}{B}\", TapSelf, ExileFromGraveyard(&Filter::CREATURE)), &[Effect::LoseLife { amount: Amount::Fixed(3), target: PlayerRel::Chosen }], target = Some(TargetSpec::AnyOpponent))"
        ]
    );
}

#[test]
fn comma_separated_and_oracle_scoped_graveyard_targets_strip_player_qualifiers() {
    let argivian = read(
        "Name:Argivian Find\nManaCost:W\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Artifact.YouCtrl,Enchantment.YouCtrl\n\
         Oracle:Return target artifact or enchantment card from your graveyard to your hand.",
    );
    assert_eq!(
        argivian.abilities,
        [
            "spell!(&[Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&Filter::ARTIFACT_OR_ENCHANTMENT, PlayerRel::You) }], targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::ARTIFACT_OR_ENCHANTMENT, PlayerRel::You))))"
        ]
    );

    let misery = read(
        "Name:Misery Charm\nManaCost:B\nTypes:Instant\n\
         A:SP$ Charm | Choices$ DBDestroy,DBReturn,DBLoseLife\n\
         SVar:DBDestroy:DB$ Destroy | ValidTgts$ Creature.Goblin | SpellDescription$ Destroy target Goblin.\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.Goblin | SpellDescription$ Return target Goblin card from your graveyard to your hand.\n\
         SVar:DBLoseLife:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ 2 | SpellDescription$ Target player loses 2 life.\n\
         Oracle:Choose one —\\n• Destroy target Goblin.\\n• Return target Goblin card from your graveyard to your hand.\\n• Target player loses 2 life.",
    );
    assert!(
        misery.abilities[0].contains("TargetSpec::CardInGraveyard(&TARGET1, PlayerRel::You)"),
        "{}",
        misery.abilities[0]
    );
}

#[test]
fn permanent_card_in_graveyard_or_library_filters_permanent_types_only() {
    let spiral = read(
        "Name:Nature's Spiral\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Permanent.YouCtrl\n\
         Oracle:Return target permanent card from your graveyard to your hand.",
    );
    assert!(
        spiral.statics.contains(
            "Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::BATTLE).union(TypeSet::CREATURE).union(TypeSet::ENCHANTMENT).union(TypeSet::LAND).union(TypeSet::PLANESWALKER))"
        ),
        "{}",
        spiral.statics
    );

    let bridge = read(
        "Name:Planar Bridge\nManaCost:6\nTypes:Legendary Artifact\n\
         A:AB$ ChangeZone | Cost$ 8 T | Origin$ Library | Destination$ Battlefield | ChangeType$ Permanent\n\
         Oracle:{8}, {T}: Search your library for a permanent card, put it onto the battlefield, then shuffle.",
    );
    assert!(
        bridge.statics.contains(
            "Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::BATTLE).union(TypeSet::CREATURE).union(TypeSet::ENCHANTMENT).union(TypeSet::LAND).union(TypeSet::PLANESWALKER))"
        ),
        "{}",
        bridge.statics
    );
}

#[test]
fn multiple_prowess_keywords_emit_separate_triggers() {
    let thor = read(
        "Name:Thor Odinson\nManaCost:3 R W\nTypes:Legendary Creature God Warrior Hero\nPT:4/4\n\
         K:Flying\nK:Vigilance\nK:Prowess\nK:Prowess\n\
         Oracle:Flying, vigilance, prowess, prowess",
    );
    assert_eq!(
        thor.keywords,
        [
            "KeywordSet::FLYING",
            "KeywordSet::VIGILANCE",
            "KeywordSet::PROWESS"
        ]
    );
    assert_eq!(
        thor.abilities,
        [
            "triggered!(Trigger::SpellCast(&TRIGGER1), &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(1), toughness: Amount::Fixed(1), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn next_turns_upkeep_delayed_trigger_reads_slowtrips() {
    let clairvoyance = "A:SP$ RevealHand | ValidTgts$ Player | SubAbility$ DelTrig\n\
                        SVar:DelTrig:DB$ DelayedTrigger | NextTurn$ True | Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ DrawSlowtrip\n\
                        SVar:DrawSlowtrip:DB$ Draw | Defined$ You";
    let infuse = "A:SP$ Untap | ValidTgts$ Creature | SubAbility$ DelTrig\n\
                  SVar:DelTrig:DB$ DelayedTrigger | NextTurn$ True | Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ DrawSlowtrip\n\
                  SVar:DrawSlowtrip:DB$ Draw | Defined$ You";
    let body = read(infuse);
    assert!(
        body.abilities[0].contains("Effect::AtNextTurnsUpkeep { effects: &[Effect::draw(1)] }"),
        "{}",
        body.abilities[0]
    );
    assert!(refused(clairvoyance));
    assert!(refused(&infuse.replace("NextTurn$ True | ", "")));
    assert!(refused(
        &infuse.replace("ValidPlayer$ Player", "ValidPlayer$ You")
    ));
}

#[test]
fn cumulative_upkeep_keyword_and_grant_read_generic_colored_and_life_costs() {
    let generic = read("K:Cumulative upkeep:2");
    assert_eq!(
        generic.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, &[Effect::AddCounter { kind: counters::AGE, amount: Amount::Fixed(1) }, Effect::PlayerMayPayOr { player: PlayerRel::You, mana: Amount::Times { base: &Amount::CountersOnSource(counters::AGE), factor: 2 }, effect: &Effect::SacrificeSelf }], condition = Some(Condition::SourceMatches(&Filter::InZone(ZoneRef::Battlefield))))"
        ]
    );

    let colored = read("K:Cumulative upkeep:1 U");
    assert_eq!(
        colored.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, &[Effect::AddCounter { kind: counters::AGE, amount: Amount::Fixed(1) }, Effect::PlayerMayPayManaEachThenElse { player: PlayerRel::You, each: mana!(\"{1}{U}\"), times: Amount::CountersOnSource(counters::AGE), paid: &[], unpaid: &[Effect::SacrificeSelf] }], condition = Some(Condition::SourceMatches(&Filter::InZone(ZoneRef::Battlefield))))"
        ]
    );

    let life = read("K:Cumulative upkeep:PayLife<2>:Pay 2 life.");
    assert_eq!(
        life.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, &[Effect::AddCounter { kind: counters::AGE, amount: Amount::Fixed(1) }, Effect::PlayerMayPayLifeOr { player: PlayerRel::You, life: Amount::Times { base: &Amount::CountersOnSource(counters::AGE), factor: 2 }, effect: &Effect::SacrificeSelf }], condition = Some(Condition::SourceMatches(&Filter::InZone(ZoneRef::Battlefield))))"
        ]
    );

    let grant = read(
        "S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 3 | AddToughness$ 3 | AddKeyword$ Cumulative upkeep:R",
    );
    assert!(
        grant.abilities[1].contains("Modifier::GrantTriggered")
            && grant.abilities[1].contains("each: mana!(\"{R}\")"),
        "{}",
        grant.abilities[1]
    );
}

#[test]
fn pump_all_and_nonbasic_landwalk_and_graveyard_gain_control_read() {
    let pump = read(
        "A:AB$ PumpAll | Cost$ 1 W | ValidCards$ Creature.attacking | NumAtt$ +1 | NumDef$ +0 | KW$ First Strike",
    );
    assert_eq!(
        pump.abilities,
        [
            "activated!(cost!(\"{1}{W}\"), &[Effect::PumpFilter { filter: &Filter::ATTACKING_CREATURE, controlled_by: None, power: Amount::Fixed(1), toughness: Amount::Fixed(0), keywords: KeywordSet::FIRST_STRIKE, duration: Duration::UntilEndOfTurn }])"
        ]
    );

    let snow_walk = read("K:Landwalk:Land.Snow:snow land");
    assert!(
        snow_walk.abilities[0].contains("Modifier::LandwalkMatching(&LANDWALK1)"),
        "{}",
        snow_walk.abilities[0]
    );

    let reanimate = read(
        "A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature | GainControl$ True",
    );
    assert!(
        reanimate.abilities[0].contains("Effect::GraveyardToBattlefield")
            && reanimate.abilities[0].contains("owner_control: false"),
        "{}",
        reanimate.abilities[0]
    );
}

#[test]
fn echo_keyword_defined_graveyard_controller_and_each_end_step_oracle_read() {
    let echo = read("K:Echo:2 R R");
    assert_eq!(
        echo.abilities,
        ["AbilityDef::Echo { cost: mana!(\"{2}{R}{R}\") }"]
    );
    assert!(refused("K:Echo:PayLife<2>"));

    let diver = read(
        "T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Artifact.Other | TargetsWithDefinedController$ TriggeredCardController\n\
         Oracle:When this creature dies, return another target artifact card from your graveyard to your hand.",
    );
    assert!(
        diver.abilities[0].contains("TargetSpec::CardInGraveyard(&TARGET1, PlayerRel::You)"),
        "{}",
        diver.abilities[0]
    );
    assert!(refused(
        "T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Artifact.Other | TargetsWithDefinedController$ TriggeredCardController\n\
         Oracle:When this creature dies, return another target artifact card from a graveyard to your hand.",
    ));

    let mark = read(
        "T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigBounce\n\
         SVar:TrigBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Defined$ Self\n\
         Oracle:At the beginning of the end step, return this Aura to its owner's hand.",
    );
    assert!(
        mark.abilities[0]
            .contains("Trigger::StepBegin { step: StepKind::End, whose: PlayerRel::EachPlayer }"),
        "{}",
        mark.abilities[0]
    );

    let keg = read(
        "A:AB$ DestroyAll | Cost$ T Sac<1/CARDNAME> | ValidCards$ Artifact.cmcEQX,Creature.cmcEQX\n\
         SVar:X:Count$CardCounters.FUSE",
    );
    assert!(
        keg.abilities[0].contains("Effect::DestroyAllWithManaValue")
            && keg.abilities[0].contains("Amount::CountersOnSource(counters::FUSE)"),
        "{}",
        keg.abilities[0]
    );

    let taunting = read("K:All creatures able to block CARDNAME do so.");
    assert_eq!(
        taunting.abilities,
        ["static_ability!(Filter::This, Modifier::MustBeBlockedByAllAble)"]
    );

    let golem = read(
        "S:Mode$ Continuous | Affected$ Card.Self+enchanted | AddPower$ 2 | AddToughness$ 2 | AddKeyword$ Flying",
    );
    assert!(
        golem
            .abilities
            .join("\n")
            .contains("Filter::And(&[Filter::This, Filter::Enchanted])"),
        "{:?}",
        golem.abilities
    );

    let glare = read("S:Mode$ Continuous | Affected$ Card.Self | CanBlockAny$ True");
    assert_eq!(
        glare.abilities,
        ["static_ability!(Filter::This, Modifier::CanBlockAnyNumber)"]
    );

    let elder = read(
        "T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigFetch | OptionalDecider$ TriggeredCardController\n\
         SVar:TrigFetch:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | ChangeNum$ 2 | ShuffleNonMandatory$ True\n\
         Oracle:When Yavimaya Elder dies, you may search your library for up to two basic land cards, reveal them, put them into your hand, then shuffle.",
    );
    assert!(
        elder
            .abilities
            .join("")
            .contains("finds: &[Find::HAND, Find::HAND], optional: true"),
        "{:?}",
        elder.abilities
    );

    let foundry = read(
        "A:AB$ ChangeZoneAll | Cost$ 1 T Exile<1/CARDNAME> | Origin$ Graveyard | Destination$ Library | ValidTgts$ Player | ChangeType$ Card | Shuffle$ True",
    );
    assert!(
        foundry.abilities[0].contains("cost!(\"{1}\", TapSelf, ExileSelf)"),
        "{}",
        foundry.abilities[0]
    );

    let enchantress = read(
        "S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$Valid Enchantment",
    );
    assert_eq!(
        enchantress.abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPTByCount(PtCount::OnBattlefield(&Filter::ENCHANTMENT)))"
        ]
    );
}

#[test]
fn attacker_blocked_attacked_you_times_count_arrest_and_event_player_targets_transcode() {
    let grifters = read(
        "T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigDiscard\n\
         SVar:TrigDiscard:DB$ Discard | Defined$ TriggeredDefendingPlayer | Mode$ TgtChoose",
    );
    assert_eq!(
        grifters.abilities,
        [
            "triggered!(Trigger::BecomesBlocked, &[Effect::DiscardForPlayers { who: PlayerRel::DefendingPlayer, count: 1 }])"
        ]
    );

    let mask = read(
        "K:Enchant:Creature\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ X | AddToughness$ X\n\
         SVar:X:Count$Valid Enchantment.Other/Times.2",
    );
    assert_eq!(mask.abilities.len(), 3, "{:?}", mask.abilities);
    assert_eq!(mask.abilities[1], mask.abilities[2]);

    let arrest = read(
        "K:Enchant:Creature\n\
         S:Mode$ CantAttack,CantBlock,CantBeActivated | ValidCard$ Creature.EnchantedBy",
    );
    assert_eq!(
        arrest.abilities,
        [
            "spell!(&[Effect::AttachSelf { target: TargetSpec::Object(&Filter::CREATURE) }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)))",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::CantActivate)"
        ]
    );

    let ballista = read(
        "A:AB$ DealDamage | Cost$ X W T | ValidTgts$ Creature.attacking,Creature.blocking | NumDmg$ X\n\
         SVar:X:Count$xPaid\n\
         Oracle:{X}{W}, {T}: Ballista Squad deals X damage to target attacking or blocking creature.",
    );
    assert!(
        ballista.abilities[0].contains("cost!(\"{X}{W}\", TapSelf)"),
        "{}",
        ballista.abilities[0]
    );

    let briar = read(
        "T:Mode$ Attacks | ValidCard$ Creature | Attacked$ You | TriggerZones$ Battlefield | Execute$ TrigPump\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredAttackerLKICopy | NumAtt$ -1",
    );
    assert!(
        briar.statics.contains("Filter::AttackingYou"),
        "{}",
        briar.statics
    );
    assert!(
        briar.abilities[0].contains("Effect::PumpFilter { filter: &Filter::Reference"),
        "{}",
        briar.abilities[0]
    );

    let wasps = read(
        "T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ TrigDestroy | CombatDamage$ True | OptionalDecider$ You\n\
         SVar:TrigDestroy:DB$ Destroy | ValidTgts$ Artifact.ControlledBy TriggeredTarget",
    );
    assert!(
        wasps.abilities[0].contains("TargetSpec::ObjectOfEventPlayer(&Filter::ARTIFACT)"),
        "{}",
        wasps.abilities[0]
    );
}

#[test]
fn chosen_color_protection_prevent_all_damage_draw_count_and_choose_color_animate_transcode() {
    let blessing = read(
        "K:Flash\n\
         K:Enchant:Creature\n\
         K:ETBReplacement:Other:ChooseColor\n\
         SVar:ChooseColor:DB$ ChooseColor | Defined$ You | AILogic$ MostProminentInHumanDeck | SpellDescription$ As CARDNAME enters, choose a color.\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddKeyword$ Protection:Card.ChosenColor:chosenColor:Card.CardUID_HostCardUID",
    );
    assert_eq!(blessing.enter_modifiers, ["EnterModifier::ChooseColor"]);
    assert!(
        blessing.abilities[1]
            .contains("Filter::And(&[Filter::OfSourceChosenColor, Filter::Not(&Filter::This)])"),
        "{}",
        blessing.abilities[1]
    );

    let cho_manno = read("K:Prevent all damage that would be dealt to CARDNAME.");
    assert_eq!(
        cho_manno.abilities,
        ["static_ability!(Filter::This, Modifier::PreventDamageFrom(&Filter::Any))"]
    );

    let cinder = read(
        "A:AB$ DealDamage | Cost$ X R T Sac<1/CARDNAME> | ValidTgts$ Any | NumDmg$ X | AITgts$ BetterThanSource\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        cinder.abilities[0].contains("cost!(\"{X}{R}\", TapSelf, SacrificeSelf)"),
        "{}",
        cinder.abilities[0]
    );

    let collective = read(
        "A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl",
    );
    assert_eq!(
        collective.abilities,
        [
            "spell!(&[Effect::DrawCards { amount: Amount::CountOf { filter: &Filter::YOUR_CREATURE, zone: ZoneSel::Battlefield } }])"
        ]
    );

    let merfolk = read(
        "A:AB$ ChangeZone | Cost$ U | Origin$ Battlefield | Destination$ Hand | SpellDescription$ Return CARDNAME to its owner's hand.",
    );
    assert_eq!(
        merfolk.abilities,
        ["activated!(cost!(\"{U}\"), &[Effect::bounce(TargetSpec::ThisObject)])"]
    );

    let lens = read(
        "A:AB$ ChooseColor | Cost$ T | Defined$ You | SubAbility$ Animate\n\
         SVar:Animate:DB$ Animate | ValidTgts$ Permanent | Colors$ ChosenColor | OverwriteColors$ True",
    );
    assert_eq!(
        lens.abilities,
        [
            "activated!(Cost::TAP, &[Effect::ChooseColors { subject: ColorSubject::Target, max: 1, duration: Duration::UntilEndOfTurn }], target = Some(TargetSpec::Object(&Filter::Any)))"
        ]
    );
}

#[test]
fn fading_graveyard_name_count_sacrifice_all_and_dominate_transcode() {
    let hydra = read(
        "K:Fading:5\n\
         A:AB$ DealDamage | Cost$ 1 SubCounter<1/FADE> | ValidTgts$ Any | NumDmg$ 1",
    );
    assert_eq!(
        hydra.enter_modifiers,
        ["EnterModifier::WithCounters { kind: counters::FADE, amount: Amount::Fixed(5) }"]
    );
    assert_eq!(
        hydra.abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, &[Effect::IfCondition { condition: Condition::CountersOnSelf(counters::FADE, 1), then: &[Effect::RemoveCounterSelf { kind: counters::FADE, n: 1 }], otherwise: &[Effect::SacrificeSelf] }])",
            "activated!(cost!(\"{1}\", RemoveCounterSelf { kind: counters::FADE, n: 1 }), &[Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::AnyTarget }], target = Some(TargetSpec::AnyTarget))"
        ]
    );

    let ak = read(
        "A:SP$ Draw | Defined$ You | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X\n\
         SVar:X:Count$ValidGraveyard Card.namedAccumulated Knowledge",
    );
    assert!(
        ak.abilities[0].contains("ZoneSel::GraveyardAll")
            && ak
                .statics
                .contains("Filter::Named(\"Accumulated Knowledge\")"),
        "{:?} / {}",
        ak.abilities,
        ak.statics
    );

    let offering = read(
        "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSacAll\n\
         SVar:TrigSacAll:DB$ SacrificeAll | ValidCards$ Creature.YouCtrl",
    );
    assert_eq!(
        offering.abilities,
        ["triggered!(Trigger::ETB, &[Effect::SacrificeAll { filter: &Filter::YOUR_CREATURE }])"]
    );

    let dominate = read(
        "A:SP$ GainControl | ValidTgts$ Creature.cmcLEX | SVar:X:Count$xPaid\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        dominate.abilities[0].contains("Modifier::GainControl")
            && dominate.abilities[0].contains("Duration::Indefinitely"),
        "{:?}",
        dominate.abilities
    );
}

#[test]
fn laccoliths_flowstone_armor_lin_sivvi_and_mossdog_transcode() {
    let laccolith = read(
        "T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigDamage | OptionalDecider$ You\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ X | SubAbility$ DBNoCombatDamage\n\
         SVar:DBNoCombatDamage:DB$ Effect | StaticAbilities$ SNoCombatDamage | Duration$ UntilHostLeavesPlayOrEOT\n\
         SVar:SNoCombatDamage:Mode$ AssignNoCombatDamage | ValidCard$ Card.EffectSource\n\
         SVar:X:Count$CardPower",
    );
    assert_eq!(
        laccolith.abilities,
        [
            "triggered!(Trigger::BecomesBlocked, &[Effect::MayDo { effects: &[Effect::DealDamage { amount: Amount::SourcePower, target: TargetSpec::Object(&Filter::CREATURE) }, Effect::ModifySource { modifier: Modifier::AssignsNoCombatDamage, duration: Duration::UntilEndOfTurn }] }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let armor = read(
        "A:AB$ Pump | Cost$ 3 T | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ -1 | Duration$ UntilUntaps",
    );
    assert_eq!(
        armor.abilities,
        [
            "activated!(cost!(\"{3}\", TapSelf), &[Effect::PumpTarget { power: Amount::Fixed(1), toughness: Amount::NegXFixed(1), keywords: KeywordSet::EMPTY, duration: Duration::WhileSourceTapped { control: false, power_limited: false } }], target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let sivvi = read(
        "A:AB$ ChangeZone | Cost$ 3 | ValidTgts$ Card.YouCtrl | Origin$ Graveyard | Destination$ Library | LibraryPosition$ -1",
    );
    assert!(
        sivvi.abilities[0].contains("Effect::PutOnBottomOfLibraryFromGraveyard"),
        "{:?}",
        sivvi.abilities
    );

    let mossdog = read(
        "T:Mode$ BecomesTarget | ValidTarget$ Card.Self | ValidSource$ SpellAbility.OppCtrl | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert_eq!(
        mossdog.abilities,
        [
            "triggered!(Trigger::TargetedByOpponent { filter: &Filter::This, you: false }, &[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }])"
        ]
    );
}

#[test]
fn off_balance_parallax_wave_rackling_and_rusting_golem_transcode() {
    let off_balance =
        read("A:SP$ Pump | ValidTgts$ Creature | KW$ HIDDEN CARDNAME can't attack or block.");
    assert_eq!(
        off_balance.abilities,
        [
            "spell!(&[Effect::PumpTarget { power: Amount::Fixed(0), toughness: Amount::Fixed(0), keywords: KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK), duration: Duration::UntilEndOfTurn }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let wave = read(
        "K:Fading:5\n\
         A:AB$ ChangeZone | Cost$ SubCounter<1/FADE> | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ ExiledWith | Origin$ Exile | Destination$ Battlefield",
    );
    assert!(
        wave.abilities[1].contains("Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE))")
            && wave.abilities[2].contains("Trigger::LeavesBattlefield(&Filter::This)")
            && wave.abilities[2].contains("Effect::ReturnLinkedToBattlefield"),
        "{:?}",
        wave.abilities
    );

    let rackling = read(
        "T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player.Opponent | TriggerZones$ Battlefield | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
         SVar:X:TriggeredPlayer$CardsInHand/NMinus.3",
    );
    assert!(
        rackling.abilities[0].contains("Amount::Difference")
            && rackling.abilities[0].contains("ZoneSel::HandActivePlayer"),
        "{:?}",
        rackling.abilities
    );

    let golem = read(
        "K:Fading:5\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X\n\
         SVar:X:Count$CardCounters.FADE",
    );
    assert!(
        golem.abilities[1].contains("PtCount::CountersOnThis(counters::FADE)"),
        "{:?}",
        golem.abilities
    );
}

#[test]
fn silkenfist_skyshroud_claim_stronghold_biologist_discipline_viseling_and_wandering_eye_transcode()
{
    let silkenfist = read(
        "T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigUntap\n\
         SVar:TrigUntap:DB$ Untap | Defined$ Self",
    );
    assert_eq!(
        silkenfist.abilities,
        ["triggered!(Trigger::BecomesBlocked, &[Effect::UntapSelf])"]
    );

    let claim = read(
        "A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Forest | ChangeNum$ 2\n\
         Oracle:Search your library for up to two Forest cards, put them onto the battlefield, then shuffle.",
    );
    assert!(
        claim.abilities[0].contains("optional: true"),
        "{:?}",
        claim.abilities
    );

    let biologist = read(
        "A:AB$ Counter | Cost$ U U T Discard<1/Card> | TargetType$ Spell | ValidTgts$ Card.Creature",
    );
    assert!(
        biologist.abilities[0].contains("TargetSpec::Spell(&Filter::CREATURE)"),
        "{:?}",
        biologist.abilities
    );

    let discipline = read(
        "A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBLoseLife\n\
         SVar:DBLoseLife:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.RememberedPlayerCtrl",
    );
    assert_eq!(
        discipline.abilities,
        [
            "spell!(&[Effect::LoseLife { amount: Amount::ControlledCount { who: PlayerRel::Recipient, filter: &Filter::CREATURE }, target: PlayerRel::EachPlayer }])"
        ]
    );

    let viseling = read(
        "T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player.Opponent | TriggerZones$ Battlefield | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
         SVar:X:TriggeredPlayer$CardsInHand/Minus.4",
    );
    assert!(
        viseling.abilities[0].contains("Amount::Difference { left: &Amount::CountOf")
            && viseling.abilities[0].contains("right: &Amount::Fixed(4)"),
        "{:?}",
        viseling.abilities
    );

    let eye = read(
        "S:Mode$ Continuous | AffectedZone$ Hand | MayLookAt$ Player | Description$ Players play with their hands revealed.",
    );
    assert_eq!(
        eye.abilities,
        ["static_ability!(Filter::Any, Modifier::RevealAllHands)"]
    );

    let convalescent = read(
        "T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | LifeTotal$ You | LifeAmount$ LE5 | Execute$ TrigGainLife\n\
         SVar:TrigGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 3 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw",
    );
    assert!(
        convalescent.abilities[0].contains("Condition::LifeAtMost(5)"),
        "{:?}",
        convalescent.abilities
    );

    let gustcloak = read(
        "T:Mode$ AttackerBlocked | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ TrigUntap\n\
         SVar:TrigUntap:DB$ Untap | Defined$ Self | SubAbility$ RemCombat\n\
         SVar:RemCombat:DB$ RemoveFromCombat | Defined$ Self",
    );
    assert!(
        gustcloak.abilities[0].contains("Effect::UntapSelf")
            && gustcloak.abilities[0].contains("Effect::RemoveTargetFromCombat { unblock: false }"),
        "{:?}",
        gustcloak.abilities
    );
}
