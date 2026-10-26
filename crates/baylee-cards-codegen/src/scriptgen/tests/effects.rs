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
