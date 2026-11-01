//! Reading mana, animation, searches, pumps and statics.

use super::*;

#[test]
fn repeated_target_declarations_keep_independent_pump_slots() {
    // Synthetic grammar fixture, not a corpus card: distinct numbers make
    // accidentally moving an instruction to the other slot observable.
    for second in ["Creature", "Creature.YouCtrl"] {
        let text = format!(
            "Name:Two separate gifts\nTypes:Instant\nManaCost:1 G\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 2 | SubAbility$ Next\n\
             SVar:Next:DB$ Pump | ValidTgts$ {second} | NumDef$ 5\n"
        );
        let body = read(&text);
        let ability = &body.abilities[0];
        assert!(
            ability.contains("Effect::PumpTarget { power: Amount::Fixed(2)"),
            "{ability}"
        );
        assert!(
            ability.contains("Effect::PumpTargetSlot { slot: TargetSlot::Second,"),
            "{ability}"
        );
        assert!(ability.contains("toughness: Amount::Fixed(5)"), "{ability}");
        assert!(
            ability.contains("second_targets = Some(TargetReq::one("),
            "{ability}"
        );
    }
}

#[test]
fn repeated_targets_without_slot_support_are_refused_even_when_types_match() {
    for next in [
        "DB$ Destroy | ValidTgts$ Creature",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | SubAbility$ Third",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ X",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | UnknownRule$ True",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | Duration$ Permanent",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | TgtNum$ 2",
    ] {
        let text = format!(
            "Name:Independent target boundary\nTypes:Instant\nManaCost:2 G\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 4 | SubAbility$ Next\n\
             SVar:Next:{next}\nSVar:Third:DB$ Pump | ValidTgts$ Creature | NumDef$ 2\n"
        );
        assert!(refused(&text), "independent target collapsed: {text}");
    }
}

#[test]
fn a_defined_target_continuation_does_not_ask_for_another_target() {
    let body = read(
        "Name:One gift in two parts\nTypes:Instant\nManaCost:1 G\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 2 | SubAbility$ Next\n\
         SVar:Next:DB$ Pump | Defined$ Targeted | NumDef$ 5\n",
    );
    assert!(!body.abilities[0].contains("second_targets"));
    assert_eq!(body.abilities[0].matches("Effect::PumpTarget {").count(), 2);
}

#[test]
fn independent_counter_targets_are_refused_until_counter_slots_are_supported() {
    let head = "Name:Separate counters\nTypes:Instant\nManaCost:2 G\n\
                A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 2";
    let single = read(head);
    assert!(single.abilities[0].contains("Effect::AddCounter"));
    let pair = format!(
        "{head} | SubAbility$ Next\n\
         SVar:Next:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 4\n"
    );
    assert!(
        refused(&pair),
        "two target clauses must not collapse into one"
    );
    assert_eq!(
        refusal_reason(&parse(&pair), &cats(), None).as_deref(),
        Some("unsupported independent target in one chain")
    );
}

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
/// dealt damage this way" uses actual recipient memory instead; an unread
/// condition on the older target-based rider and a player target are refused.
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
    // Actual recipients differ from original targets when damage redirects.
    let recipient_restriction = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SubAbility$ E | RememberDamaged$ True\n\
         SVar:E:DB$ Effect | RememberObjects$ Remembered.Creature | ForgetOnMoved$ Battlefield | \
         StaticAbilities$ NoRegen | IsCurse$ True\n\
         SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered",
    );
    let effects = recipient_restriction.abilities.join("");
    assert!(effects.contains("Effect::DamagedThisWayCantBeRegenerated"));
    assert!(!effects.contains("Effect::CantBeRegeneratedThisTurn"));
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

#[test]
fn bronze_horse_damage_reader_requires_exact_source_and_other_creature_predicates() {
    let script = "Name:X\nTypes:Artifact Creature Goblin\nPT:4/4\nR:Event$ DamageDone | Prevent$ True | ValidCause$ Spell.IsTargeting Self | ValidTarget$ Creature.Self | CauseIsSource$ True | IsPresent$ Creature.YouCtrl+Other";
    let body = read(script);
    assert!(body.abilities[0].contains("SpellTargetingAffected"));
    assert!(body.abilities[0].contains("Condition::ControlCount(&Filter::CREATURE, 2)"));
    assert!(!body.abilities[0].contains("Filter::Another"));
    for (from, to) in [
        ("Spell.IsTargeting Self", "Ability.IsTargeting Self"),
        ("CauseIsSource$ True", "CauseIsSource$ False"),
        ("Creature.YouCtrl+Other", "Creature.YouCtrl"),
        ("Creature.Self", "Creature.YouCtrl"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
    assert!(refused(&format!("{script} | IsCombat$ True")));
}
#[test]
fn johan_reader_keeps_whole_optional_source_bound_combat_chain() {
    let script = "Name:X\nTypes:Creature Goblin\nPT:5/4\nT:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ Stop | TriggerZones$ Battlefield | OptionalDecider$ You\nSVar:Stop:DB$ Pump | Defined$ Self | KW$ CARDNAME can't attack. | Duration$ UntilEndOfCombat | SubAbility$ Rule\nSVar:Rule:DB$ Effect | StaticAbilities$ Untapped | Duration$ UntilEndOfCombat\nSVar:Untapped:Mode$ AttackVigilance | ValidCard$ Creature.YouCtrl | IsPresent$ Card.EffectSource+untapped";
    let body = read(script);
    let ability = &body.abilities[0];
    assert!(ability.contains("Effect::MayDo"));
    assert!(ability.contains("KeywordSet::CANT_ATTACK"));
    assert!(ability.contains("AttackWithoutTappingWhileSourceUntappedThisCombat"));
    assert!(!ability.contains("KeywordSet::VIGILANCE"));
    for (from, to) in [
        ("ValidPlayer$ You", "ValidPlayer$ Opponent"),
        ("OptionalDecider$ You", "OptionalDecider$ Opponent"),
        ("UntilEndOfCombat", "UntilEndOfTurn"),
        ("Card.EffectSource+untapped", "Card.EffectSource+tapped"),
        ("Creature.YouCtrl", "Creature"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
    assert!(refused(&format!("{script} | Unknown$ True")));
}

// Append to scriptgen/tests/effects.rs: synthetic structural fixtures, not corpus text.
#[test]
fn player_protection_life_lock_and_simultaneous_phase_chain_are_read_exactly() {
    let text = "Name:Boundary veil\nTypes:Instant\nManaCost:2 W\nA:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ Lock | SubAbility$ Shield\nSVar:Lock:Mode$ CantChangeLife | ValidPlayer$ You\nSVar:Shield:DB$ Pump | Defined$ You | Duration$ UntilYourNextTurn | KW$ Protection from everything | SubAbility$ Out\nSVar:Out:DB$ Phases | Defined$ Valid Permanent.YouCtrl | SubAbility$ Leave\nSVar:Leave:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile\n";
    let body = read(text);
    let emitted = body.abilities.join("\n");
    for expected in [
        "Modifier::LifeTotalCantChange",
        "Modifier::PlayerProtectionFromEverything",
        "Effect::PhaseOutFilter(&Filter::ControlledByYou)",
        "Effect::ExileSource",
    ] {
        assert!(emitted.contains(expected), "missing {expected}: {emitted}");
    }
    assert!(refused(
        &text.replace("ValidPlayer$ You", "ValidPlayer$ Opponent")
    ));
    assert!(refused(
        &text.replace("Valid Permanent.YouCtrl", "Valid Permanent")
    ));
    assert!(refused(&text.replace(
        "KW$ Protection from everything",
        "KW$ Protection from everything | UnreadRule$ True"
    )));
    assert!(refused(
        &text.replace("UntilYourNextTurn", "UntilEndOfTurn")
    ));
}

#[test]
fn cast_self_entry_protection_and_source_counter_amount_do_not_mean_announced_x() {
    let text = "Name:Boundary ring\nTypes:Artifact\nManaCost:4\nT:Mode$ ChangesZone | ValidCard$ Card.wasCastByYou+Self | Destination$ Battlefield | Execute$ Shield\nSVar:Shield:DB$ Pump | Defined$ You | Duration$ UntilYourNextTurn | KW$ Protection from everything\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ Loss | TriggerZones$ Battlefield\nSVar:Loss:DB$ LoseLife | LifeAmount$ X\nA:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ BURDEN | CounterNum$ 1 | SubAbility$ Draw\nSVar:Draw:DB$ Draw | Defined$ You | NumCards$ X\nSVar:X:Count$CardCounters.BURDEN\n";
    let body = read(text);
    let emitted = body.abilities.join("\n");
    assert!(emitted.contains("Condition::SourceWasCastByYou"));
    assert!(emitted.contains("Amount::CountersOnSource"));
    assert!(!emitted.contains("Amount::X"));
    assert!(refused(&text.replace(
        "Card.wasCastByYou+Self",
        "Card.wasCastByYou+Self+Creature"
    )));
    assert!(refused(&text.replace(
        "CardCounters.BURDEN",
        "CardCounters.UNKNOWN_UNASSIGNED"
    )));
}

// Append to scriptgen/tests/effects.rs; original synthetic grammar fixtures.
fn gond_read(text: &str) -> Option<CardBody> {
    let mut catalog = cats();
    catalog.land.push("Gate".into());
    catalog.normalize();
    transcode(&parse(text), &catalog, None)
}
#[test]
fn gond_entry_reader_accepts_only_own_gate_battlefield_untap() {
    let text = "Name:Entry boundary\nTypes:Land Gate\nR:Event$ Moved | ValidCard$ Gate.YouCtrl | Destination$ Battlefield | ReplacementResult$ Updated | ActiveZones$ Battlefield | ReplaceWith$ Child\nSVar:Child:DB$ Untap | ETB$ True | Defined$ ReplacedCard\n";
    let body = gond_read(text).unwrap();
    assert!(body.abilities[0].contains("ReplacementRule::EnterUntapped"));
    for (from, to) in [
        ("Gate.YouCtrl", "Gate.OppCtrl"),
        ("ActiveZones$ Battlefield", "ActiveZones$ Graveyard"),
        ("ETB$ True", "ETB$ False"),
        ("ReplacedCard", "Self"),
        ("ReplaceWith$ Child", "ReplaceWith$ Child | Unknown$ True"),
    ] {
        assert!(gond_read(&text.replace(from, to)).is_none(), "{to}");
    }
}
#[test]
fn gond_reflection_is_color_only_own_gate_and_keeps_unknown_guards() {
    let text = "Name:Reflection boundary\nTypes:Land Gate\nA:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Gate.YouCtrl | ReflectProperty$ Produce\n";
    let body = gond_read(text).unwrap();
    assert!(
        body.abilities[0]
            .contains("mana_from_lands(&Filter::HasSubtype(subtypes::land::GATE), true, false)")
    );
    for (from, to) in [
        ("ColorOrType$ Color", "ColorOrType$ Type"),
        ("Gate.YouCtrl", "Land.YouCtrl"),
        ("Gate.YouCtrl", "Gate.OppCtrl"),
        ("ReflectProperty$ Produce", "ReflectProperty$ Color"),
        ("Cost$ T", "Cost$ 1 T"),
        (
            "ReflectProperty$ Produce",
            "ReflectProperty$ Produce | Amount$ 2",
        ),
        (
            "ReflectProperty$ Produce",
            "ReflectProperty$ Produce | Unknown$ True",
        ),
    ] {
        assert!(gond_read(&text.replace(from, to)).is_none(), "{to}");
    }
}

// Append to scriptgen/tests/effects.rs. Synthetic grammar, no corpus script copied.
#[test]
fn graveyard_land_grant_has_exact_source_zone_and_refuses_unknowns() {
    let text = "Name:Buried boundary\nTypes:Land\nS:Mode$ Continuous | EffectZone$ Graveyard | Affected$ Land.YouCtrl | AddAbility$ Gift\nSVar:Gift:AB$ Mana | Cost$ T | Produced$ Combo G W\n";
    let body = read(text);
    assert!(body.abilities[0].contains("zone = ZoneRef::Graveyard"));
    assert!(body.abilities[0].contains("Modifier::GrantActivated"));
    for (from, to) in [
        ("EffectZone$ Graveyard", "EffectZone$ Exile"),
        ("EffectZone$ Graveyard", "EffectZone$ Hand"),
        ("EffectZone$ Graveyard", "EffectZone$ All"),
        ("Land.YouCtrl", "Land.OppCtrl"),
        ("AddAbility$ Gift", "AddAbility$ Gift | Unknown$ True"),
        ("Cost$ T", "Cost$ 1 T"),
        ("Combo G W", "Combo G W | Amount$ 2"),
        ("Combo G W", "Combo G W | Unknown$ True"),
    ] {
        assert!(refused(&text.replace(from, to)), "{to}");
    }
}

fn portals_body(text: &str) -> Option<CardBody> {
    let mut catalog = cats();
    catalog
        .land
        .extend(["Gate".into(), "Plains".into(), "Swamp".into()]);
    catalog.normalize();
    transcode(&parse(text), &catalog, None)
}
#[test]
fn remembered_white_mana_replacement_preserves_indefinite_incarnation_binding() {
    let script = "Name:X\nTypes:Creature Gnome\nManaCost:3 R\nPT:1/1\nA:AB$ Effect | Cost$ T | ValidTgts$ Plains | IsCurse$ True | RememberObjects$ Targeted | ForgetOnMoved$ Battlefield | Duration$ Permanent | ReplacementEffects$ Rewrite\nSVar:Rewrite:Event$ ProduceMana | ValidCard$ Card.IsRemembered | ReplaceWith$ Mana\nSVar:Mana:DB$ ReplaceMana | ReplaceColor$ C | ReplaceOnly$ W";
    // The shared tiny test catalog does not contain Gnome; this grammar does
    // not depend on the source's creature subtype.
    let script = script.replace("Types:Creature Gnome", "Types:Creature Goblin");
    let body = portals_body(&script).unwrap();
    let ability = body.abilities.join("");
    assert!(ability.contains("Modifier::ReplaceTappedMana"));
    assert!(ability.contains("Duration::Indefinitely"));
    for (from, to) in [
        ("ReplaceOnly$ W", "ReplaceOnly$ U"),
        ("Card.IsRemembered", "Card"),
        ("Duration$ Permanent", "Duration$ UntilEndOfTurn"),
        ("ForgetOnMoved$ Battlefield", "ForgetOnMoved$ Graveyard"),
    ] {
        assert!(portals_body(&script.replace(from, to)).is_none(), "{to}");
    }
}
#[test]
fn self_mana_life_tax_refuses_general_abilities_and_unproven_all_zone_semantics() {
    let script = "Name:X\nTypes:Land\nS:Mode$ RaiseCost | Type$ Ability | ValidSpell$ Activated.ManaAbility | ValidCard$ Card.Self | Cost$ PayLife<1>";
    assert!(portals_body(script).unwrap().abilities[0].contains("ManaAbilitiesCostLife(1)"));
    assert!(portals_body(&script.replace("Activated.ManaAbility", "Activated")).is_none());
    assert!(portals_body(&format!("{script} | EffectZone$ All")).is_none());
}

#[test]
fn thran_additive_chosen_type_requires_its_own_basic_land_choice() {
    let script = "Name:X\nTypes:Land Gate\nK:ETBReplacement:Other:Pick\nSVar:Pick:DB$ ChooseType | Type$ Basic Land\nS:Mode$ Continuous | Affected$ Card.Self | AddType$ ChosenType";
    let body = portals_body(script).unwrap();
    assert_eq!(body.enter_modifiers, ["EnterModifier::ChooseBasicLandType"]);
    assert_eq!(
        body.abilities,
        ["static_ability!(Filter::This, Modifier::AddChosenBasicLandType)"]
    );
    assert!(portals_body(&script.replace("Card.Self", "Land.YouCtrl")).is_none());
    assert!(portals_body(&script.replace("K:ETBReplacement:Other:Pick\n", "")).is_none());
    assert!(
        portals_body(&script.replace("AddType$ ChosenType", "AddType$ ChosenType | Unknown$ True"))
            .is_none()
    );
}
#[test]
fn wall_reader_keeps_blocked_source_and_wall_only_permission_distinct() {
    let damage = "Name:X\nTypes:Creature Goblin\nPT:0/1\nR:Event$ DamageDone | Prevent$ True | ValidSource$ Creature.blockedBySource | ValidTarget$ Creature.Self";
    assert!(read(damage).abilities[0].contains("PreventDamageFrom(&Filter::BlockedBySource)"));
    assert!(refused(&format!("{damage} | IsCombat$ True")));
    let permission = "Name:X\nTypes:Creature Goblin\nPT:0/1\nS:Mode$ CantTarget | ValidTarget$ Card.Self | SourceCanOnlyTarget$ Wall";
    assert!(read(permission).abilities[0].contains("CantBeTargetedByWallOnly"));
    assert!(refused(&permission.replace(
        "SourceCanOnlyTarget$ Wall",
        "SourceCanOnlyTarget$ Creature"
    )));
    assert!(refused(&format!("{permission} | ValidSource$ Spell")));
}
#[test]
fn guardian_reader_does_not_drop_hidden_control_or_new_attachment_predicate() {
    let rule = "Name:X\nTypes:Creature Goblin\nPT:2/4\nS:Mode$ Continuous | Affected$ Artifact.nonCreature+YouCtrl | AddKeyword$ Indestructible | AddHiddenKeyword$ Other players can't gain control of CARDNAME. | IsPresent$ Card.Self+untapped";
    let body = read(rule);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities
            .iter()
            .any(|a| a.contains("OtherPlayersCantGainControl"))
    );
    assert!(
        body.abilities
            .iter()
            .all(|a| a.contains("Condition::SourceMatches(&Filter::Untapped)"))
    );
    assert!(refused(
        &rule.replace("Card.Self+untapped", "Card.Self+tapped")
    ));
    let attach = "Name:X\nTypes:Creature Goblin\nPT:2/4\nS:Mode$ CantAttach | ValidCardToTarget$ Aura.!Attached | Target$ Artifact.nonCreature+YouCtrl | IsPresent$ Card.Self+untapped";
    assert!(read(attach).abilities[0].contains("Modifier::CantBecomeEnchanted"));
    for (from, to) in [
        ("Aura.!Attached", "Aura"),
        ("Artifact.nonCreature+YouCtrl", "Artifact.YouCtrl"),
        ("Card.Self+untapped", "Card.Self"),
    ] {
        assert!(refused(&attach.replace(from, to)), "{to}");
    }
}

#[test]
fn transform48_source_mana_trigger_preserves_event_mana_value_and_refuses_other_sources() {
    let script = "Name:X\nTypes:Land\nT:Mode$ SpellCast | ValidCard$ Permanent | ValidSA$ Spell.ManaFromCard.StrictlySelf | ValidActivatingPlayer$ You | Execute$ D\nSVar:D:DB$ Discover | Num$ TriggeredSpellAbility$CardManaCostLKI";
    let body = transcode(&parse(script), &cats(), None).unwrap();
    assert!(body.abilities[0].contains("SpellCastUsingSourceMana"));
    assert!(body.abilities[0].contains("Amount::EventManaValue"));
    for (from, to) in [
        ("StrictlySelf", "Other"),
        (
            "ValidActivatingPlayer$ You",
            "ValidActivatingPlayer$ Opponent",
        ),
        ("CardManaCostLKI", "CardManaCost"),
        (
            "Num$ TriggeredSpellAbility$CardManaCostLKI",
            "Num$ TriggeredSpellAbility$CardManaCostLKI | Unknown$ True",
        ),
    ] {
        assert!(
            transcode(&parse(&script.replace(from, to)), &cats(), None).is_none(),
            "{to}"
        );
    }
}
#[test]
fn transform48_first_main_requires_exact_optional_union_five_cost() {
    let script = "Name:X\nTypes:Artifact\nT:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ F\nSVar:F:AB$ SetState | Cost$ tapXType<5/Artifact;Creature/artifact or creature> | Defined$ Self | Mode$ Transform";
    let body = transcode(&parse(script), &cats(), None).unwrap();
    assert!(body.abilities[0].contains("StepKind::FirstMain"));
    assert!(body.abilities[0].contains("CostPart::TapN"));
    for (from, to) in [
        ("tapXType<5/", "tapXType<4/"),
        ("Phase$ Main1", "Phase$ Main2"),
        ("Defined$ Self", "Defined$ Targeted"),
        ("Mode$ Transform", "Mode$ Transform | Unknown$ True"),
    ] {
        assert!(
            transcode(&parse(&script.replace(from, to)), &cats(), None).is_none(),
            "{to}"
        );
    }
}
#[test]
fn transform48_optional_discard_draw_chain_is_read_as_a_complete_sentence() {
    let script = "Name:X\nTypes:Artifact\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ D\nSVar:D:DB$ Discard | AnyNumber$ True | Optional$ True | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ N\nSVar:N:DB$ Draw | Defined$ You | NumCards$ Remembered$Amount/Plus.1 | SubAbility$ C\nSVar:C:DB$ Cleanup | ClearRemembered$ True";
    let body = transcode(&parse(script), &cats(), None).unwrap();
    assert!(body.abilities[0].contains("DiscardAnyThenDraw, Effect::draw(1)"));
    for (from, to) in [
        ("Optional$ True", "Optional$ False"),
        ("Plus.1", "Plus.2"),
        ("ClearRemembered$ True", "ClearRemembered$ False"),
        (
            "RememberDiscarded$ True",
            "RememberDiscarded$ True | Unknown$ True",
        ),
    ] {
        assert!(
            transcode(&parse(&script.replace(from, to)), &cats(), None).is_none(),
            "{to}"
        );
    }
}

#[test]
fn transform48_clue_sacrifice_condition_keeps_subtype_count_player_and_timing() {
    // Synthetic names around the verified property grammar; no corpus body copied.
    let script = "Name:Boundary laboratory\nTypes:Land\n\
        T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Flip | CheckSVar$ Clues | SVarCompare$ GE3\n\
        SVar:Clues:PlayerCountPropertyYou$SacrificedThisTurn Clue\n\
        SVar:Flip:DB$ SetState | Defined$ Self | Mode$ Transform";
    let body = read(script);
    assert_eq!(body.abilities.len(), 1);
    assert!(body.abilities[0].contains("StepKind::End, whose: PlayerRel::You"));
    assert!(body.abilities[0].contains(
        "Condition::SacrificedSubtypeThisTurn { subtype: subtypes::artifact::CLUE, count: 3 }"
    ));
    for (from, to) in [
        ("SacrificedThisTurn Clue", "SacrificedThisTurn Food"),
        ("PropertyYou$", "PropertyOpponent$"),
        ("SVarCompare$ GE3", "SVarCompare$ GE2"),
        ("SVarCompare$ GE3", "SVarCompare$ EQ3"),
        ("ValidPlayer$ You", "ValidPlayer$ Opponent"),
        ("Phase$ End of Turn", "Phase$ Upkeep"),
        ("TriggerZones$ Battlefield", "TriggerZones$ Graveyard"),
        ("Defined$ Self", "Defined$ Targeted"),
        ("Mode$ Transform", "Mode$ Transform | Unknown$ True"),
        ("CheckSVar$ Clues", "CheckSVar$ Clues | Unknown$ True"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
}

#[test]
fn bore_counter_generic_reader_preserves_the_verified_exact_counter_word() {
    use super::super::constants::counter_kind;
    assert_eq!(counter_kind("BORE").as_deref(), Some("CounterKind::Bore"));
    for unknown in ["Bore", "bore", "BORES", "BORE.EXTRA"] {
        assert!(counter_kind(unknown).is_none(), "{unknown}");
    }
    // Generic activated-counter grammar rather than the transform reader's
    // specialized PutCounter/Branch chain.
    let script = "Name:Boundary drill\nTypes:Artifact\n\
        A:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ BORE | CounterNum$ 1";
    let body = read(script);
    assert!(body.abilities[0].contains("CounterKind::Bore"));
    for wrong in ["Bore", "bore", "BORES"] {
        assert!(refused(&script.replace("BORE", wrong)), "{wrong}");
    }
}

#[test]
fn extravagant_spirit_food_chain_flailing_and_haunted_crossroads_shapes_read() {
    let spirit = "Name:Extravagant Spirit\nManaCost:3 U\nTypes:Creature Spirit\nPT:4/4\nK:Flying\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigUpkeep\n\
        SVar:TrigUpkeep:DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ Y\n\
        SVar:Y:Count$ValidHand Card.YouOwn";
    let body = read(spirit);
    assert!(
        body.abilities[0].contains("Amount::HandSize(PlayerRel::You)"),
        "{}",
        body.abilities[0]
    );

    let flailing = "Name:Flailing Soldier\nManaCost:R\nTypes:Creature Human Soldier\nPT:2/2\n\
        A:AB$ Pump | Cost$ 1 | Defined$ Self | NumAtt$ +1 | NumDef$ +1 | Activator$ Player";
    let body = read(flailing);
    assert!(
        body.abilities[0].contains("zone = ActivationZone::BattlefieldAnyPlayer"),
        "{}",
        body.abilities[0]
    );

    let food_chain = "Name:Food Chain\nManaCost:2 G\nTypes:Enchantment\n\
        A:AB$ Mana | Cost$ Exile<1/Creature> | Produced$ Any | Amount$ X | RestrictValid$ Spell.Creature\n\
        SVar:X:Exiled$CardManaCost/Plus.1";
    let body = read(food_chain);
    assert!(
        body.abilities[0].contains("ExilePermanent(&Filter::YOUR_CREATURE)")
            && body.abilities[0]
                .contains("Amount::Plus { base: &Amount::SacrificedManaValue, offset: 1 }"),
        "{}",
        body.abilities[0]
    );

    let crossroads = "Name:Haunted Crossroads\nManaCost:2 B\nTypes:Enchantment\n\
        A:AB$ ChangeZone | Cost$ B | ValidTgts$ Creature.YouCtrl | Origin$ Graveyard | Destination$ Library";
    let body = read(crossroads);
    assert!(
        body.abilities[0].contains("Effect::GraveyardToTop"),
        "{}",
        body.abilities[0]
    );

    let furious = "Name:Furious Assault\nManaCost:2 R\nTypes:Enchantment\n\
        T:Mode$ SpellCast | ValidCard$ Creature | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDealDamage\n\
        SVar:TrigDealDamage:DB$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 1";
    let body = read(furious);
    assert!(
        body.abilities[0].contains("TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)"),
        "{}",
        body.abilities[0]
    );

    let high_seas = "Name:High Seas\nManaCost:2 U\nTypes:Enchantment\n\
        S:Mode$ RaiseCost | ValidCard$ Creature.Red,Creature.Green | Type$ Spell | Amount$ 1";
    let body = read(high_seas);
    assert!(
        body.abilities[0].contains("Modifier::SpellsCostMore(1)"),
        "{}",
        body.abilities[0]
    );

    assert!(refused(
        "Name:Hammer Mage\nManaCost:1 R\nTypes:Creature Human Spellshaper\nPT:1/1\n\
         A:AB$ DestroyAll | Cost$ X R T Discard<1/Card> | ValidCards$ Artifact.cmcLEX\n\
         SVar:X:Count$xPaid"
    ));
}

#[test]
fn jeweled_torque_karns_touch_cant_block_liability_muzzle_and_pangosaur_shapes_read() {
    let torque = "Name:Jeweled Torque\nManaCost:2\nTypes:Artifact\n\
        K:ETBReplacement:Other:ChooseColor\n\
        SVar:ChooseColor:DB$ ChooseColor | Defined$ You\n\
        T:Mode$ SpellCast | ValidCard$ Card.ChosenColor | TriggerZones$ Battlefield | Execute$ TrigGainLife\n\
        SVar:TrigGainLife:AB$ GainLife | Cost$ 2 | Defined$ You | LifeAmount$ 2";
    let body = read(torque);
    assert!(
        body.abilities[0].contains("PlayerMayPayThen")
            && body.abilities[0].contains("Amount::Fixed(2)"),
        "{}",
        body.abilities[0]
    );

    let touch = "Name:Karn's Touch\nManaCost:U U\nTypes:Instant\n\
        A:SP$ Animate | ValidTgts$ Artifact.nonCreature | Power$ X | Toughness$ X | Types$ Artifact,Creature\n\
        SVar:X:Targeted$CardManaCost";
    let body = read(touch);
    assert!(
        body.abilities[0].contains("Effect::SetPTFilter")
            && body.abilities[0].contains("Amount::TargetCmc"),
        "{}",
        body.abilities[0]
    );

    let veto = "Name:Magistrate's Veto\nManaCost:2 R\nTypes:Enchantment\n\
        S:Mode$ CantBlock | ValidCard$ Creature.White,Creature.Blue";
    let body = read(veto);
    assert!(
        body.abilities[0].contains("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"),
        "{}",
        body.abilities[0]
    );

    let liability = "Name:Liability\nManaCost:1 B B\nTypes:Enchantment\n\
        T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Permanent.nonToken | TriggerZones$ Battlefield | Execute$ TrigLoseLife\n\
        SVar:TrigLoseLife:DB$ LoseLife | Defined$ TriggeredCardOwner | LifeAmount$ 1";
    let body = read(liability);
    assert!(
        body.abilities[0].contains("PlayerRel::OwnerOfEvent"),
        "{}",
        body.abilities[0]
    );

    let muzzle = "Name:Muzzle\nManaCost:1 W\nTypes:Enchantment Aura\n\
        K:Enchant:Creature\n\
        R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidSource$ Creature.EnchantedBy";
    let body = read(muzzle);
    assert!(
        body.abilities[1].contains("Modifier::PreventAllDamageFromIt"),
        "{}",
        body.abilities[1]
    );

    let pangosaur = "Name:Pangosaur\nManaCost:2 G G\nTypes:Creature Dinosaur\nPT:6/6\n\
        T:Mode$ LandPlayed | ValidCard$ Land | TriggerZones$ Battlefield | Execute$ TrigBounce\n\
        SVar:TrigBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand";
    let body = read(pangosaur);
    assert!(
        body.abilities[0].contains("Trigger::PlaysLand(PlayerRel::EachPlayer)"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn inviolability_natural_affinity_noble_purpose_pious_warrior_revered_elder_and_rishadan_pirate_shapes_read()
 {
    let inviolability = "Name:Inviolability\nManaCost:1 W\nTypes:Enchantment Aura\n\
        K:Enchant:Creature\n\
        R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidTarget$ Creature.EnchantedBy";
    let body = read(inviolability);
    assert!(
        body.abilities[1].contains("Modifier::PreventDamageFrom(&Filter::Any)"),
        "{}",
        body.abilities[1]
    );

    let natural_affinity = "Name:Natural Affinity\nManaCost:2 G\nTypes:Instant\n\
        A:SP$ AnimateAll | ValidCards$ Land | Types$ Creature | Power$ 2 | Toughness$ 2";
    let body = read(natural_affinity);
    assert!(
        body.abilities[0].contains("Modifier::AddType(TypeSet::CREATURE)")
            && body.abilities[0].contains("Modifier::SetPT(2, 2)"),
        "{}",
        body.abilities[0]
    );

    let noble_purpose = "Name:Noble Purpose\nManaCost:3 W W\nTypes:Enchantment\n\
        T:Mode$ DamageDealtOnce | CombatDamage$ True | ValidSource$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigGain\n\
        SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:TriggerCount$DamageAmount";
    let body = read(noble_purpose);
    assert!(
        body.abilities[0].contains("Trigger::DealsCombatDamage(&Filter::YOUR_CREATURE)")
            && body.abilities[0].contains("Effect::GainLife { amount: Amount::EventAmount }"),
        "{}",
        body.abilities[0]
    );

    let pious_warrior = "Name:Pious Warrior\nManaCost:3 W\nTypes:Creature Human Rebel Warrior\nPT:2/3\n\
        T:Mode$ DamageDoneOnce | CombatDamage$ True | ValidTarget$ Card.Self | Execute$ TrigGain\n\
        SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:TriggerCount$DamageAmount";
    let body = read(pious_warrior);
    assert!(
        body.abilities[0].contains("Trigger::DealtCombatDamage(&Filter::This)")
            && body.abilities[0].contains("Effect::GainLife { amount: Amount::EventAmount }"),
        "{}",
        body.abilities[0]
    );

    let revered_elder = "Name:Revered Elder\nManaCost:2 W\nTypes:Creature Human Cleric\nPT:1/2\n\
        A:AB$ PreventDamage | Cost$ 1 | Defined$ Self | Amount$ 1";
    let body = read(revered_elder);
    assert!(
        body.abilities[0].contains("TargetSpec::ThisObject")
            && body.abilities[0].contains("Amount::Fixed(1)"),
        "{}",
        body.abilities[0]
    );

    let cutpurse = "Name:Rishadan Cutpurse\nManaCost:2 U\nTypes:Creature Human Pirate\nPT:1/1\n\
        T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ RepeatEach | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ DBSac\n\
        SVar:DBSac:DB$ Sacrifice | Defined$ Remembered | SacValid$ Permanent | UnlessCost$ 1 | UnlessPayer$ Remembered";
    let body = read(cutpurse);
    assert!(
        body.abilities[0].contains("PlayerRel::EachOpponent")
            && body.abilities[0].contains("PlayerRel::EventPlayer"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn pawnshop_heir_outrigger_scandalmonger_sever_soul_silent_assassin_and_fugitive_shapes_read() {
    let pawnshop = "Name:Rishadan Pawnshop\nManaCost:2\nTypes:Artifact\n\
        A:AB$ ChangeZone | Cost$ 2 T | Origin$ Battlefield | Destination$ Library | Shuffle$ True | ValidTgts$ Permanent.nonToken+YouCtrl";
    let body = read(pawnshop);
    assert!(
        body.abilities[0].contains("Effect::ShuffleTargetsIntoOwnersLibraries"),
        "{}",
        body.abilities[0]
    );

    let heir = "Name:Saprazzan Heir\nManaCost:1 U\nTypes:Creature Merfolk\nPT:1/1\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigDraw\n\
        SVar:TrigDraw:DB$ Draw | NumCards$ 3 | OptionalDecider$ You";
    let body = read(heir);
    assert!(
        body.abilities[0].contains("Effect::MayDo { effects: &[Effect::draw(3)] }"),
        "{}",
        body.abilities[0]
    );

    let outrigger = "Name:Saprazzan Outrigger\nManaCost:3 U\nTypes:Creature Merfolk\nPT:5/5\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDelayed\n\
        T:Mode$ Blocks | ValidCard$ Card.Self | Secondary$ True | Execute$ TrigDelayed\n\
        SVar:TrigDelayed:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | Execute$ TrigTop\n\
        SVar:TrigTop:DB$ ChangeZone | Origin$ Battlefield | Destination$ Library | LibraryPosition$ 0";
    let body = read(outrigger);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities[0].contains("Trigger::Attacks(&Filter::This)")
            && body.abilities[0].contains("Effect::PutSourceOnTopOfLibrary")
            && body.abilities[1].contains("Trigger::Blocks(&Filter::This)"),
        "{:?}",
        body.abilities
    );

    let scandalmonger = "Name:Scandalmonger\nManaCost:3 B\nTypes:Creature Boar Monger\nPT:3/3\n\
        A:AB$ Discard | Cost$ 2 | ValidTgts$ Player | NumCards$ 1 | Mode$ TgtChoose | IsCurse$ True | SorcerySpeed$ True | Activator$ Player";
    let body = read(scandalmonger);
    assert!(
        body.abilities[0].contains("timing = ActivationTiming::SorcerySpeed")
            && body.abilities[0].contains("zone = ActivationZone::BattlefieldAnyPlayer"),
        "{}",
        body.abilities[0]
    );

    let sever_soul = "Name:Sever Soul\nManaCost:3 B B\nTypes:Sorcery\n\
        A:SP$ Destroy | ValidTgts$ Creature.nonBlack | NoRegen$ True | SubAbility$ DBGain\n\
        SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:Targeted$CardToughness";
    let body = read(sever_soul);
    assert!(
        body.abilities[0].contains("Amount::TargetToughness"),
        "{}",
        body.abilities[0]
    );

    let silent_assassin = "Name:Silent Assassin\nManaCost:B B\nTypes:Creature Human Mercenary Assassin\nPT:2/1\n\
        A:AB$ Effect | Cost$ 3 B | ValidTgts$ Creature.blocking | RememberObjects$ Targeted | Triggers$ DelTrig\n\
        SVar:DelTrig:Mode$ Phase | Phase$ EndCombat | Execute$ TrigDestroy\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ Remembered";
    let body = read(silent_assassin);
    assert!(
        body.abilities[0].contains(
            "Effect::AtEndOfCombat { about: TargetSpec::Object(&TARGET1), effects: &[Effect::destroy(TargetSpec::EventObject)] }"
        ),
        "{}",
        body.abilities[0]
    );

    let fugitive = "Name:Skulking Fugitive\nManaCost:2 B\nTypes:Creature Horror Mercenary\nPT:3/4\n\
        T:Mode$ BecomesTarget | ValidTarget$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice";
    let body = read(fugitive);
    assert!(
        body.abilities[0].contains("Trigger::BecomesTarget")
            && body.abilities[0].contains("Effect::SacrificeSelf"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn overtaker_squee_statecraft_and_territorial_dispute_shapes_read() {
    let overtaker = "Name:Overtaker\nManaCost:1 U\nTypes:Creature Merfolk Spellshaper\nPT:1/1\n\
        A:AB$ Untap | Cost$ 3 U T Discard<1/Card> | ValidTgts$ Creature | SubAbility$ DBChange\n\
        SVar:DBChange:DB$ GainControl | Defined$ Targeted | AddKWs$ Haste | LoseControl$ EOT";
    let body = read(overtaker);
    assert!(
        body.abilities[0].contains("Effect::UntapTarget")
            && body.abilities[0].contains(
                "Effect::continuous(&Filter::This, Modifier::GainControl, Duration::UntilEndOfTurn)"
            )
            && body.abilities[0].contains("keywords: KeywordSet::HASTE"),
        "{}",
        body.abilities[0]
    );

    let squee = "Name:Squee, Goblin Nabob\nManaCost:2 R\nTypes:Legendary Creature Goblin\nPT:1/1\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Graveyard | OptionalDecider$ You | Execute$ TrigChange\n\
        SVar:TrigChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ Self";
    let body = read(squee);
    assert!(
        body.abilities[0].contains("zone = TriggerZone::Graveyard")
            && body.abilities[0].contains("Effect::bounce(TargetSpec::ThisObject)"),
        "{}",
        body.abilities[0]
    );

    let statecraft = "Name:Statecraft\nManaCost:3 U\nTypes:Enchantment\n\
        R:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidTarget$ Creature.YouCtrl | ActiveZones$ Battlefield\n\
        R:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidSource$ Creature.YouCtrl | ActiveZones$ Battlefield | Secondary$ True";
    let body = read(statecraft);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities[0].contains("Modifier::PreventDamageToIt")
            && body.abilities[1].contains("Modifier::PreventDamageFromIt"),
        "{:?}",
        body.abilities
    );

    let dispute = "Name:Territorial Dispute\nManaCost:4 R R\nTypes:Enchantment\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigUpkeep\n\
        SVar:TrigUpkeep:DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ Sac<1/Land>\n\
        S:Mode$ CantPlayLand | Description$ Players can't play lands.";
    let body = read(dispute);
    assert!(
        body.abilities
            .iter()
            .any(|a| a.contains("Modifier::PlayersCantPlay(&Filter::LAND)")),
        "{:?}",
        body.abilities
    );

    let warpath = "Name:Warpath\nManaCost:3 R\nTypes:Instant\n\
        A:SP$ DamageAll | NumDmg$ 3 | ValidCards$ Creature.blocked,Creature.blocking";
    let body = read(warpath);
    assert!(
        body.statics.contains("Filter::Blocked") && body.statics.contains("Filter::Blocking"),
        "{}",
        body.statics
    );
}

#[test]
fn prophecy_batch_3_patterns_generate() {
    let fiend = "Name:Plague Fiend\nManaCost:1 B\nTypes:Creature Insect\nPT:1/1\n\
        T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Creature | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigDestroy\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ TriggeredTargetLKICopy | UnlessCost$ 2 | UnlessPayer$ TriggeredTargetController";
    let body = read(fiend);
    assert!(
        body.abilities[0].contains("Trigger::DealsCombatDamageToCreature(&Filter::This)")
            && body.abilities[0].contains("PlayerRel::ControllerOfEvent")
            && body.abilities[0].contains("Effect::destroy(TargetSpec::EventObject)"),
        "{}",
        body.abilities[0]
    );

    let informer = "Name:Rebel Informer\nManaCost:2 B\nTypes:Creature Human Mercenary Rebel\nPT:1/2\n\
        S:Mode$ CantTarget | ValidTarget$ Card.Self | ValidSource$ Card.White\n\
        A:AB$ ChangeZone | Cost$ 3 | ValidTgts$ Goblin.!token | IsCurse$ True | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1";
    let body = read(informer);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities[0].contains("Modifier::CantBeTargetedBy(&SOURCE1)")
            && body.abilities[1].contains("Effect::PutTargetOnBottomOfLibrary"),
        "{:?}",
        body.abilities
    );

    let rethink = "Name:Rethink\nManaCost:2 U\nTypes:Instant\n\
        A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ X | UnlessPayer$ TargetedController\n\
        SVar:X:Targeted$CardManaCost";
    let body = read(rethink);
    assert!(
        body.abilities[0].contains("Amount::TargetCmc")
            && body.abilities[0].contains("Effect::CounterTargetSpell"),
        "{}",
        body.abilities[0]
    );

    let syphon = "Name:Rhystic Syphon\nManaCost:3 B B\nTypes:Sorcery\n\
        A:SP$ LoseLife | LifeAmount$ 5 | ValidTgts$ Player | SubAbility$ DBGainLife | UnlessCost$ 3 | UnlessPayer$ Targeted | UnlessResolveSubs$ WhenNotPaid\n\
        SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 5";
    let body = read(syphon);
    assert!(
        body.abilities[0].contains("Effect::PlayerMayPayOr")
            && body.abilities[0].contains("Effect::Sequence(&[Effect::LoseLife")
            && body.abilities[0].contains("Effect::gain_life(5)"),
        "{}",
        body.abilities[0]
    );

    let snake = "Name:Ribbon Snake\nManaCost:1 U U\nTypes:Creature Snake\nPT:2/3\nK:Flying\n\
        A:AB$ Debuff | Cost$ 2 | Defined$ Self | Keywords$ Flying | Activator$ Player";
    let body = read(snake);
    assert!(
        body.abilities[0].contains("Modifier::RemoveKeyword(KeywordSet::FLYING)")
            && body.abilities[0].contains("zone = ActivationZone::BattlefieldAnyPlayer"),
        "{}",
        body.abilities[0]
    );

    let serpent = "Name:Shrouded Serpent\nManaCost:4 U U U\nTypes:Creature Serpent\nPT:4/4\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigUnblockable\n\
        SVar:TrigUnblockable:DB$ Effect | UnlessCost$ 4 | UnlessPayer$ TriggeredDefendingPlayer | RememberObjects$ Self | ExileOnMoved$ Battlefield | StaticAbilities$ Unblockable\n\
        SVar:Unblockable:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered";
    let body = read(serpent);
    assert!(
        body.abilities[0].contains("PlayerRel::DefendingPlayer")
            && body.abilities[0].contains("KeywordSet::UNBLOCKABLE"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn sacrifice_effect_accepts_change_num_one() {
    let beast = "Name:Thresher Beast\nManaCost:3 G G\nTypes:Creature Beast\nPT:4/4\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice | SacValid$ Land | Defined$ TriggeredDefendingPlayer | ChangeNum$ 1";
    let body = read(beast);
    assert_eq!(
        body.abilities,
        [
            "triggered!(Trigger::BecomesBlocked, &[Effect::SacrificeFilter { \
             who: PlayerRel::DefendingPlayer, filter: &Filter::LAND }])"
        ]
    );

    let two = parse(
        "Name:X\nManaCost:3 G G\nTypes:Creature Beast\nPT:4/4\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice | SacValid$ Land | Defined$ TriggeredDefendingPlayer | ChangeNum$ 2",
    );
    assert!(refusal_reason(&two, &cats(), None).is_some());
}

#[test]
fn invasion_batch_one_reader_patterns() {
    let leech = "Name:Alabaster Leech\nManaCost:W\nTypes:Creature Leech\nPT:1/3\n\
        S:Mode$ RaiseCost | ValidCard$ Card.White | Activator$ You | Type$ Spell | Cost$ W";
    let body = read(leech);
    assert_eq!(
        body.abilities,
        [
            "static_ability!(Filter::And(&[Filter::HasColor(ColorSet::from_slice(&[Color::White])), \
             Filter::ControlledByYou]), Modifier::SpellsCostMoreMana(mana!(\"{W}\")))"
        ]
    );

    let kavu = "Name:Ancient Kavu\nManaCost:3 R\nTypes:Creature Kavu\nPT:3/3\n\
        A:AB$ Animate | Cost$ 2 | Defined$ Self | Colors$ Colorless | OverwriteColors$ True";
    let body = read(kavu);
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{2}\"), &[Effect::continuous(&Filter::This, \
             Modifier::SetColor(ColorSet::EMPTY), Duration::UntilEndOfTurn)])"
        ]
    );

    let guardian = "Name:Armored Guardian\nManaCost:3 W U\nTypes:Creature Cat Soldier\nPT:2/5\n\
        A:AB$ Protection | Cost$ 1 W W | ValidTgts$ Creature.YouCtrl | Gains$ Choice | Choices$ AnyColor";
    let body = read(guardian);
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{1}{W}{W}\"), &[Effect::ProtectionFromChosenColor { \
             duration: Duration::UntilEndOfTurn }], target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)))"
        ]
    );

    let bind = "Name:Bind\nManaCost:1 G\nTypes:Instant\n\
        A:SP$ Counter | ValidTgts$ Card | TargetType$ Activated | SubAbility$ DBDraw\n\
        SVar:DBDraw:DB$ Draw";
    let body = read(bind);
    assert_eq!(
        body.abilities,
        ["spell!(&[Effect::CounterTargetAbility, Effect::draw(1)], \
             targets = Some(TargetReq::one(TargetSpec::AbilityOnStack(&TARGET1))))"]
    );
    assert!(
        body.statics
            .contains("static TARGET1: Filter = Filter::ActivatedAbilityFrom(&Filter::Any);")
    );

    let mongoose = "Name:Blurred Mongoose\nManaCost:1 G\nTypes:Creature Mongoose\nPT:2/1\n\
        R:Event$ Counter | ValidCard$ Card.Self | ValidSA$ Spell | Layer$ CantHappen\n\
        K:Shroud";
    let body = read(mongoose);
    assert_eq!(
        body.keywords,
        ["KeywordSet::SHROUD", "KeywordSet::UNCOUNTERABLE"]
    );
}

#[test]
fn invasion_batch_two_reader_patterns() {
    let shade = "Name:Cinder Shade\nManaCost:1 B R\nTypes:Creature Shade\nPT:1/1\n\
        A:AB$ DealDamage | Cost$ R Sac<1/CARDNAME> | ValidTgts$ Creature | NumDmg$ X\n\
        SVar:X:Sacrificed$CardPower";
    let body = read(shade);
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{R}\", SacrificeSelf), &[Effect::DealDamage { amount: Amount::SourcePower, \
             target: TargetSpec::Object(&Filter::CREATURE) }], target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let tears = "Name:Defiling Tears\nManaCost:2 B\nTypes:Instant\n\
        A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ -1 | SubAbility$ DBAnimate\n\
        SVar:DBAnimate:DB$ Animate | Colors$ Black | OverwriteColors$ True | Abilities$ ABRegenerate | Defined$ Targeted\n\
        SVar:ABRegenerate:AB$ Regenerate | Cost$ B";
    let body = read(tears);
    assert_eq!(
        body.abilities,
        [
            "spell!(&[Effect::PumpTarget { power: Amount::Fixed(1), toughness: Amount::NegXFixed(1), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }, \
             Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(&[Color::Black])), Duration::UntilEndOfTurn), \
             Effect::continuous(&Filter::This, Modifier::GrantActivated { cost: cost!(\"{B}\"), effects: &[Effect::regenerate(TargetSpec::ThisObject)], mana_ability: false }, Duration::UntilEndOfTurn)], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let sisay = "Name:Captain Sisay\nManaCost:2 G W\nTypes:Legendary Creature Human Soldier\nPT:2/2\n\
        A:AB$ ChangeZone | Cost$ T | Origin$ Library | Destination$ Hand | ChangeType$ Legendary | ChangeNum$ 1";
    assert_eq!(
        read(sisay).abilities,
        [
            "activated!(Cost::TAP, &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::HAND], optional: false }])"
        ]
    );

    let cremate = "Name:Cremate\nManaCost:B\nTypes:Instant\n\
        A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | SubAbility$ DBDraw\n\
        SVar:DBDraw:DB$ Draw";
    assert_eq!(
        read(cremate).abilities,
        [
            "spell!(&[Effect::exile(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer)), Effect::draw(1)], \
             targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer))))"
        ]
    );

    let acolyte = "Name:Crimson Acolyte\nManaCost:1 W\nTypes:Creature Human Cleric\nPT:1/1\n\
        K:Protection from red\n\
        A:AB$ Protection | Cost$ W | ValidTgts$ Creature | Gains$ red";
    assert_eq!(
        read(acolyte).abilities,
        [
            "static_ability!(Filter::This, Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red]))))",
            "activated!(cost!(\"{W}\"), &[Effect::continuous(&Filter::This, Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red]))), Duration::UntilEndOfTurn)], \
             target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let knight = "Name:Crusading Knight\nManaCost:2 W W\nTypes:Creature Human Knight\nPT:2/2\n\
        K:Protection from black\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
        SVar:X:Count$Valid Forest.OppCtrl";
    assert_eq!(
        read(knight).abilities,
        [
            "static_ability!(Filter::This, Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))))",
            "static_ability!(Filter::This, Modifier::ModifyPTByCount(PtCount::OnBattlefield(&COUNTED1)))"
        ]
    );

    let wake = "Name:Distorting Wake\nManaCost:X U U U\nTypes:Sorcery\n\
        A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | TargetMin$ X | TargetMax$ X | ValidTgts$ Permanent.nonLand\n\
        SVar:X:Count$xPaid";
    assert_eq!(
        read(wake).abilities,
        [
            "spell!(&[Effect::bounce(TargetSpec::Object(&Filter::NONLAND))], \
             targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::NONLAND))))"
        ]
    );
}

#[test]
fn invasion_batch_three_reader_patterns() {
    let ranger = "Name:Firebrand Ranger\nManaCost:1 R\nTypes:Creature Human Soldier\nPT:2/1\n\
        A:AB$ ChangeZone | Cost$ G T | Origin$ Hand | Destination$ Battlefield | ChangeType$ Land.Basic | Optional$ You";
    assert_eq!(
        read(ranger).abilities,
        [
            "activated!(cost!(\"{G}\", TapSelf), &[Effect::PutFromHandOntoBattlefield { tapped: false, filter: &PUT_FROM_HAND1, mana_value: None, optional: true }])"
        ]
    );

    let tilling = "Name:Frenzied Tilling\nManaCost:3 R G\nTypes:Sorcery\n\
        A:SP$ Destroy | ValidTgts$ Land | SubAbility$ DBSearch\n\
        SVar:DBSearch:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | Tapped$ True";
    assert_eq!(
        read(tilling).abilities,
        [
            "spell!(&[Effect::destroy(TargetSpec::Object(&Filter::LAND)), Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD_TAPPED], optional: false }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))))"
        ]
    );

    let kavu = "Name:Hunting Kavu\nManaCost:1 R G\nTypes:Creature\nPT:2/3\n\
        A:AB$ ChangeZone | Cost$ 1 R G T | ValidTgts$ Creature.withoutFlying+attackingYou | ThisDefinedAndTgts$ Self | Origin$ Battlefield | Destination$ Exile";
    assert_eq!(
        read(kavu).abilities,
        [
            "activated!(cost!(\"{1}{R}{G}\", TapSelf), &[Effect::ExileSource, Effect::exile(TargetSpec::Object(&TARGET1))], \
             target = Some(TargetSpec::Object(&TARGET1)))"
        ]
    );

    let chameleon = "Name:Kavu Chameleon\nManaCost:3 G G\nTypes:Creature\nPT:4/4\n\
        A:AB$ ChooseColor | Cost$ G | Defined$ You | SubAbility$ Animate\n\
        SVar:Animate:DB$ Animate | Defined$ Self | Colors$ ChosenColor | OverwriteColors$ True";
    assert_eq!(
        read(chameleon).abilities,
        [
            "activated!(cost!(\"{G}\"), &[Effect::ChooseColors { subject: ColorSubject::Source, max: 1, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn invasion_batch_four_reader_patterns() {
    let delver = "Name:Phyrexian Delver\nManaCost:3 B B\nTypes:Creature Phyrexian Zombie\nPT:3/2\n\
        T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
        SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl | GainControl$ True | ChangeNum$ 1 | SubAbility$ DBLoseLifeYou\n\
        SVar:DBLoseLifeYou:DB$ LoseLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:Targeted$CardManaCost";
    assert_eq!(
        read(delver).abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::GraveyardToBattlefield { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You), owner_control: false, counters: None }, Effect::LoseLife { amount: Amount::TargetCmc, target: PlayerRel::You }], targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))))"
        ]
    );
}

#[test]
fn invasion_batch_five_reader_patterns() {
    let rewards = "Name:Rewards of Diversity\nManaCost:2 W\nTypes:Enchantment\n\
        T:Mode$ SpellCast | ValidCard$ Card.MultiColor | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigGainLife\n\
        SVar:TrigGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 4";
    assert_eq!(
        read(rewards).abilities,
        ["triggered!(Trigger::SpellCast(&TRIGGER1), &[Effect::gain_life(4)])"]
    );

    let rogue = "Name:Rogue Kavu\nManaCost:1 R\nTypes:Creature\nPT:1/1\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Alone$ True | Execute$ TrigPump\n\
        SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +2";
    assert_eq!(
        read(rogue).abilities,
        [
            "triggered!(Trigger::AttacksAlone(&Filter::This), &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(2), toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn invasion_batch_six_reader_patterns() {
    let skittish = "Name:Skittish Kavu\nManaCost:1 R\nTypes:Creature\nPT:1/1\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1 | AddToughness$ 1 | CheckSVar$ X | SVarCompare$ EQ0\n\
        SVar:X:Count$Valid Creature.White+OppCtrl/Plus.Y\n\
        SVar:Y:Count$Valid Creature.Blue+OppCtrl";
    assert_eq!(
        read(skittish).abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPT(1, 1), condition = Some(Condition::BattlefieldCountAtMost(&CHECK1, 0)))"
        ]
    );

    let sparring = "Name:Sparring Golem\nManaCost:3\nTypes:Artifact Creature\nPT:2/2\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigPump\n\
        SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +X | NumDef$ +X\n\
        SVar:X:Count$Valid Creature.blockingTriggeredAttacker";
    assert_eq!(
        read(sparring).abilities,
        [
            "triggered!(Trigger::BecomesBlocked, &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::CountOf { filter: &COUNT1, zone: ZoneSel::Battlefield }, toughness: Amount::CountOf { filter: &COUNT1, zone: ZoneSel::Battlefield }, keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn invasion_batch_seven_reader_patterns() {
    let moat = "Name:Teferi's Moat\nManaCost:3 W U\nTypes:Enchantment\n\
        K:ETBReplacement:Other:ChooseColor\n\
        SVar:ChooseColor:DB$ ChooseColor | Defined$ You\n\
        S:Mode$ CantAttack | ValidCard$ Creature.withoutFlying+ChosenColor | Target$ You";
    let moat_body = read(moat);
    assert_eq!(moat_body.enter_modifiers, ["EnterModifier::ChooseColor"]);
    assert_eq!(
        moat_body.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)), Filter::OfSourceChosenColor]), Modifier::CantAttackYou)"
        ]
    );

    let undermine = "Name:Undermine\nManaCost:U U B\nTypes:Instant\n\
        A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ DBDrain | Destination$ Graveyard\n\
        SVar:DBDrain:DB$ LoseLife | Defined$ TargetedController | LifeAmount$ 3";
    assert_eq!(
        read(undermine).abilities,
        [
            "spell!(&[Effect::CounterTargetSpell, Effect::LoseLife { amount: Amount::Fixed(3), target: PlayerRel::ControllerOfTarget }], targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))"
        ]
    );
}

#[test]
fn planeshift_batch_one_reader_patterns() {
    let destructive_flow = "Name:Destructive Flow\nManaCost:B R G\nTypes:Enchantment\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice | SacValid$ Land.nonBasic | Defined$ TriggeredPlayer | SacMessage$ nonbasic land";
    assert_eq!(
        read(destructive_flow).abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }, &[Effect::SacrificeFilter { who: PlayerRel::ActivePlayer, filter: &Filter::NONBASIC_LAND }])"
        ]
    );

    let ertai = "Name:Ertai, the Corrupted\nManaCost:2 W U B\nTypes:Legendary Creature Phyrexian Human Wizard\nPT:3/4\n\
        A:AB$ Counter | Cost$ U T Sac<1/Creature;Enchantment/creature or enchantment> | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card";
    assert_eq!(
        read(ertai).abilities,
        [
            "activated!(cost!(\"{U}\", TapSelf, Sacrifice(&COST1)), &[Effect::CounterTargetSpell], target = Some(TargetSpec::Spell(&Filter::Any)))"
        ]
    );
}

#[test]
fn planeshift_batch_two_reader_patterns() {
    let hobble = "Name:Hobble\nManaCost:2 W\nTypes:Enchantment Aura\n\
        K:Enchant:Creature\n\
        T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
        SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
        S:Mode$ CantAttack | ValidCard$ Creature.EnchantedBy\n\
        S:Mode$ CantBlock | ValidCard$ Creature.EnchantedBy+Black";
    assert_eq!(
        read(hobble).abilities,
        [
            "spell!(&[Effect::AttachSelf { target: TargetSpec::Object(&Filter::CREATURE) }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))",
            "triggered!(Trigger::ETB, &[Effect::draw(1)])",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::AddKeyword(KeywordSet::CANT_ATTACK))",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource, Filter::HasColor(ColorSet::from_slice(&[Color::Black]))]), Modifier::AddKeyword(KeywordSet::CANT_BLOCK))",
        ]
    );

    let mogg_jailer = "Name:Mogg Jailer\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/2\n\
        S:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ !controlsCreature.untapped+powerLE2";
    assert_eq!(
        read(mogg_jailer).abilities,
        [
            "static_ability!(Filter::This, Modifier::CantAttackIfDefenderControls(&DEFENDER_CONTROLS1))"
        ]
    );

    let bloodstock = "Name:Phyrexian Bloodstock\nManaCost:4 B\nTypes:Creature Phyrexian Zombie\nPT:3/3\n\
        T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigDestroy\n\
        SVar:TrigDestroy:DB$ Destroy | ValidTgts$ Creature.White | NoRegen$ True";
    assert_eq!(
        read(bloodstock).abilities,
        [
            "triggered!(Trigger::LeavesBattlefield(&Filter::This), &[Effect::destroy_no_regen(TargetSpec::Object(&TARGET1))], targets = Some(TargetReq::one(TargetSpec::Object(&TARGET1))))"
        ]
    );

    let tyranny = "Name:Phyrexian Tyranny\nManaCost:U B R\nTypes:Enchantment\n\
        T:Mode$ Drawn | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ TrigLose\n\
        SVar:TrigLose:DB$ LoseLife | Defined$ TriggeredCardController | LifeAmount$ 2 | UnlessCost$ 2 | UnlessPayer$ TriggeredCardController";
    assert_eq!(
        read(tyranny).abilities,
        [
            "triggered!(Trigger::Draws(PlayerRel::EachPlayer), &[Effect::PlayerMayPayOr { player: PlayerRel::EventPlayer, mana: Amount::Fixed(2), effect: &Effect::LoseLife { amount: Amount::Fixed(2), target: PlayerRel::EventPlayer } }])"
        ]
    );
}

#[test]
fn planeshift_batch_three_and_four_reader_patterns() {
    let pilgrim = "Name:Samite Pilgrim\nManaCost:1 W\nTypes:Creature Human Cleric\nPT:1/1\n\
        A:AB$ PreventDamage | Cost$ T | ValidTgts$ Creature | Amount$ X | PrecostDesc$ Domain —\n\
        SVar:X:Count$Domain";
    assert_eq!(
        read(pilgrim).abilities,
        [
            "activated!(Cost::TAP, &[Effect::PreventNextDamage { target: TargetSpec::Object(&Filter::CREATURE), amount: Amount::BasicLandTypesAmong(&Filter::YOUR_LAND) }], target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let explorer = "Name:Quirion Explorer\nManaCost:1 G\nTypes:Creature Elf Druid Scout\nPT:1/1\n\
        A:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Land.OppCtrl | ReflectProperty$ Produce";
    assert_eq!(
        read(explorer).abilities,
        ["mana_ability!(&[Effect::mana_land_color(false)])"]
    );

    let compass = "Name:Star Compass\nManaCost:2\nTypes:Artifact\n\
        A:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Basic.YouCtrl | ReflectProperty$ Produce";
    assert_eq!(
        read(compass).abilities,
        ["mana_ability!(&[Effect::mana_from_lands(&Filter::BASIC_LAND, true, false)])"]
    );

    let radiant_kavu = "Name:Radiant Kavu\nManaCost:R G W\nTypes:Creature Kavu\nPT:3/3\n\
        A:AB$ Effect | Cost$ R G W | ReplacementEffects$ RPrevent\n\
        SVar:RPrevent:Event$ DamageDone | Prevent$ True | IsCombat$ True | ActiveZones$ Command | ValidSource$ Creature.Blue,Creature.Black";
    assert_eq!(
        read(radiant_kavu).abilities,
        [
            "activated!(cost!(\"{R}{G}{W}\"), &[Effect::continuous(&EFFECT1, Modifier::PreventDamageFromIt, Duration::UntilEndOfTurn)])"
        ]
    );

    let familiar = "Name:Stormscape Familiar\nManaCost:1 U\nTypes:Creature Bird\nPT:1/1\n\
        S:Mode$ ReduceCost | ValidCard$ Card.White,Card.Black | Type$ Spell | Activator$ You | Amount$ 1";
    assert_eq!(
        read(familiar).abilities,
        [
            "static_ability!(Filter::Or(&[Filter::And(&[Filter::HasColor(ColorSet::from_slice(&[Color::White])), Filter::ControlledByYou]), Filter::And(&[Filter::HasColor(ColorSet::from_slice(&[Color::Black])), Filter::ControlledByYou])]), Modifier::SpellsCostLess(1))"
        ]
    );

    let sunken_hope = "Name:Sunken Hope\nManaCost:3 U U\nTypes:Enchantment\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigChangeZoneYou\n\
        SVar:TrigChangeZoneYou:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Hidden$ True | Mandatory$ True | ChangeType$ Creature | DefinedPlayer$ TriggeredPlayer | ChangeNum$ 1";
    assert_eq!(
        read(sunken_hope).abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }, &[Effect::ReturnChosenToHand { who: PlayerRel::ActivePlayer, filter: &Filter::CREATURE }])"
        ]
    );
}

#[test]
fn apocalypse_reader_patterns() {
    let divine_light = "Name:Divine Light\nManaCost:W\nTypes:Sorcery\n\
        A:SP$ Effect | ReplacementEffects$ RPrevent\n\
        SVar:RPrevent:Event$ DamageDone | Prevent$ True | ActiveZones$ Command | ValidTarget$ Creature.YouCtrl";
    assert_eq!(
        read(divine_light).abilities,
        [
            "spell!(&[Effect::continuous(&Filter::YOUR_CREATURE, Modifier::PreventDamageFrom(&Filter::Any), Duration::UntilEndOfTurn)])"
        ]
    );

    let consume_strength = "Name:Consume Strength\nManaCost:1 B G\nTypes:Instant\n\
        A:SP$ Pump | ValidTgts$ Creature | TargetUnique$ True | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBPumpNeg\n\
        SVar:DBPumpNeg:DB$ Pump | ValidTgts$ Creature | TargetUnique$ True | NumAtt$ -2 | NumDef$ -2 | IsCurse$ True";
    let body = read(consume_strength);
    assert!(
        body.statics
            .contains("Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Reference)])"),
        "{}",
        body.statics
    );
    assert!(
        body.abilities[0].contains("second_targets = Some(TargetReq::one("),
        "{}",
        body.abilities[0]
    );

    let bloodfire_infusion = "Name:Bloodfire Infusion\nManaCost:2 R\nTypes:Enchantment Aura\n\
        K:Enchant:Creature.YouCtrl\n\
        A:AB$ DamageAll | Cost$ R Sac<1/Enchanted> | ValidCards$ Creature | NumDmg$ X | ActivationZone$ Battlefield\n\
        SVar:X:Sacrificed$CardPower";
    assert!(read(bloodfire_infusion).abilities[1].contains(
        "Effect::DealDamageEach { amount: Amount::SacrificedPower, filter: &Filter::CREATURE }"
    ),);

    let planar_despair = "Name:Planar Despair\nManaCost:3 B B\nTypes:Sorcery\n\
        A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True\n\
        SVar:X:Count$Domain";
    assert_eq!(
        read(planar_despair).abilities,
        [
            "spell!(&[Effect::PumpFilter { filter: &Filter::CREATURE, controlled_by: None, power: Amount::Negated(&Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)), toughness: Amount::Negated(&Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );

    let urborg_uprising = "Name:Urborg Uprising\nManaCost:4 B\nTypes:Sorcery\n\
        A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDraw\n\
        SVar:DBDraw:DB$ Draw | NumCards$ 1";
    assert_eq!(
        read(urborg_uprising).abilities,
        [
            "spell!(&[Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You) }, Effect::draw(1)], targets = Some(TargetReq::up_to(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You), 2)))"
        ]
    );
}

#[test]
fn odyssey_reader_patterns() {
    let horror = "Name:Childhood Horror\nManaCost:3 B\nTypes:Creature\nPT:2/2\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | AddToughness$ 2 | Condition$ Threshold\n\
        S:Mode$ CantBlock | ValidCard$ Card.Self | Condition$ Threshold | Secondary$ True";
    assert_eq!(
        read(horror).abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPT(2, 2), condition = Some(Condition::GraveyardCountAtLeast(7)))",
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CANT_BLOCK), condition = Some(Condition::GraveyardCountAtLeast(7)))",
        ]
    );

    let basilisk = "Name:Stone-Tongue Basilisk\nManaCost:4 G G G\nTypes:Creature\nPT:4/5\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddHiddenKeyword$ All creatures able to block CARDNAME do so. | Condition$ Threshold\n\
        T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Creature | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ DelTrig\n\
        SVar:DelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | ValidPlayer$ Player | Execute$ TrigDestroy | RememberObjects$ TriggeredTargetLKICopy\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ DelayTriggerRememberedLKI";
    assert_eq!(
        read(basilisk).abilities,
        [
            "static_ability!(Filter::This, Modifier::MustBeBlockedByAllAble, condition = Some(Condition::GraveyardCountAtLeast(7)))",
            "triggered!(Trigger::DealsCombatDamageToCreature(&Filter::This), &[Effect::AtEndOfCombat { about: TargetSpec::EventObject, effects: &[Effect::destroy(TargetSpec::EventObject)] }])",
        ]
    );

    let dragon = "Name:Vampiric Dragon\nManaCost:6 B R\nTypes:Creature\nPT:5/5\n\
        T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.DamagedBy | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
        SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1";
    assert_eq!(
        read(dragon).abilities,
        [
            "triggered!(Trigger::DiesAfterDamageByThis(&Filter::CREATURE), &[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }])",
        ]
    );

    let epicenter = "Name:Epicenter\nManaCost:4 R\nTypes:Sorcery\n\
        A:SP$ Branch | ValidTgts$ Player | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE7 | TrueSubAbility$ ThreshWaste | FalseSubAbility$ WasteLand\n\
        SVar:WasteLand:DB$ Sacrifice | Defined$ Targeted | SacValid$ Land\n\
        SVar:ThreshWaste:DB$ SacrificeAll | ValidCards$ Land\n\
        SVar:X:Count$ValidGraveyard Card.YouOwn";
    assert_eq!(
        read(epicenter).abilities,
        [
            "spell!(&[Effect::IfCondition { condition: Condition::GraveyardCountAtLeast(7), then: &[Effect::SacrificeAll { filter: &Filter::LAND }], otherwise: &[Effect::SacrificeFilter { who: PlayerRel::Chosen, filter: &Filter::LAND }] }], targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))",
        ]
    );
}

#[test]
fn mirrodin_reader_patterns() {
    let frogmite =
        "Name:Frogmite\nManaCost:4\nTypes:Artifact Creature\nPT:2/2\nK:Affinity:Artifact";
    assert_eq!(
        read(frogmite).cost_reduction.as_deref(),
        Some(
            "Some(CostReduction::PerCount { amount: Amount::CountOf { filter: &Filter::YOUR_ARTIFACT, zone: ZoneSel::Battlefield }, each: 1 })"
        )
    );

    let bosh = "Name:Bosh, Iron Golem\nManaCost:8\nTypes:Legendary Artifact Creature\nPT:6/7\n\
        A:AB$ DealDamage | Cost$ 3 R Sac<1/Artifact> | ValidTgts$ Any | NumDmg$ X\n\
        SVar:X:Sacrificed$CardManaCost";
    assert_eq!(
        read(bosh).abilities,
        [
            "activated!(cost!(\"{3}{R}\", Sacrifice(&Filter::YOUR_ARTIFACT)), &[Effect::DealDamage { amount: Amount::SacrificedManaValue, target: TargetSpec::AnyTarget }], target = Some(TargetSpec::AnyTarget))",
        ]
    );

    let beetle = "Name:Clockwork Beetle\nManaCost:1\nTypes:Artifact Creature\nPT:0/0\n\
        K:etbCounter:P1P1:2\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDelayed\n\
        T:Mode$ Blocks | ValidCard$ Card.Self | Secondary$ True | Execute$ TrigDelayed\n\
        SVar:TrigDelayed:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | Execute$ TrigRemoveCounter\n\
        SVar:TrigRemoveCounter:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1";
    assert_eq!(
        read(beetle).abilities,
        [
            "triggered!(Trigger::Attacks(&Filter::This), &[Effect::AtEndOfCombat { about: TargetSpec::EventObject, effects: &[Effect::RemoveCounterSelf { kind: CounterKind::P1P1, n: 1 }] }])",
            "triggered!(Trigger::Blocks(&Filter::This), &[Effect::AtEndOfCombat { about: TargetSpec::EventObject, effects: &[Effect::RemoveCounterSelf { kind: CounterKind::P1P1, n: 1 }] }])",
        ]
    );
}

// Table-style regression: all batch-two reader patterns share one assertion helper.
#[allow(clippy::too_many_lines)]
#[test]
fn mirrodin_batch_two_reader_patterns() {
    let assert_reads = |script: &str, expected: &[&str]| {
        let parsed = parse(script);
        let body = transcode(&parsed, &cats(), None).unwrap_or_else(|| {
            panic!(
                "should be read in full: {:?}\n{script}",
                refusal_reason(&parsed, &cats(), None)
            )
        });
        let rendered = format!(
            "{} {}\n{}",
            body.statics,
            body.entwine.as_deref().unwrap_or(""),
            body.abilities.join("\n")
        );
        for needle in expected {
            assert!(
                rendered.contains(needle),
                "missing `{needle}` in {rendered}"
            );
        }
    };

    assert_reads(
        "Name:Two activation costs\nTypes:Artifact\n\
         A:AB$ DealDamage | Cost$ 3 T | AlternateCost$ R T | ValidTgts$ Any | NumDmg$ 1",
        &["cost!(\"{3}\", TapSelf)", "cost!(\"{R}\", TapSelf)"],
    );
    assert_reads(
        "Name:Two modes\nManaCost:2 G\nTypes:Sorcery\nK:Entwine:2 G\n\
         A:SP$ Charm | Choices$ Search,Grow\n\
         SVar:Search:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land | ChangeNum$ 2 | Optional$ True\n\
         SVar:Grow:DB$ Effect | StaticAbilities$ Lands\n\
         SVar:Lands:Mode$ Continuous | Affected$ You | AdjustLandPlays$ 2",
        &[
            "Some(cost!(\"{2}{G}\"))",
            "AbilityDef::ModalSpell",
            "Modifier::ExtraLandDrops(2)",
        ],
    );
    assert_reads(
        "Name:No creatures\nTypes:Artifact Creature\nPT:4/6\n\
         S:Mode$ CantBeCast | ValidCard$ Creature | Caster$ You",
        &[
            "Modifier::PlayersCantCast",
            "who: PlayerRel::You",
            "filter: &Filter::CREATURE",
        ],
    );
    assert_reads(
        "Name:Combat loot\nTypes:Artifact Equipment\nK:Equip:1\n\
         T:Mode$ DamageDone | ValidSource$ Creature.AttachedBy | ValidTarget$ Player | CombatDamage$ True | Execute$ Loot | TriggerZones$ Battlefield\n\
         SVar:Loot:AB$ Discard | Defined$ You | Mode$ TgtChoose | NumCards$ 1 | Cost$ Draw<2/You>",
        &[
            "Trigger::DealsCombatDamageToPlayer",
            "Effect::MayDo",
            "Effect::draw(2)",
            "Effect::DiscardForPlayers",
        ],
    );
    assert_reads(
        "Name:Draw with payment\nTypes:Artifact\n\
         T:Mode$ Drawn | ValidCard$ Card.OppOwn | TriggerZones$ Battlefield | Execute$ Draw\n\
         SVar:Draw:AB$ Draw | Cost$ 1 | Defined$ You | NumCards$ 1",
        &[
            "Trigger::Draws(PlayerRel::EachOpponent)",
            "Effect::PlayerMayPayThen",
            "Effect::draw(1)",
        ],
    );
    assert_reads(
        "Name:Nonbasic land destruction\nManaCost:1 R R\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Land | SubAbility$ Damage\n\
         SVar:Damage:DB$ DealDamage | Defined$ TargetedController | NumDmg$ 2 | ConditionDefined$ Targeted | ConditionPresent$ Land.Basic | ConditionCompare$ EQ0",
        &[
            "Effect::IfTargetMatches",
            "filter: &Filter::NONBASIC_LAND",
            "TargetSpec::Player(PlayerRel::ControllerOfTarget)",
        ],
    );
    assert_reads(
        "Name:Shuffle activation\nTypes:Artifact Creature\nPT:1/1\n\
         A:AB$ Shuffle | Cost$ 2 T | Defined$ You",
        &["Effect::ShuffleLibrary", "who: PlayerRel::You"],
    );
    assert_reads(
        "Name:Reveal and sort\nManaCost:1 U\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ Dig\n\
         SVar:Dig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Artifact | DestinationZone2$ Graveyard",
        &[
            "Effect::RevealTopAndSort",
            "matched: SearchDest::Hand",
            "otherwise: SearchDest::Graveyard",
        ],
    );
    assert_reads(
        "Name:Artifact evasion\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Card.Self | IsPresent$ Artifact.DefenderCtrl",
        &[
            "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)",
            "Condition::SomeDefendingPlayerControlsNone",
        ],
    );
    assert_reads(
        "Name:Artifact blocker\nManaCost:4 R\nTypes:Creature\nPT:3/3\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature.Artifact | TriggerZones$ Battlefield | Execute$ Destroy\n\
         SVar:Destroy:DB$ Destroy | Defined$ TriggeredBlockerLKICopy",
        &[
            "Trigger::BecomesBlockedBy",
            "Effect::destroy(TargetSpec::EventObject)",
        ],
    );
    assert_reads(
        "Name:Counted unless cost\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ Y\n\
         SVar:Y:Count$Valid Artifact.YouCtrl",
        &[
            "Effect::PlayerMayPayOr",
            "player: PlayerRel::ControllerOfTarget",
            "filter: &Filter::YOUR_ARTIFACT",
        ],
    );
    assert_reads(
        "Name:Empty library\nTypes:Artifact Creature\nPT:10/10\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Exile\n\
         SVar:Exile:DB$ ChangeZoneAll | ChangeType$ Card.YouOwn | Origin$ Library | Destination$ Exile",
        &[
            "Effect::ExileTop",
            "count: Amount::LibrarySize(PlayerRel::You)",
        ],
    );
    assert_reads(
        "Name:Animate artifacts\nManaCost:3 U\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Artifact.nonCreature | SetPower$ AffectedX | SetToughness$ AffectedX | AddType$ Creature\n\
         SVar:AffectedX:Count$CardManaCost",
        &[
            "Filter::And(&[Filter::ARTIFACT, Filter::NONCREATURE])",
            "Modifier::AnimateNoncreatureArtifact",
        ],
    );
    assert_reads(
        "Name:Counter combat costs\nManaCost:5\nTypes:Artifact Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Counter\n\
         SVar:Counter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1\n\
         S:Mode$ CantAttackUnless | ValidCard$ Card.Self | Cost$ Y\n\
         S:Mode$ CantBlockUnless | ValidCard$ Card.Self | Cost$ Y\n\
         SVar:Y:Count$CardCounters.P1P1",
        &[
            "Modifier::CantAttackUnlessPayManaPerCounter(CounterKind::P1P1)",
            "Modifier::CantBlockUnlessPayManaPerCounter(CounterKind::P1P1)",
        ],
    );
    assert_reads(
        "Name:Printed tap correction\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ 6 Sac<1/CARDNAME> | Defined$ You | NumCards$ 1\n\
         Oracle:{6}, {T}, Sacrifice this artifact: Draw a card.",
        &["cost!(\"{6}\", TapSelf, SacrificeSelf)"],
    );
    assert_reads(
        "Name:Printed tap correction after sacrifice\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ 6 Sac<1/CARDNAME> | Defined$ You | NumCards$ 1\n\
         Oracle:{6}, Sacrifice this artifact, {T}: Draw a card.",
        &["cost!(\"{6}\", SacrificeSelf, TapSelf)"],
    );
    assert_reads(
        "Name:Linked instant\nTypes:Artifact\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | OptionalDecider$ You | Execute$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Imprint$ True | Origin$ Hand | Destination$ Exile | ChangeType$ Instant.cmcLE2 | ChangeNum$ 1\n\
         A:AB$ Play | Cost$ 2 T | Valid$ Card.IsImprinted+ExiledWithSource | ValidZone$ Exile | Amount$ All | WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | CopyCard$ True",
        &[
            "Effect::ExileChosenFromHandWithSource",
            "Filter::CmcAtMost(2)",
            "Effect::MayCopyCastLinkedExile",
        ],
    );
    assert_reads(
        "Name:Coin replacement\nTypes:Artifact\n\
         S:Mode$ FlipCoinDoubler | ValidPlayer$ You",
        &["Modifier::FlipTwoCoinsIgnoreOne"],
    );
    assert_reads(
        "Name:Next turn control\nTypes:Artifact\n\
         A:AB$ ControlPlayer | Cost$ 4 T Sac<1/CARDNAME> | ValidTgts$ Player",
        &[
            "Effect::ControlPlayerNextTurn",
            "player: PlayerRel::Chosen",
            "TargetSpec::AnyPlayer",
        ],
    );
    assert_reads(
        "Name:Named cast history\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | ChangeType$ Card.nonLand | ChangeNum$ 1 | Imprint$ True | Reveal$ True | Shuffle$ False | StackDescription$ SpellDescription | SubAbility$ Repeat | SpellDescription$ Search and reveal, punish matching casts, then shuffle.\n\
         SVar:Repeat:DB$ RepeatEach | RepeatPlayers$ Opponent | RepeatSubAbility$ Lose | SubAbility$ Shuffle\n\
         SVar:Lose:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ 6 | ConditionCheckSVar$ LoseLifeCheck | ConditionSVarCompare$ GE1\n\
         SVar:Shuffle:DB$ Shuffle | Defined$ You | SubAbility$ Cleanup\n\
         SVar:Cleanup:DB$ Cleanup | ClearImprinted$ True\n\
         SVar:LoseLifeCheck:Count$ThisTurnCast_Card.RememberedPlayerCtrl+sharesNameWith Imprinted",
        &[
            "Effect::RevealLibrarySearch",
            "Effect::PlayersWhoCastRevealedNameThisTurnLoseLife",
            "Effect::ShuffleLibrary",
        ],
    );
    assert_reads(
        "Name:Guess a hand\nTypes:Artifact\n\
         A:AB$ NameCard | Cost$ 2 T | Defined$ You | SubAbility$ Guess\n\
         SVar:Guess:DB$ GenericChoice | ValidTgts$ Opponent | Choices$ GuessInHand,GuessNotInHand | ShowChoice$ True | Guess$ True\n\
         SVar:GuessInHand:DB$ RevealHand | Defined$ You | Optional$ True | RememberRevealedPlayer$ True | SubAbility$ DBInHandDraw\n\
         SVar:GuessNotInHand:DB$ RevealHand | Defined$ You | Optional$ True | RememberRevealedPlayer$ True | SubAbility$ DBNotInHandDraw\n\
         SVar:DBInHandDraw:DB$ Draw | Defined$ You | ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | SubAbility$ Cleanup\n\
         SVar:DBNotInHandDraw:DB$ Draw | Defined$ You | ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You | ConditionCheckSVar$ X | SubAbility$ Cleanup\n\
         SVar:Cleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$ValidHand Card.NamedCard",
        &[
            "Effect::ChooseCardName",
            "Effect::GuessChosenNameInHand",
            "Effect::RevealHandForGuess",
            "Effect::DrawIfGuessWrong",
        ],
    );
    assert_reads(
        "Name:Artifact tax\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | ValidCard$ Artifact | Activator$ Player | Type$ Spell | Amount$ X | Relative$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
        &["Modifier::SpellsCostMorePerControlled(&Filter::ARTIFACT)"],
    );
    assert_reads(
        "Name:No game loss\nTypes:Artifact Creature\nPT:4/4\n\
         R:Event$ GameLoss | ActiveZones$ Battlefield | ValidPlayer$ You | Layer$ CantHappen\n\
         R:Event$ GameWin | ActiveZones$ Battlefield | ValidPlayer$ Opponent | Layer$ CantHappen | Secondary$ True",
        &["Modifier::NamedPlayersCantLose", "Modifier::PlayersCantWin"],
    );
}

#[test]
fn affinity_scopes_every_alternative_and_refuses_extra_fields() {
    let body = read("Name:X\nManaCost:4\nTypes:Artifact\nK:Affinity:Artifact,Creature");
    assert_eq!(
        body.statics.matches("Filter::ControlledByYou").count(),
        2,
        "each alternative is something you control: {}",
        body.statics
    );
    assert!(body.statics.contains("Filter::ARTIFACT"));
    assert!(body.statics.contains("Filter::CREATURE"));
    assert!(refused(
        "Name:X\nManaCost:4\nTypes:Artifact\nK:Affinity:Artifact:UnreadRule"
    ));
}

#[test]
fn exile_sweeps_link_only_when_a_companion_reads_the_link() {
    let entry = "T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Exile\n\
        SVar:Exile:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature";
    let body = read(&format!("Name:X\nTypes:Creature\n{entry}"));
    assert!(body.abilities[0].contains("Effect::ExileAll"));
    assert!(body.abilities[0].contains("linked: false"));

    let body = read(&format!(
        "Name:X\nTypes:Creature\n{entry}\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ Return\n\
         SVar:Return:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Battlefield | ChangeType$ Card.ExiledWithSource"
    ));
    assert!(body.abilities[0].contains("linked: true"));
    assert!(body.abilities[1].contains("Effect::ReturnLinkedToBattlefield"));
}
