use super::*;

#[test]
fn wave25a_keyword_counters_and_etb_counters() {
    for (code, expected) in [
        ("First Strike", "CounterKind::FirstStrike"),
        ("Double Strike", "CounterKind::DoubleStrike"),
        ("Haste", "CounterKind::Haste"),
        ("Hexproof", "CounterKind::Hexproof"),
        ("Menace", "CounterKind::Menace"),
        ("Reach", "CounterKind::Reach"),
        ("Trample", "CounterKind::Trample"),
        ("Vigilance", "CounterKind::Vigilance"),
        ("SHIELD", "CounterKind::Shield"),
        ("FINALITY", "CounterKind::Finality"),
    ] {
        let put = read(&format!(
            "Name:CounterSpell\nManaCost:1 G\nTypes:Sorcery\n\
             A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ {code} | CounterNum$ 1"
        ));
        assert!(
            put.abilities[0].contains(&format!(
                "Effect::AddCounter {{ kind: {expected}, amount: Amount::Fixed(1) }}"
            )),
            "code={code} got {:?}",
            put.abilities
        );

        let etb = read(&format!(
            "Name:CounterBeast\nManaCost:2 G\nTypes:Creature\nPT:2/2\nK:etbCounter:{code}:1"
        ));
        assert_eq!(
            etb.enter_modifiers,
            [format!(
                "EnterModifier::WithCounters {{ kind: {expected}, amount: Amount::Fixed(1) }}"
            )],
            "code={code}"
        );
    }

    // Exact case sensitivity preserved for CR 122.1b keyword counters.
    for bad_case in [
        "MENACE",
        "trample",
        "FIRST STRIKE",
        "DOUBLE_STRIKE",
        "VIGILANCE",
    ] {
        assert!(
            refused(&format!(
                "Name:BadCase\nManaCost:1 G\nTypes:Sorcery\n\
                 A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ {bad_case} | CounterNum$ 1"
            )),
            "must refuse wrong case `{bad_case}`"
        );
    }
}

#[test]
fn wave25a_separate_into_two_piles_and_hidden_info_refusals() {
    // Steam Augury (You separate, Opponent chooses, Hand / Graveyard).
    let steam = read(
        "Name:Steam Augury\nManaCost:2 U R\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ 5 | RememberRevealed$ True | NoPeek$ True | SubAbility$ DBTwoPiles\n\
         SVar:DBTwoPiles:DB$ TwoPiles | Chooser$ Opponent | DefinedCards$ Remembered | Separator$ You | ChosenPile$ DBHand | UnchosenPile$ DBGrave\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBGrave:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard",
    );
    assert!(
        steam.abilities[0].contains("Effect::RevealUntilThen")
            && steam.abilities[0].contains("Effect::SeparateIntoTwoPiles { separator: PlayerRel::You, pile_chooser: PlayerRel::Opponent, zone: ZoneSel::LibraryYou, filter: &Filter::Any, only_found: true")
            && steam.abilities[0].contains("not_chosen_movement: ChooseMove::Hand")
            && steam.abilities[0].contains("not_chosen_movement: ChooseMove::Graveyard"),
        "{:?}",
        steam.abilities
    );

    // Epiphany at the Drownyard (dynamic X+1 count, You separate, Opponent chooses).
    let epiphany = read(
        "Name:Epiphany at the Drownyard\nManaCost:X U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ Y | RememberRevealed$ True | NoPeek$ True | SubAbility$ DBTwoPiles\n\
         SVar:DBTwoPiles:DB$ TwoPiles | Chooser$ Opponent | DefinedCards$ Remembered | Separator$ You | ChosenPile$ DBHand | UnchosenPile$ DBGrave\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBGrave:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard\n\
         SVar:Y:Count$xPaid/Plus.1\nSVar:X:Count$xPaid",
    );
    assert!(
        epiphany.abilities[0].contains("Effect::SeparateIntoTwoPiles")
            && epiphany.abilities[0].contains("Amount::Plus"),
        "{:?}",
        epiphany.abilities
    );

    // Jace, Architect of Thought `-2` shape (Hand + BottomOfLibraryChosenOrder).
    let jace_minus = read(
        "Name:JacePiles\nManaCost:2 U U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ 3 | RememberRevealed$ True | NoPeek$ True | SubAbility$ DBTwoPiles\n\
         SVar:DBTwoPiles:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | ChosenPile$ DBHand | UnchosenPile$ DBLibraryBottom\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBLibraryBottom:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1",
    );
    assert!(
        jace_minus.abilities[0]
            .contains("not_chosen_movement: ChooseMove::BottomOfLibraryChosenOrder"),
        "{:?}",
        jace_minus.abilities
    );

    // Random bottom pile destination.
    let random_bottom = read(
        "Name:RandomBottomPiles\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ 4 | RememberRevealed$ True | NoPeek$ True | SubAbility$ DBTwoPiles\n\
         SVar:DBTwoPiles:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | ChosenPile$ DBHand | UnchosenPile$ DBLibraryBottom\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBLibraryBottom:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True",
    );
    assert!(
        random_bottom.abilities[0]
            .contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{:?}",
        random_bottom.abilities
    );

    // Canonical Fact or Fiction fast path stays `Effect::RevealAndSeparate { count: 5 }`.
    let fof = read(
        "Name:Fact or Fiction\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ 5 | RememberRevealed$ True | NoPeek$ True | SubAbility$ DBTwoPiles\n\
         SVar:DBTwoPiles:DB$ TwoPiles | Defined$ You | DefinedCards$ Remembered | Separator$ Opponent | ChosenPile$ DBHand | UnchosenPile$ DBGrave\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBGrave:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard",
    );
    assert!(
        fof.abilities[0].contains("Effect::RevealAndSeparate { count: 5 }"),
        "{:?}",
        fof.abilities
    );

    // Honest refusals: face-down piles (Atris/Fortune's Favor), Truth or Tale unified bottom,
    // Intrude on the Mind RememberChanged on DBGrave, Hostile Negotiations pre-formed piles.
    for bad in [
        "Name:AtrisRefusal\nManaCost:2 U B\nTypes:Instant\n\
         A:SP$ PeekAndReveal | Defined$ You | PeekAmount$ 3 | NoReveal$ True | RememberPeeked$ True | SubAbility$ Sep\n\
         SVar:Sep:DB$ TwoPiles | Defined$ You | Separator$ Opponent | Chooser$ You | DefinedCards$ Remembered | ChosenPile$ H | UnchosenPile$ G | FaceDown$ One\n\
         SVar:H:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:G:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard",
        "Name:TruthOrTaleRefusal\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ 5 | NoPeek$ True | ImprintRevealed$ True | SubAbility$ DBTwoPile\n\
         SVar:DBTwoPile:DB$ TwoPiles | Chooser$ Opponent | DefinedCards$ Imprinted | Separator$ You | ChosenPile$ DBHand\n\
         SVar:DBHand:DB$ ChooseCard | Defined$ You | Choices$ Card.IsRemembered | ChoiceZone$ Library | Mandatory$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Library | Destination$ Hand",
        "Name:IntrudeRefusal\nManaCost:3 U U\nTypes:Instant\n\
         A:SP$ PeekAndReveal | PeekAmount$ 5 | RememberRevealed$ True | NoPeek$ True | SubAbility$ DBTwoPiles\n\
         SVar:DBTwoPiles:DB$ TwoPiles | Chooser$ Opponent | DefinedCards$ Remembered | Separator$ You | ChosenPile$ DBHand | UnchosenPile$ DBGrave\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBGrave:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Graveyard | RememberChanged$ True",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave25a_dig_exile_choose_play_and_move_chains() {
    // You Find Some Prisoners (target opponent library, ExileWithPermission spend_mana_as_any_color).
    let prisoners = read(
        "Name:You Find Some Prisoners\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ Dig | ValidTgts$ Opponent | DigNum$ 3 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | Mandatory$ True | Choices$ Card.IsRemembered | ChoiceZone$ Exile | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ STMayPlay | Duration$ UntilTheEndOfYourNextTurn | RememberObjects$ ChosenCard | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True\n\
         SVar:STMayPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True | MayPlayIgnoreColor$ True",
    );
    assert!(
        prisoners.abilities[0].contains("Effect::RevealUntilThen { library: PlayerRel::Chosen")
            && prisoners.abilities[0].contains("zone: ChooseZone::Found")
            && prisoners.abilities[0].contains(
                "Effect::ExileWithPermission { target: TargetSpec::EventObject, until: None, permission: ExilePermission::play(PlayWindow::ThroughYourNextTurn).spend_mana_as_any_color() }"
            ),
        "{:?}",
        prisoners.abilities
    );

    // Chandra / Strongbox Raider / Feldon pattern (`ForgetOtherRemembered$ True` + `Affected$ Card.ChosenCard`).
    let chandra_plus = read(
        "Name:ChandraPlus\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ 3 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | Choices$ Card.IsRemembered | ChoiceZone$ Exile | Mandatory$ True | ForgetOtherRemembered$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ ChosenCard | StaticAbilities$ Play | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.ChosenCard | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        chandra_plus.abilities[0].contains("zone: ChooseZone::Found")
            && chandra_plus.abilities[0].contains("ExilePermission::play(PlayWindow::ThisTurn)"),
        "{:?}",
        chandra_plus.abilities
    );

    // Party Thrasher / Jaya `-1` pattern (intermediate `Cleanup | ClearRemembered$ True` before `DBPlayEffect`).
    let jaya_minus = read(
        "Name:JayaMinus\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | Amount$ 1 | Mandatory$ True | Choices$ Card.IsRemembered | ChoiceZone$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | SubAbility$ DBPlayEffect\n\
         SVar:DBPlayEffect:DB$ Effect | StaticAbilities$ STPlay | ExileOnMoved$ Exile | SubAbility$ DBClearChosen\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.ChosenCard | AffectedZone$ Exile\n\
         SVar:DBClearChosen:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        jaya_minus.abilities[0].contains("zone: ChooseZone::Found")
            && jaya_minus.abilities[0].contains("ExilePermission::play(PlayWindow::ThisTurn)"),
        "{:?}",
        jaya_minus.abilities
    );

    // Lord of the Void pattern (`Dig` -> `ChooseCard` `Creature.IsRemembered` -> `ChangeZone` Battlefield `GainControl$ True`).
    let lord = read(
        "Name:Lord of the Void\nManaCost:4 B B B\nTypes:Creature Demon\nPT:7/7\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigExile | TriggerZones$ Battlefield\n\
         SVar:TrigExile:DB$ Dig | DigNum$ 7 | ChangeNum$ All | Defined$ TriggeredTarget | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ PickOne\n\
         SVar:PickOne:DB$ ChooseCard | Defined$ You | Amount$ 1 | Mandatory$ True | Choices$ Creature.IsRemembered | ChoiceZone$ Exile | SubAbility$ MoveChosen\n\
         SVar:MoveChosen:DB$ ChangeZone | Origin$ Exile | Destination$ Battlefield | GainControl$ True | Defined$ ChosenCard | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        lord.abilities[0].contains("library: PlayerRel::DamagedPlayer")
            && lord.abilities[0].contains("zone: ChooseZone::Found")
            && lord.abilities[0].contains("movement: ChooseMove::Battlefield { tapped: false }"),
        "{:?}",
        lord.abilities
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn wave25a_choose_card_ex_for_unchosen_exile_and_hand() {
    // Archfiend of Depravity (single-seat `TriggeredPlayer`, `ControlledByPlayer$ Chooser`, `SacrificeAll` unchosen).
    let archfiend = read(
        "Name:Archfiend of Depravity\nManaCost:3 B B\nTypes:Creature Demon\nPT:5/4\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigChoose\n\
         SVar:TrigChoose:DB$ ChooseCard | Defined$ TriggeredPlayer | MinAmount$ 0 | Amount$ 2 | ControlledByPlayer$ Chooser | Choices$ Creature | Reveal$ True | SubAbility$ DBSacrificeAll\n\
         SVar:DBSacrificeAll:DB$ SacrificeAll | ValidCards$ Creature.nonChosenCard | Controller$ TriggeredPlayer | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        archfiend.abilities[0].contains("Effect::ChooseCardEx")
            && archfiend.abilities[0].contains("chooser: PlayerRel::ActivePlayer")
            && archfiend.abilities[0].contains("reveal_chosen: false")
            && archfiend.abilities[0].contains("for_unchosen: &[Effect::RepeatEachObject { zone: ZoneSel::Battlefield, filter: &Filter::Any, only_found: true, then: &[Effect::SacrificeEvent] }]")
            && archfiend.statics.contains("Filter::ControlledByChooser"),
        "{:?} / {:?}",
        archfiend.abilities,
        archfiend.statics
    );

    // Divine Reckoning (`EachPlayer`, `DestroyAll` unchosen).
    let reckoning = read(
        "Name:Divine Reckoning\nManaCost:2 W W\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Player | Amount$ 1 | Choices$ Creature | ControlledByPlayer$ Chooser | Mandatory$ True | SubAbility$ DBDestroyAll\n\
         SVar:DBDestroyAll:DB$ DestroyAll | ValidCards$ Creature.!ChosenCard",
    );
    assert!(
        reckoning.abilities[0].contains("Effect::RepeatEach { domain: RepeatDomain::EachPlayer")
            && reckoning.abilities[0]
                .contains("Effect::Destroy { target: TargetSpec::EventObject, no_regen: false }"),
        "{:?}",
        reckoning.abilities
    );

    // Consuming Tide body (`not_chosen_movement: ChooseMove::Hand` + trailing follow-up outside RepeatEach).
    let tide = read(
        "Name:TideSweep\nManaCost:2 U U\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Player | Mandatory$ True | ControlledByPlayer$ Chooser | Choices$ Permanent.nonLand | SubAbility$ DBReturnAll\n\
         SVar:DBReturnAll:DB$ ChangeZoneAll | ChangeType$ Permanent.nonLand+!ChosenCard | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        tide.abilities[0].contains("not_chosen_movement: ChooseMove::Hand")
            && tide.abilities[0].contains("], }, Effect::draw(1)")
            || tide.abilities[0].contains("Effect::draw(1)"),
        "{:?}",
        tide.abilities
    );

    // Duneblast (`You` chooses up to 1 creature, destroy all other creatures).
    let duneblast = read(
        "Name:Duneblast\nManaCost:4 W B G\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | Amount$ 1 | Choices$ Creature | SubAbility$ DBDestroyAll\n\
         SVar:DBDestroyAll:DB$ DestroyAll | ValidCards$ Creature.nonChosenCard",
    );
    assert!(
        duneblast.abilities[0].contains("Effect::ChooseCardEx")
            && duneblast.abilities[0].contains("min: 0, max: Amount::Fixed(1)")
            && duneblast.abilities[0].contains("for_unchosen: &[Effect::RepeatEachObject"),
        "{:?}",
        duneblast.abilities
    );

    // Rags // Riches (`GainControl | AllValid$ Permanent.ChosenCard`).
    let riches = read(
        "Name:Riches\nManaCost:5 U U\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Opponent | Amount$ 1 | Choices$ Creature | ControlledByPlayer$ Chooser | Mandatory$ True | SubAbility$ StealChosen\n\
         SVar:StealChosen:DB$ GainControl | AllValid$ Permanent.ChosenCard",
    );
    assert!(
        riches.abilities[0].contains("Effect::RepeatEach { domain: RepeatDomain::EachOpponent")
            && riches.abilities[0].contains(
                "Effect::GainControlAll { filter: &Filter::Reference, to: PlayerRel::You }"
            ),
        "{:?}",
        riches.abilities
    );

    // Muse Vessel (`Hand -> Exile` with `ChooseMove::ExileLinked` + `ChooseCard` `ChoiceZone$ Exile` `LinkedExile` -> `ExileWithPermission`).
    let muse = read(
        "Name:Muse Vessel\nManaCost:4\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ 3 T | ValidTgts$ Player | SorcerySpeed$ True | Origin$ Hand | Destination$ Exile | ChangeType$ Card | ChangeNum$ 1 | Chooser$ Targeted | Hidden$ True | Mandatory$ True\n\
         A:AB$ ChooseCard | Mandatory$ True | Cost$ 1 | ChoiceZone$ Exile | Choices$ Card.ExiledWithSource | Amount$ 1 | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | Duration$ EndOfTurn | StaticAbilities$ STPlay | RememberObjects$ ChosenCard | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True\n\
         Oracle:{3}, {T}: Target player exiles a card from their hand. Activate only as a sorcery.\\n{1}: Choose a card exiled with Muse Vessel. You may play that card this turn.",
    );
    assert!(
        muse.abilities[0].contains("movement: ChooseMove::ExileLinked")
            && muse.abilities[1].contains("zone: ChooseZone::LinkedExile")
            && muse.abilities[1].contains("ExilePermission::play(PlayWindow::ThisTurn)"),
        "{:?}",
        muse.abilities
    );

    // Dauthi Voidwalker (`ChoiceZone$ Exile` `Card.OppOwn+counters_GE1_VOID` -> `ExileWithPermission` free).
    let voidwalker = read(
        "Name:Dauthi Voidwalker\nManaCost:B B\nTypes:Creature\nPT:3/2\nK:Shadow\n\
         R:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | ValidCard$ Card.!token+OppOwn | ReplaceWith$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard | WithCountersType$ VOID\n\
         A:AB$ ChooseCard | Cost$ T Sac<1/CARDNAME> | Defined$ You | Amount$ 1 | Mandatory$ True | Choices$ Card.OppOwn+counters_GE1_VOID | ChoiceZone$ Exile | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ MayPlay | RememberObjects$ ChosenCard | ForgetOnMoved$ Exile\n\
         SVar:MayPlay:Mode$ Continuous | MayPlay$ True | MayPlayWithoutManaCost$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile",
    );
    assert!(
        voidwalker.abilities[1].contains("zone: ChooseZone::Exile(PlayerRel::EachOpponent)")
            && voidwalker.abilities[1].contains("without_paying_mana_cost()"),
        "{:?}",
        voidwalker.abilities
    );

    // Graveyard owner atom-order bug fix (`Card.YouOwn+nonLand` where `YouOwn` is not the trailing atom).
    let gy_order = read(
        "Name:GyAtomOrder\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | Choices$ Card.YouOwn+nonLand | ChoiceZone$ Graveyard | Mandatory$ True | SubAbility$ DBHand\n\
         SVar:DBHand:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        gy_order.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::You)")
            && gy_order.abilities[0].contains("movement: ChooseMove::Hand"),
        "{:?}",
        gy_order.abilities
    );

    // Hand `Reveal$ True` sets `reveal_chosen: true`.
    let hand_rev = read(
        "Name:RevealHandExile\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | ChoiceZone$ Hand | Choices$ Creature.YouOwn | Reveal$ True | Mandatory$ True | SubAbility$ DBEx\n\
         SVar:DBEx:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Hand | Destination$ Exile",
    );
    assert!(
        hand_rev.abilities[0].contains("reveal_chosen: true"),
        "{:?}",
        hand_rev.abilities
    );

    // Honest refusals: Urza's Sylex wider DestroyAll, Alchemy Perpetual/MakeCard on Hand,
    // Hand->Graveyard discard bypass, and unlinked Delve/cost exile producers.
    for bad in [
        "Name:UrzasSylexRefusal\nManaCost:3\nTypes:Artifact\n\
         A:AB$ ChooseCard | Cost$ 2 T | Defined$ Player | ControlledByPlayer$ Chooser | Choices$ Land | Amount$ 6 | Mandatory$ True | SubAbility$ D\n\
         SVar:D:DB$ DestroyAll | ValidCards$ Permanent.!ChosenCard",
        "Name:AlchemyPerpetualHand\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ChoiceZone$ Hand | Choices$ Creature.YouOwn | Mandatory$ True | SubAbility$ P\n\
         SVar:P:DB$ Pump | PumpZone$ Hand | Defined$ ChosenCard | NumAtt$ +1 | Duration$ Perpetual",
        "Name:AlchemyConjureHand\nManaCost:U\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ChoiceZone$ Hand | Choices$ Creature.YouOwn | Mandatory$ True | SubAbility$ M\n\
         SVar:M:DB$ MakeCard | Conjure$ True | DefinedName$ ChosenCard | Zone$ Hand",
        "Name:DelveLinkedExileRefusal\nManaCost:4 U U\nTypes:Creature\nPT:3/3\nK:Delve\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Exile | Destination$ Hand | ChangeType$ Instant.ExiledWithSource | Hidden$ True | Mandatory$ True",
        "Name:OppScopeMismatchSweep\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Opponent | ControlledByPlayer$ Chooser | Choices$ Creature | Mandatory$ True | SubAbility$ D\n\
         SVar:D:DB$ DestroyAll | ValidCards$ Creature.!ChosenCard",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn wave25a_change_zone_hand_exile_library_and_owner_battlefield() {
    // Tidehollow Sculler (publicly revealed opponent hand -> ExileLinked + LTB ReturnLinkedExile).
    let sculler = read(
        "Name:Tidehollow Sculler\nManaCost:W B\nTypes:Artifact Creature Zombie\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigBounce\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ You | ChangeType$ Card.nonLand | ChangeNum$ 1 | IsCurse$ True\n\
         SVar:TrigBounce:DB$ ChangeZone | Origin$ Exile | Destination$ Hand | Defined$ ExiledWith\n\
         Oracle:When Tidehollow Sculler enters, target opponent reveals their hand and you choose a nonland card from it. Exile that card.\\nWhen Tidehollow Sculler leaves the battlefield, return the exiled card to its owner's hand.",
    );
    assert!(
        sculler.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && sculler.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::Chosen)")
            && sculler.abilities[0].contains("movement: ChooseMove::ExileLinked")
            && sculler.abilities[1].contains("Effect::ReturnLinkedExile { to: ZoneRef::Hand }"),
        "{:?}",
        sculler.abilities
    );

    // Thought-Knot Seer (preceded by RevealHand -> no duplicate RevealHand!).
    let tks = read(
        "Name:Thought-Knot Seer\nManaCost:3 C\nTypes:Creature\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigReveal\n\
         SVar:TrigReveal:DB$ RevealHand | ValidTgts$ Opponent | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Targeted | Chooser$ You | ChangeType$ Card.nonLand\n\
         Oracle:When Thought-Knot Seer enters, target opponent reveals their hand. You choose a nonland card from it and exile that card.",
    );
    assert_eq!(
        tks.abilities[0].matches("Effect::RevealHand").count(),
        1,
        "must not emit duplicate RevealHand: {:?}",
        tks.abilities
    );
    assert!(
        tks.abilities[0].contains("Effect::ChooseCardEx")
            && tks.abilities[0].contains("movement: ChooseMove::Exile"),
        "{:?}",
        tks.abilities
    );

    // Yarok's Fenlurker (EachOpponent exiles a card from their own hand).
    let fenlurker = read(
        "Name:Yarok's Fenlurker\nManaCost:B B\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Player.Opponent | Mandatory$ True | ChangeType$ Card | ChangeNum$ 1 | Hidden$ True | IsCurse$ True",
    );
    assert!(
        fenlurker.abilities[0].contains("Effect::RepeatEach { domain: RepeatDomain::EachOpponent")
            && fenlurker.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::EventPlayer)")
            && fenlurker.abilities[0].contains("movement: ChooseMove::Exile"),
        "{:?}",
        fenlurker.abilities
    );

    // Amass the Components (mandatory Hand -> LibraryPosition$ -1).
    let amass = read(
        "Name:Amass the Components\nManaCost:3 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 3 | SubAbility$ ChangeZoneDB\n\
         SVar:ChangeZoneDB:DB$ ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ -1 | Mandatory$ True",
    );
    assert!(
        amass.abilities[0].contains("Effect::draw(3)")
            && amass.abilities[0].contains("Effect::ChooseCardEx")
            && amass.abilities[0].contains("movement: ChooseMove::BottomOfLibrary"),
        "{:?}",
        amass.abilities
    );

    // Chittering Rats (`Origin$ Hand | Destination$ Library | LibraryPosition$ 0 | Chooser$ Targeted`).
    let rats = read(
        "Name:Chittering Rats\nManaCost:1 B B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ 0 | ValidTgts$ Opponent | Chooser$ Targeted | Mandatory$ True | ChangeType$ Card | ChangeNum$ 1 | IsCurse$ True",
    );
    assert!(
        rats.abilities[0]
            .contains("Effect::PutFromHandOnTopFor { player: PlayerRel::Chosen, count: 1 }"),
        "{:?}",
        rats.abilities
    );

    // Wrenn and Seven [0] ("any number of land cards from your hand onto the battlefield tapped").
    let wrenn = read(
        "Name:WrennZero\nManaCost:3 G G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Land | ChangeNum$ X | Tapped$ True | Hidden$ True\n\
         SVar:X:Count$ValidHand Land.YouCtrl\n\
         Oracle:Put any number of land cards from your hand onto the battlefield tapped.",
    );
    assert!(
        wrenn.abilities[0].contains("Effect::ChooseCardEx")
            && wrenn.abilities[0].contains("min: 0, max: Amount::Fixed(255)")
            && wrenn.abilities[0].contains("movement: ChooseMove::Battlefield { tapped: true }"),
        "{:?}",
        wrenn.abilities
    );

    // Braids, Conjurer Adept (non-controller Hand -> Battlefield under owner's control).
    let braids = read(
        "Name:Braids, Conjurer Adept\nManaCost:2 U U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | TriggerZones$ Battlefield | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature,Artifact,Land | DefinedPlayer$ TriggeredPlayer | ChangeNum$ 1\n\
         Oracle:At the beginning of each player's upkeep, that player may put an artifact, creature, or land card from their hand onto the battlefield.",
    );
    assert!(
        braids.abilities[0].contains(
            "chooser: PlayerRel::ActivePlayer, zone: ChooseZone::Hand(PlayerRel::ActivePlayer)"
        ) && braids.abilities[0].contains("owner_control: true"),
        "{:?}",
        braids.abilities
    );

    // Sisters of Stone Death: `Effect::exile_linked` producer + `ChooseZone::LinkedExile` consumer.
    let sisters = read(
        "Name:Sisters of Stone Death\nManaCost:4 B B G G\nTypes:Legendary Creature Gorgon\nPT:7/5\n\
         A:AB$ MustBlock | Cost$ G | ValidTgts$ Creature\n\
         A:AB$ ChangeZone | Cost$ B G | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature.blockingSource,Creature.blockedBySource\n\
         A:AB$ ChangeZone | Cost$ 2 B | Origin$ Exile | Destination$ Battlefield | ChangeType$ Creature.ExiledWithSource | Hidden$ True | Mandatory$ True | ChangeNum$ 1 | GainControl$ True\n\
         Oracle:{G}: Target creature blocks Sisters of Stone Death this turn if able.\\n{B}{G}: Exile target creature blocking or blocked by Sisters of Stone Death.\\n{2}{B}: Put a creature card exiled with Sisters of Stone Death onto the battlefield under your control.",
    );
    assert!(
        sisters.abilities[1].contains("Effect::exile_linked(")
            && sisters.abilities[2].contains("zone: ChooseZone::LinkedExile")
            && sisters.abilities[2].contains("movement: ChooseMove::Battlefield { tapped: false }"),
        "{:?}",
        sisters.abilities
    );

    // Honest refusals: Kitesail Freebooter (`Duration$ UntilHostLeavesPlay` from Hand),
    // Deep-Cavern Bat (private `"look at"` opponent's hand), Painful Memories (private `"look at"`
    // to top of library), Nautiloid Ship (unlinked `ExileGraveyard` producer), Show and Tell /
    // Exhume (multi-player simultaneous entry per CR 101.4), and bare `Card` / `Player.Other`.
    for bad in [
        "Name:KitesailRefusal\nManaCost:1 B\nTypes:Creature\nPT:1/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ You | ChangeType$ Card.nonLand | ChangeNum$ 1 | Duration$ UntilHostLeavesPlay\n\
         Oracle:When this enters, target opponent reveals their hand. Exile a nonland card until this leaves the battlefield.",
        "Name:DeepCavernBatRefusal\nManaCost:1 B\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ You | ChangeType$ Card.nonLand | ChangeNum$ 1\n\
         Oracle:When this enters, look at target opponent's hand. You may exile a nonland card from it.",
        "Name:PainfulMemoriesRefusal\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ 0 | ValidTgts$ Opponent | ChangeType$ Card | ChangeNum$ 1 | IsCurse$ True | Mandatory$ True\n\
         Oracle:Look at target opponent's hand and choose a card from it. Put that card on top of that player's library.",
        "Name:NautiloidShipRefusal\nManaCost:4\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExileGrave\n\
         SVar:TrigExileGrave:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Player | ChangeType$ Card\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | Execute$ TrigPut | CombatDamage$ True | ValidTarget$ Player | TriggerZones$ Battlefield | OptionalDecider$ You\n\
         SVar:TrigPut:DB$ ChangeZone | Origin$ Exile | Destination$ Battlefield | Hidden$ True | ChangeType$ Creature.ExiledWithSource | GainControl$ True\n\
         Oracle:When this enters, exile target player's graveyard.\\nWhenever this deals combat damage to a player, you may put a creature card exiled with this onto the battlefield under your control.",
        "Name:ShowAndTellRefusal\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature,Artifact,Enchantment,Land | DefinedPlayer$ Player | ChangeNum$ 1\n\
         Oracle:Each player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.",
        "Name:ExhumeRefusal\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Creature | DefinedPlayer$ Player | ChangeNum$ 1 | Hidden$ True | Mandatory$ True\n\
         Oracle:Each player puts a creature card from their graveyard onto the battlefield.",
        "Name:NonPermanentHandBfRefusal\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | DefinedPlayer$ Opponent | Origin$ Hand | Destination$ Battlefield | ChangeType$ Card | Optional$ True",
        "Name:PlayerOtherHandBfRefusal\nManaCost:3 W\nTypes:Creature\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Trig\n\
         SVar:Trig:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Artifact,Enchantment | DefinedPlayer$ Player.Other | ChangeNum$ 1\n\
         Oracle:When this enters, each other player may put an artifact or enchantment card onto the battlefield from their hand.",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn each_basic_type_lowers_to_one_scoped_keep_and_sacrifice_effect() {
    let card = read(
        "Name:BasicLandSweep\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Player | ControlledByPlayer$ Chooser | Choices$ Land | ChoiceZone$ Battlefield | Reveal$ True | EachBasicType$ True | SubAbility$ SacrificeRest\n\
         SVar:SacrificeRest:DB$ SacrificeAll | ValidCards$ Land.nonChosenCard",
    );
    let ability = &card.abilities[0];
    assert!(ability.contains("Effect::KeepOneOfEachAndSacrificeRestMatching"));
    assert!(ability.contains("rest: &Filter::LAND"));
    for subtype in ["PLAINS", "ISLAND", "SWAMP", "MOUNTAIN", "FOREST"] {
        assert!(
            ability.contains(&format!(
                "Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::{subtype})])"
            )),
            "missing {subtype}: {ability}"
        );
    }
    assert!(!ability.contains("Effect::RepeatEach"));
    assert!(!ability.contains("Effect::ChooseCardEx"));
    assert!(
        card.statics.is_empty(),
        "unused choice filter: {}",
        card.statics
    );
}
