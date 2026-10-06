//! Reading mana, animation, searches, pumps and statics.

use super::*;

/// `Produced$ W U` is "add {W}{U}" — two mana at once. `Combo W U` is
/// the choice between them, and reading one as the other would hand a
/// bounce land twice the mana or half of it.
#[test]
fn produced_lists_two_mana_and_combo_offers_a_choice() {
    let both = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ W U");
    assert_eq!(
        both.abilities,
        ["mana_ability!(&[Effect::mana(ManaColor::White, 1), Effect::mana(ManaColor::Blue, 1)])"]
    );
    let either = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W U");
    assert_eq!(
        either.abilities,
        ["mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])"]
    );
}

/// "Spend this mana only to cast a creature spell" is a rider on the
/// mana, and only on spells: `Activated.Hero` restricts an *ability*,
/// which `ManaRestriction` cannot say, so a card printing both stays a
/// stub rather than becoming the half of itself we can express.
#[test]
fn restricted_mana_reads_a_spell_filter_and_only_that() {
    let body = read(
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Creature",
    );
    assert_eq!(
        body.abilities,
        [
            "mana_ability!(&[Effect::mana_of_any_color().restricted(&Filter::CREATURE, SpendRider::None)])"
        ]
    );

    let script = parse(
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Hero,Activated.Hero",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("`Mana.RestrictValid` beyond a spell")
    );
}

/// A manland: one printed sentence, four layers. "It's still a land" is
/// why the types are *added*, and CR 613.1 is why each layer is its own
/// effect rather than one lump.
#[test]
fn animate_becomes_one_continuous_effect_per_layer() {
    let body = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Animate | Cost$ 1 G | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature,Goblin | Colors$ Green | OverwriteColors$ True | Keywords$ Trample",
    );
    let a = body.abilities.join("");
    // No layer is written: `Effect::continuous` derives it from the
    // modifier (CR 613.1), so the modifier *is* the layer claim here.
    for expected in [
        "Effect::continuous(&Filter::This, Modifier::AddType(TypeSet::CREATURE), Duration::UntilEndOfTurn)",
        "Modifier::AddSubtype(subtypes::creature::GOBLIN)",
        "Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(&[Color::Green])), Duration::UntilEndOfTurn)",
        "Effect::continuous(&Filter::This, Modifier::AddKeyword(KeywordSet::TRAMPLE), Duration::UntilEndOfTurn)",
        "Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::UntilEndOfTurn)",
    ] {
        assert!(a.contains(expected), "missing `{expected}` in {a}");
    }
    assert!(!a.contains("RemoveType"), "it's still a land");

    // `Filter::This` binds to the first target when the chain has one,
    // so an animate that also targets would animate the wrong
    // permanent. Refuse rather than guess which was meant.
    let script = parse(
        "Name:X\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Land | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("`Animate` of something other than the source")
    );

    // The Laces: the target is what `Filter::This` binds to, the stack
    // is a zone it may be in, and "becomes" lasts the game.
    let lace = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Animate | Colors$ Red | OverwriteColors$ True | ValidTgts$ Card \
         | TgtZone$ Stack,Battlefield | Duration$ Permanent",
    );
    let a = lace.abilities.join("");
    assert!(
        a.contains(
            "Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(\
             &[Color::Red])), Duration::Indefinitely)"
        ),
        "{a}"
    );
    assert!(
        a.contains("TargetSpec::StackOrBattlefield(&Filter::Any)"),
        "{a}"
    );
    let exiled = parse(
        "Name:X\nTypes:Instant\n\
         A:SP$ Animate | Colors$ Red | ValidTgts$ Card | TgtZone$ Exile",
    );
    assert_eq!(
        refusal_reason(&exiled, &cats(), None).as_deref(),
        Some("a target in `TgtZone$ Exile`")
    );
}

/// The bounce land's sentence: nobody is targeted, the ability's
/// controller picks one of their own lands, and it goes to its owner's
/// hand.
///
/// Each half is struck on its own below, because a reader that emitted
/// this shape for *any* untargeted `Battlefield` → `Hand` line would
/// pass the first assertion and be wrong about four other cards.
#[test]
fn a_hidden_battlefield_to_hand_is_a_choice_among_your_own() {
    let body = read(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | Mandatory$ True | ChangeType$ Land.YouCtrl \
         | AILogic$ NeverBounceItself | SpellDescription$ Return a land you control.",
    );
    assert_eq!(
        body.abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::ReturnChosenToHand { who: PlayerRel::You, \
             filter: &Filter::YOUR_LAND }])"
        ]
    );

    // `Mandatory$ True` is what separates "return a land you control"
    // from "you may return a land you control", and the effect can only
    // say the first. Without the word it is refused rather than read as
    // either — the same bargain a library search makes about its count.
    let silent = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | ChangeType$ Land.YouCtrl",
    );
    assert_eq!(
        refusal_reason(&silent, &cats(), None).as_deref(),
        Some("`ChangeZone` chosen without `Mandatory$`")
    );

    // `Hidden$` is the discriminator, and without it the line falls
    // through to the report it had before this rule existed.
    let open = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Mandatory$ True | ChangeType$ Land.YouCtrl",
    );
    assert_eq!(
        refusal_reason(&open, &cats(), None).as_deref(),
        Some("`ChangeZone` with neither a target nor `Defined$`")
    );

    // Arid Archway: the return is this rule, and the surveil hanging
    // off it reads the permanent that came back. Nothing claims
    // `RememberLKI$`, so the card is refused whole rather than written
    // as a bounce land that forgot half its sentence.
    let remembered = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | Mandatory$ True | ChangeType$ Land.YouCtrl | RememberLKI$ True",
    );
    assert_eq!(
        refusal_reason(&remembered, &cats(), None).as_deref(),
        Some("unclaimed parameter `ChangeZone.RememberLKI`")
    );

    // A second permanent is a different rule, and it says so rather
    // than writing a card that returns one of them.
    let two = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return two lands you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | Mandatory$ True | ChangeNum$ 2 | ChangeType$ Land.YouCtrl",
    );
    assert_eq!(
        refusal_reason(&two, &cats(), None).as_deref(),
        Some("`ChangeZone` returning 2 chosen permanents")
    );
}

/// The two enters-tapped sentences that count players, and the four
/// ways a line is refused instead.
///
/// Both directions are struck on their own, because the whole risk in
/// this rule is reading one of them with the other's sign: the
/// comparator says when the land comes down *tapped* and the card prints
/// when it does not.
#[test]
fn an_enters_tapped_condition_may_count_players() {
    let head = "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield \
         | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n";

    // Luxury Suite: taps while you have fewer than two opponents, which
    // is "unless you have two or more opponents".
    let crowd = read(&format!(
        "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
         | ConditionCheckSVar$ Y | ConditionSVarCompare$ LT2\n\
         SVar:Y:PlayerCountOpponents$Amount"
    ));
    assert_eq!(
        crowd.enter_modifiers,
        ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
    );

    // Razortrap Gorge: taps while the lowest life total is above
    // thirteen, which is "unless a player has 13 or less life".
    let unlucky = read(&format!(
        "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
         | ConditionCheckSVar$ X | ConditionSVarCompare$ GT13\n\
         SVar:X:PlayerCountPlayers$LowestLifeTotal"
    ));
    assert_eq!(
        unlucky.enter_modifiers,
        ["EnterModifier::TappedUnlessSomeoneAtOrBelow { life: 13 }"]
    );

    // `LE`/`GE` are the same sentences off by one, and the arithmetic is
    // asserted rather than assumed.
    let off_by_one = read(&format!(
        "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
         | ConditionCheckSVar$ Y | ConditionSVarCompare$ LE1\n\
         SVar:Y:PlayerCountOpponents$Amount"
    ));
    assert_eq!(
        off_by_one.enter_modifiers,
        ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
    );

    for (tail, svar, why) in [
        // The opponent count with the life total's comparator. Read with
        // a flipped sign this would be a land that enters untapped in
        // every duel; refused, it is a stub.
        (
            "| ConditionCheckSVar$ Y | ConditionSVarCompare$ GT1",
            "SVar:Y:PlayerCountOpponents$Amount",
            "an enter-tapped condition counting `PlayerCountOpponents$Amount` `GT1`",
        ),
        // And the life total with the count's comparator.
        (
            "| ConditionCheckSVar$ X | ConditionSVarCompare$ LT13",
            "SVar:X:PlayerCountPlayers$LowestLifeTotal",
            "an enter-tapped condition counting `PlayerCountPlayers$LowestLifeTotal` `LT13`",
        ),
        // A count this rule has never seen: named by what it counts, not
        // by the letter the corpus wrote.
        (
            "| ConditionCheckSVar$ Z | ConditionSVarCompare$ EQ0",
            "SVar:Z:Count$Valid Creature.YouCtrl",
            "an enter-tapped condition counting `Count$Valid Creature.YouCtrl` `EQ0`",
        ),
        // The comparison missing altogether.
        (
            "| ConditionCheckSVar$ Y",
            "SVar:Y:PlayerCountOpponents$Amount",
            "replacement `Moved` counting an SVar with no comparison",
        ),
    ] {
        let script = parse(&format!(
            "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True {tail}\n{svar}"
        ));
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(why),
            "{tail}"
        );
    }
}

/// A fetchland: `Origin$ Library` is a *search*, not a zone change with
/// a hidden target — a card in a library cannot be targeted at all.
#[test]
fn a_library_change_zone_is_a_search() {
    let body = read(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Forest",
    );
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(TapSelf, SacrificeSelf), &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD], optional: false }])"
        ]
    );

    // `finds` is positional and its length is the count, so two cards
    // are the same `Find` twice — and `Tapped$ True` is a different
    // `Find`, not a flag beside it.
    let two = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeNum$ 2 | Optional$ True | ChangeType$ Forest",
    );
    assert!(
        two.abilities.join("").contains(
            "finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED], optional: true"
        ),
        "{:?}",
        two.abilities
    );

    // The same line with neither word. `ChangeNum$` alone does not say
    // whether two is a maximum or a requirement, and the two are
    // different cards, so it is refused rather than guessed at.
    let ambiguous = parse(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeNum$ 2 | ChangeType$ Forest",
    );
    assert_eq!(
        refusal_reason(&ambiguous, &cats(), None).as_deref(),
        Some("`ChangeZone` finding several cards without saying whether that is a maximum")
    );

    // `Mandatory$ True` is the other word, and it says the opposite of
    // `Optional$ True` rather than merely failing to say it.
    let must = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeNum$ 2 | Mandatory$ True | ChangeType$ Card",
    );
    assert!(
        must.abilities
            .join("")
            .contains("finds: &[Find::HAND, Find::HAND], optional: false"),
        "{:?}",
        must.abilities
    );

    // A count of one needs no word at all, which is what keeps every
    // fetchland in the pool readable.
    let one = read(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land",
    );
    assert!(
        one.abilities.join("").contains("optional: false"),
        "{:?}",
        one.abilities
    );
}

/// `ChangeTypeDesc$` restates the filter beside it for a human, and is
/// claimed only while that filter is there to carry the meaning.
///
/// Not a `PROSE_KEYS` entry, and this is the difference: alone on a
/// line it is the only thing said about what is being found, and a
/// reader that dropped it would be inventing a filter.
#[test]
fn a_search_label_is_claimed_only_beside_the_filter_it_restates() {
    let labelled = read(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land",
    );
    assert!(
        labelled
            .abilities
            .join("")
            .contains("finds: &[Find::BATTLEFIELD_TAPPED]"),
        "{:?}",
        labelled.abilities
    );

    let bare = parse(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeTypeDesc$ basic land",
    );
    assert_eq!(
        refusal_reason(&bare, &cats(), None).as_deref(),
        Some("unreadable value in `ChangeZone`"),
        "a label with no filter beside it says nothing a card can be built from"
    );
}

/// The load-bearing rule: an unread parameter, an unread effect, an
/// unread line kind or an unread keyword all refuse the whole card. Each
/// Giant Growth: the commonest shape in the whole script corpus.
#[test]
fn a_pump_binds_to_the_target_and_keeps_its_sign() {
    let body = read(
        "Name:Giant Growth\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | \
         SpellDescription$ gets +3/+3.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::PumpTarget"), "{text}");
    assert!(text.contains("power: Amount::Fixed(3)"), "{text}");
    assert!(text.contains("keywords: KeywordSet::EMPTY"), "{text}");

    // A shrink is the same effect with the sign in the variant,
    // because `Amount::Fixed` cannot hold one.
    let body = read(
        "Name:Weakness\nManaCost:B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -2 | NumDef$ -1\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("power: Amount::NegXFixed(2)"), "{text}");
    assert!(text.contains("toughness: Amount::NegXFixed(1)"), "{text}");
}

/// A pump that **counts**, both ways round.
///
/// The letter is not the number: `NumAtt$ -X` with `SVar:X:Count$xPaid`
/// is the X a player announced and reads as `Amount::NegX`, while
/// `Count$Valid Artifact.YouCtrl` is a board count and has nothing to do
/// with an announced number at all. Both were refused, because a pump
/// asked [`amount`] and [`amount`] reads no count; a mana line has asked
/// [`Tx::counted_amount`] the whole time.
///
/// The positive side is the larger half — 148 reference scripts against
/// 26 — and comes from the same fallthrough, so refusing it would have
/// been a guard written for no reason a card could state.
#[test]
fn a_pump_counts_in_either_direction() {
    // Irradiate: "-1/-1 until end of turn for each artifact you control".
    let body = read(
        "Name:Irradiate\nManaCost:3 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::PumpTarget"), "{text}");
    assert_eq!(
        text.matches("Amount::Negated(&Amount::CountOf {").count(),
        2,
        "both sides of the pump negate the same count: {text}"
    );

    // Wirewood Pride's shape — "+X/+X, where X is the number of Elves" —
    // over a subtype this fixture's catalog knows. The same reading with
    // no sign in front of it.
    let body = read(
        "Name:Goblin Pride\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ X | NumDef$ X\n\
         SVar:X:Count$Valid Goblin\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("power: Amount::CountOf {"), "{text}");
    assert!(
        !text.contains("Negated"),
        "nothing here counts downwards: {text}"
    );

    // And a count this reader still cannot say is still a refusal, named
    // by what it resolves *through*. Blood Lust is the card; the
    // honest-stub rule does not care which side of the sign it is on.
    assert_eq!(
        refusal_reason(
            &parse(
                "Name:Blood Lust\nManaCost:B\nTypes:Instant\n\
                 A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
                 SVar:X:Count$Compare T GE4.4.T\n"
            ),
            &cats(),
            None
        )
        .as_deref(),
        Some("pump amount `-X` = `Count$Compare T GE4.4.T`"),
        "a refused pump says what its letter resolves through"
    );
}

/// "Doesn't untap during your untap step" is an `R:` line in the
/// reference and a **static ability** here, because CR 613.11 makes it a
/// continuous effect modifying a game rule rather than a replacement of
/// any event. Basalt Monolith is the card, and the whole of it is read:
/// the rule, the mana ability and the way out.
#[test]
fn a_cant_happen_untap_replacement_is_a_static_ability() {
    let body = read(
        "Name:Basalt Monolith\nManaCost:3\nTypes:Artifact\n\
         R:Event$ Untap | ValidCard$ Card.Self | ValidStepTurnToController$ You | \
         Layer$ CantHappen | Description$ This artifact doesn't untap.\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 3\n\
         A:AB$ Untap | Cost$ 3\n",
    );
    assert_eq!(
        body.abilities,
        [
            "static_ability!(Filter::This, Modifier::DoesNotUntap)",
            "mana_ability!(&[Effect::mana(ManaColor::Colorless, 3)])",
            "activated!(cost!(\"{3}\"), &[Effect::UntapSelf])",
        ]
    );
    assert!(
        body.statics.is_empty(),
        "`Card.Self` is `Filter::This` and needs no `static`: {}",
        body.statics
    );
}

/// `Defined$ Self` is the source, not the target, even inside an
/// ability that has one.
#[test]
fn a_pump_on_itself_is_not_a_pump_on_the_target() {
    let body = read(
        "Name:X\nManaCost:R\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ Pump | Cost$ R | Defined$ Self | NumAtt$ +1 | NumDef$ +0\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::PumpFilter"), "{text}");
    assert!(text.contains("filter: &Filter::This"), "{text}");
}

/// A static's `IsPresent$` is the condition it exists under, and
/// `GainControl$ You` is layer 2 to the static's controller.
#[test]
fn a_static_holds_while_its_clause_does_and_can_give_control() {
    // Sedge Troll, with a land the test catalog knows.
    let body = read(
        "Name:X\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 1 | AddToughness$ 1 | \
         IsPresent$ Mountain.YouCtrl | Description$ gets +1/+1.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("condition = Some(Condition::ControlCount(&CHECK"),
        "{text}"
    );
    assert!(text.trim_end().ends_with("))"), "{text}");

    // Control Magic: layer 2, to the static's controller.
    let body = read(
        "Name:X\nManaCost:2 U U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | GainControl$ You | \
         Description$ You control enchanted creature.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("static_ability!(Filter::AttachedToBySource, Modifier::GainControl)"),
        "{text}"
    );

    // Control given to anyone else is not a sentence it can write.
    let script = parse(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | GainControl$ Opponent\n",
    );
    assert!(transcode(&script, &cats(), None).is_none());
}

/// Disrupting Scepter: the target player chooses the card, and the
/// ability is activated only during its controller's turn.
#[test]
fn a_chosen_discard_on_your_turn_only() {
    let body = read(
        "Name:X\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Discard | Cost$ 3 T | ValidTgts$ Player | NumCards$ 1 | Mode$ TgtChoose | \
         PlayerTurn$ True | SpellDescription$ Target player discards a card.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 1 }"),
        "{text}"
    );
    assert!(
        text.contains("condition = Some(Condition::YourTurn)"),
        "{text}"
    );
}

/// Psionic Blast: `DamageMap$` gathers, `DamageResolve` deals.
#[test]
fn gathered_damage_is_dealt_in_one_resolution() {
    let body = read(
        "Name:X\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 4 | DamageMap$ True | \
         SubAbility$ DBDealDamage | SpellDescription$ 4 to any target and 2 to you.\n\
         SVar:DBDealDamage:DB$ DealDamage | Defined$ You | NumDmg$ 2 | \
         SubAbility$ DBDamageResolve\n\
         SVar:DBDamageResolve:DB$ DamageResolve\n",
    );
    let text = body.abilities.join("\n");
    assert_eq!(text.matches("Effect::DealDamage").count(), 2, "{text}");
    assert!(
        text.contains("TargetSpec::Player(PlayerRel::You)"),
        "{text}"
    );
}

/// Zombie Master: a granted activated ability, "this permanent" being
/// the one that has it; a granted ability that targets is refused.
#[test]
fn a_granted_ability_is_the_holders_own() {
    let body = read(
        "Name:X\nManaCost:1 B B\nTypes:Creature Goblin\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Card.Goblin+Other | AddAbility$ Regenerate | \
         Description$ Other Goblins have regenerate.\n\
         SVar:Regenerate:AB$ Regenerate | Cost$ B | SpellDescription$ Regenerate this permanent.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Modifier::GrantActivated {"), "{text}");
    assert!(text.contains("cost: cost!(\"{B}\")"), "{text}");
    assert!(text.contains("TargetSpec::ThisObject"), "{text}");
    assert!(text.contains("mana_ability: false"), "{text}");

    assert!(refused(
        "Name:X\nTypes:Creature Goblin\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Creature.Goblin | AddAbility$ Ping\n\
         SVar:Ping:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1\n"
    ));
}

/// Lifetap and Psychic Venom: a `Taps` trigger on another permanent, and
/// "that land's controller" read off it.
#[test]
fn a_tapped_trigger_reads_any_permanent_and_its_controller() {
    let venom = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         T:Mode$ Taps | ValidCard$ Card.AttachedBy | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 2",
    );
    let a = venom.abilities.join("");
    assert!(
        a.contains("Trigger::BecomesTapped(&Filter::AttachedToBySource)"),
        "{a}"
    );
    assert!(
        a.contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
        "{a}"
    );
    let lifetap = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Taps | ValidCard$ Forest.OppCtrl | TriggerZones$ Battlefield | Execute$ G\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1",
    );
    assert!(
        lifetap
            .abilities
            .join("")
            .contains("Trigger::BecomesTapped(&TRIGGER"),
        "{:?}",
        lifetap.abilities
    );
}

/// Disintegrate and Magma Spray: "if it's a creature, it can't be
/// regenerated this turn, and if it would die this turn, exile it
/// instead", on an any-target and on a creature target. "A creature
/// dealt damage this way" (`Remembered`), a condition on the rider and
/// a player target are refused.
#[test]
fn exile_if_it_dies_and_no_regeneration_read_the_lines_target() {
    let disintegrate = read(
        "Name:X\nTypes:Sorcery\nManaCost:X R\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SubAbility$ E | \
         ReplaceDyingDefined$ ThisTargetedCard.Creature\n\
         SVar:E:DB$ Effect | RememberObjects$ ParentTarget | ForgetOnMoved$ Battlefield | \
         StaticAbilities$ NoRegen | IsCurse$ True | ConditionDefined$ ParentTarget | \
         ConditionPresent$ Creature | AILogic$ CantRegenerate\n\
         SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered | \
         Description$ It can't be regenerated.\n\
         SVar:X:Count$xPaid",
    );
    let a = disintegrate.abilities.join("");
    assert!(
        a.contains(
            "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: \
             &[Effect::ExileIfDiesThisTurn { target: TargetSpec::AnyTarget }] }"
        ),
        "{a}"
    );
    assert!(
        a.contains(
            "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: \
             &[Effect::CantBeRegeneratedThisTurn { target: TargetSpec::AnyTarget }] }"
        ),
        "{a}"
    );
    let spray = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | ReplaceDyingDefined$ Targeted",
    );
    let a = spray.abilities.join("");
    assert!(
        a.contains("Effect::ExileIfDiesThisTurn { target: TargetSpec::Object("),
        "{a}"
    );
    assert!(!a.contains("IfTargetMatches"), "{a}");
    for refused_line in [
        "A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | ReplaceDyingDefined$ Remembered.Creature",
        "A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3 | ReplaceDyingDefined$ Targeted",
        "A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | \
         ReplaceDyingDefined$ ThisTargetedCard.Creature | ReplaceDyingCondition$ Kicked",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Instant\n{refused_line}")),
            "{refused_line}"
        );
    }
    // "A creature dealt damage this way can't be regenerated"
    // (Incinerate) asks whether damage was dealt.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SubAbility$ E | RememberDamaged$ True\n\
         SVar:E:DB$ Effect | RememberObjects$ Remembered.Creature | ForgetOnMoved$ Battlefield | \
         StaticAbilities$ NoRegen | IsCurse$ True\n\
         SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered"
    ));
}

/// Wheel of Fortune, Timetwister and Natural Selection: whose hand,
/// whose graveyard, whose library. A library position, a random pick
/// and a count the player announced are refused.
#[test]
fn whole_hands_graveyards_and_another_players_library_read_whose() {
    let wheel = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ Discard | Mode$ Hand | Defined$ Player | SubAbility$ D\n\
         SVar:D:DB$ Draw | Defined$ Player | NumCards$ 7",
    );
    let a = wheel.abilities.join("");
    assert!(
        a.contains("Effect::DiscardHand { who: PlayerRel::EachPlayer }"),
        "{a}"
    );
    let twister = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand,Graveyard | \
         Destination$ Library | Shuffle$ True | UseAllOriginZones$ True",
    );
    let a = twister.abilities.join("");
    assert!(
        a.contains(
            "Effect::ShuffleIntoLibrary { who: PlayerRel::EachPlayer, hand: true, \
             graveyard: true }"
        ),
        "{a}"
    );
    let feldon = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Player | ChangeType$ Card | Origin$ Graveyard | \
         Destination$ Library | Shuffle$ True",
    );
    let a = feldon.abilities.join("");
    assert!(
        a.contains(
            "Effect::ShuffleIntoLibrary { who: PlayerRel::Chosen, hand: false, \
             graveyard: true }"
        ),
        "{a}"
    );
    let selection = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ RearrangeTopOfLibrary | ValidTgts$ Player | NumCards$ 3 | MayShuffle$ True",
    );
    let a = selection.abilities.join("");
    assert!(
        a.contains("Effect::ReorderTopLibraryOf { who: PlayerRel::Chosen, count: 3 }"),
        "{a}"
    );
    assert!(
        a.contains(
            "Effect::MayDo { effects: &[Effect::ShuffleLibrary { who: PlayerRel::Chosen }] }"
        ),
        "{a}"
    );
    let mine = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ RearrangeTopOfLibrary | Cost$ 1 | Defined$ You | NumCards$ 3",
    );
    assert!(
        mine.abilities
            .join("")
            .contains("Effect::ReorderTopLibrary { count: 3 }"),
        "{:?}",
        mine.abilities
    );
    for refused_line in [
        "A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand,Graveyard | \
         Destination$ Library | Shuffle$ True | Random$ True",
        "A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | \
         Destination$ Library | LibraryPosition$ -1",
        "A:SP$ RearrangeTopOfLibrary | Defined$ You | NumCards$ X",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Sorcery\n{refused_line}")),
            "{refused_line}"
        );
    }
}

/// Stone Giant: "toughness less than this creature's power" only where
/// `X` is the source's power, and "destroy that creature at the
/// beginning of the next end step" only on a targeted pump.
#[test]
fn a_comparison_with_the_sources_power_and_a_destroy_at_the_next_end_step() {
    let giant = read(
        "Name:X\nTypes:Creature\nPT:3/4\n\
         A:AB$ Pump | Cost$ T | ValidTgts$ Creature.YouCtrl+toughnessLTX | KW$ Flying | \
         AtEOT$ Destroy\n\
         SVar:X:Count$CardPower",
    );
    let a = giant.abilities.join("");
    assert!(
        a.contains(
            "Effect::AtNextEndStep { effects: &[Effect::destroy(TargetSpec::EventObject)] }"
        ),
        "{a}"
    );
    assert!(
        giant
            .statics
            .contains("Filter::ToughnessLessThanSourcePower"),
        "{}",
        giant.statics
    );
    for refused_card in [
        // `X` is not the source's power.
        "A:AB$ Pump | Cost$ T | ValidTgts$ Creature.toughnessLTX | KW$ Flying\n\
         SVar:X:Count$Valid Island.YouCtrl",
        // Another delayed sentence, and a delayed one about no target.
        "A:AB$ Pump | Cost$ T | ValidTgts$ Creature | KW$ Haste | AtEOT$ Sacrifice",
        "A:AB$ Pump | Cost$ R | Defined$ Self | NumAtt$ +1 | AtEOT$ Destroy",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Creature\nPT:3/4\n{refused_card}")),
            "{refused_card}"
        );
    }
}

/// Stasis: "Players skip their untap steps" is every player's untap step,
/// from the battlefield; a plane's skip from the command zone, another
/// step, and a skip for one player refuse.
#[test]
fn a_skipped_untap_step_is_every_players() {
    let body = read(
        "Name:X\nTypes:Enchantment\n\
         R:Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Untap | Skip$ True | \
         Description$ Players skip their untap steps.",
    );
    assert_eq!(
        body.abilities,
        vec![
            "static_ability!(Filter::Any, Modifier::SkipUntapStep { who: PlayerRel::EachPlayer })"
        ]
    );
    for refused_line in [
        "R:Event$ BeginPhase | ActiveZones$ Command | Phase$ Untap | Skip$ True",
        "R:Event$ BeginPhase | Phase$ Draw | Skip$ True",
        "R:Event$ BeginPhase | Phase$ Untap | Skip$ True | ValidPlayer$ You",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Enchantment\n{refused_line}")),
            "{refused_line}"
        );
    }
}

/// "Can attack as though it didn't have defender" and "…as though it
/// had haste", on the creatures the line names; naming what may be
/// attacked instead is another sentence and refuses.
#[test]
fn an_attack_as_though_is_a_permission_on_the_named_creatures() {
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CanAttackIfHaste | ValidCard$ Creature.EnchantedBy | Description$ …",
    );
    assert!(
        body.abilities.iter().any(|a| a
            == "static_ability!(Filter::And(&[Filter::CREATURE, \
                Filter::AttachedToBySource]), Modifier::AttacksAsThoughHaste)"),
        "{:?}",
        body.abilities
    );
    let body = read(
        "Name:X\nTypes:Creature\nPT:0/4\nK:Defender\n\
         S:Mode$ CanAttackDefender | ValidCard$ Card.Self | Description$ …",
    );
    assert!(
        body.abilities
            .iter()
            .any(|a| a == "static_ability!(Filter::This, Modifier::AttacksDespiteDefender)"),
        "{:?}",
        body.abilities
    );
    assert!(refused(
        "Name:X\nTypes:Creature\nPT:1/1\n\
         S:Mode$ CanAttackIfHaste | ValidTarget$ Opponent | Description$ …"
    ));
}

/// "Enchanted land is a Swamp" and "target land becomes a Forest": CR
/// 305.7's setting of a land's subtype, from a static ability and from
/// an `Animate`, and nothing but one basic land type is read that way.
#[test]
fn a_land_set_to_a_basic_land_type_is_read_as_one() {
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Mountain | \
         RemoveLandTypes$ True | Description$ …",
    );
    assert!(
        body.abilities.iter().any(|a| a
            == "static_ability!(Filter::AttachedToBySource, \
                Modifier::SetLandType(subtypes::land::MOUNTAIN))"),
        "{:?}",
        body.abilities
    );
    let body = read(
        "Name:X\nTypes:Creature\nPT:1/1\n\
         A:AB$ Animate | Cost$ T | ValidTgts$ Land | Types$ Forest | \
         RemoveLandTypes$ True | Duration$ UntilHostLeavesPlay | SpellDescription$ …",
    );
    assert!(
        body.abilities.iter().any(|a| a.contains(
            "Effect::continuous(&Filter::This, Modifier::SetLandType(subtypes::land::FOREST), \
             Duration::WhileSourceOnBattlefield)"
        )),
        "{:?}",
        body.abilities
    );
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         K:ETBReplacement:Other:DBChooseBasic\n\
         SVar:DBChooseBasic:DB$ ChooseType | Type$ Basic Land | SpellDescription$ …\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
         RemoveLandTypes$ True | Description$ …",
    );
    assert_eq!(body.enter_modifiers, ["EnterModifier::ChooseBasicLandType"]);
    assert!(
        body.abilities
            .iter()
            .any(|a| a
                == "static_ability!(Filter::AttachedToBySource, Modifier::SetLandTypeToChosen)"),
        "{:?}",
        body.abilities
    );
    assert!(refused(
        "Name:X\nTypes:Enchantment\n\
         K:ETBReplacement:Other:DBChoose\n\
         SVar:DBChoose:DB$ ChooseType | Type$ Creature | SpellDescription$ …"
    ));
    // A chosen type nothing asked for as the card entered.
    for refused_line in [
        "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
         RemoveLandTypes$ True | Description$ …",
        "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Desert | \
         RemoveLandTypes$ True | Description$ …",
        "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Island Swamp | \
         RemoveLandTypes$ True | Description$ …",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n{refused_line}"
            )),
            "{refused_line}"
        );
    }
}

/// Smoke's and Winter Orb's "players can't untap more than one …
/// during their untap steps": a limit on the players the line affects,
/// with the Orb's "as long as this is untapped" as the static's
/// condition. Another keyword beside it, another player, or a key the
/// reader does not claim refuses.
#[test]
fn an_untap_limit_is_read_for_the_players_it_names() {
    let body = read(
        "Name:X\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Creature:1 | \
         Description$ Players can't untap more than one creature during their untap steps.",
    );
    assert_eq!(
        body.abilities,
        vec![
            "static_ability!(Filter::Any, Modifier::UntapAtMost { who: PlayerRel::EachPlayer, \
             of: &Filter::CREATURE, count: 1 })"
        ]
    );
    let body = read(
        "Name:X\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Player.Opponent | AddKeyword$ UntapAdjust:Land:2 | \
         IsPresent$ Card.Self+untapped | Description$ …",
    );
    assert_eq!(body.abilities.len(), 1);
    assert!(
        body.abilities[0].contains("who: PlayerRel::EachOpponent")
            && body.abilities[0].contains("count: 2")
            && body.abilities[0].contains("condition = Some(Condition::SourceMatches(&CHECK"),
        "{}",
        body.abilities[0]
    );
    for refused_line in [
        "S:Mode$ Continuous | Affected$ Creature | AddKeyword$ UntapAdjust:Land:1",
        "S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Land:one",
        "S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Land:1 | \
         AddHiddenKeyword$ Shroud",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Enchantment\n{refused_line}")),
            "{refused_line}"
        );
    }
}
