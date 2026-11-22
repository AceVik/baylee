//! Wave 26e unit tests: `Dig.RememberChanged` (per-card `ExileWithPermission`,
//! multi-card `ChooseCard`/`Play` + bottom-rest, `PeekAndReveal.if_picked`),
//! `DigUntil` (`ExileWithPermission` on permanents + `MayPlayIgnoreColor`,
//! `Play` + non-found bottom-rest), `Mill.RememberMilled` (reflexive
//! `ImmediateTrigger`, omitted `NumCards$`, `Transform`), non-targeted /
//! self-default `ChangeZone` triggers, and `targets.rs` Wave 14a dynamic
//! amount filters & `ControlledBy`/`OwnedBy`/mixed target specs, plus strict
//! Paper-MTG and semantic-soundness refusals.

use super::*;

fn w26e_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Beast".into(),
            "Elemental".into(),
            "Elf".into(),
            "Goblin".into(),
            "Horror".into(),
            "Human".into(),
            "Insect".into(),
            "Kraken".into(),
            "Leviathan".into(),
            "Octopus".into(),
            "Pilot".into(),
            "Rat".into(),
            "Rogue".into(),
            "Scout".into(),
            "Serpent".into(),
            "Soldier".into(),
            "Thopter".into(),
            "Turtle".into(),
            "Warlock".into(),
            "Wizard".into(),
            "Wolf".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Food".into(),
            "Treasure".into(),
            "Vehicle".into(),
        ],
        land: vec![
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w26e(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &w26e_cats(), None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &w26e_cats(), None)
        )
    })
}

fn refused_w26e(text: &str) -> bool {
    transcode(&parse(text), &w26e_cats(), None).is_none()
}

// ---------------------------------------------------------------------------
// 1. Dig.RememberChanged triggers (C2+ExileWithPermission, C3, Pattern D/F)
// ---------------------------------------------------------------------------

#[test]
fn dig_remember_changed_exile_with_event_step_and_permission() {
    // Daxos of Meletis shape: exile top of damaged player's library, gain life
    // equal to its mana value, and cast it this turn with mana of any color.
    let daxos = read_w26e(
        "Name:Meletis Raider\nManaCost:1 W U\nTypes:Legendary Creature Human Soldier\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ Dig | Defined$ TriggeredTarget | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBEffect\n\
         SVar:X:Remembered$CardManaCost\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ STPlay | ForgetOnMoved$ Exile | RememberObjects$ Remembered | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreColor$ True | Affected$ Card.IsRemembered+nonLand | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let ab = &daxos.abilities[0];
    assert!(ab.contains("Effect::RevealUntilThen"), "{ab}");
    assert!(ab.contains("library: PlayerRel::DamagedPlayer"), "{ab}");
    assert!(
        ab.contains("Effect::GainLife { amount: Amount::EventManaValue }"),
        "{ab}"
    );
    assert!(
        ab.contains(
            "Effect::ExileWithPermission { target: TargetSpec::EventObject, until: None, \
             permission: ExilePermission::cast(PlayWindow::ThisTurn).spend_mana_as_any_color() }"
        ),
        "{ab}"
    );

    // Tavern Brawler-style upkeep trigger: exile top 1, pump self by +MV/+0
    // (omitting `Defined$ Self`), and grant play-this-turn permission.
    let brawler = read_w26e(
        "Name:Tavern Fighter\nManaCost:2 R\nTypes:Creature Human\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | TriggerZones$ Battlefield | ValidPlayer$ You | Execute$ PseudoDraw\n\
         SVar:PseudoDraw:DB$ Dig | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | NumAtt$ +X | SubAbility$ DBEffect\n\
         SVar:X:Remembered$CardManaCost\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ MayPlay | SubAbility$ DBCleanup | ExileOnMoved$ Exile\n\
         SVar:MayPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let bab = &brawler.abilities[0];
    assert!(
        bab.contains(
            "Effect::PumpTargetSlot { slot: TargetSlot::This, power: Amount::EventManaValue"
        ),
        "{bab}"
    );
    assert!(
        bab.contains(
            "Effect::ExileWithPermission { target: TargetSpec::EventObject, until: None, \
             permission: ExilePermission::play(PlayWindow::ThisTurn) }"
        ),
        "{bab}"
    );
}

#[test]
fn dig_remember_changed_multi_exile_choose_play_and_peek_if_picked() {
    // Feldon, Ronom Excavator shape: exile top X, choose one to play until the
    // end of your next turn (using `ChosenCard` + `ClearChosenCard$ True`).
    let feldon = read_w26e(
        "Name:Ronom Scout\nManaCost:1 R\nTypes:Creature Human\nPT:2/2\n\
         T:Mode$ DamageDoneOnce | ValidTarget$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | Choices$ Card.IsRemembered | ChoiceZone$ Exile | Mandatory$ True | ForgetOtherRemembered$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ ChosenCard | StaticAbilities$ STPlay | Duration$ UntilTheEndOfYourNextTurn | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.ChosenCard | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True\n\
         SVar:X:TriggerCount$DamageAmount",
    );
    let fab = &feldon.abilities[0];
    assert!(fab.contains("Effect::RevealUntilThen"), "{fab}");
    assert!(fab.contains("count: Amount::EventAmount"), "{fab}");
    assert!(fab.contains("Effect::ChooseCardEx"), "{fab}");
    assert!(fab.contains("zone: ChooseZone::Found"), "{fab}");
    assert!(
        fab.contains(
            "Effect::ExileWithPermission { target: TargetSpec::EventObject, until: None, \
             permission: ExilePermission::play(PlayWindow::ThroughYourNextTurn) }"
        ),
        "{fab}"
    );

    // Sanwell, Avenger Ace shape: exile top 6, may cast a Vehicle or artifact
    // creature spell from among them, rest to bottom in random order.
    let sanwell = read_w26e(
        "Name:Avenger Pilot\nManaCost:1 W\nTypes:Legendary Creature Human Pilot\nPT:3/1\n\
         T:Mode$ Taps | ValidCard$ Card.Self | Execute$ DBExileTopCard\n\
         SVar:DBExileTopCard:DB$ Dig | Defined$ You | DigNum$ 6 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Valid$ Vehicle.IsRemembered,Artifact.Creature+IsRemembered | ValidSA$ Spell | ValidZone$ Exile | Controller$ You | Optional$ True | Amount$ 1 | SubAbility$ PutRestBackBottomLibrary\n\
         SVar:PutRestBackBottomLibrary:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let sab = &sanwell.abilities[0];
    assert!(sab.contains("Effect::RevealUntilThen"), "{sab}");
    assert!(sab.contains("count: Amount::Fixed(6)"), "{sab}");
    assert!(
        sab.contains("Effect::Play { source: PlaySource::Subject(TargetSpec::EventObject)"),
        "{sab}"
    );
    assert!(
        sab.contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{sab}"
    );

    // Chrome Courier shape: reveal top 2, pick 1 into hand, other into yard,
    // if artifact was put into hand gain 3 life -> `PeekAndReveal.if_picked`.
    let courier = read_w26e(
        "Name:Chrome Delivery\nManaCost:1 W U\nTypes:Artifact Creature Thopter\nPT:1/1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Destination$ Battlefield | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 2 | ChangeNum$ 1 | Reveal$ True | ChangeType$ Card | DestinationZone2$ Graveyard | RememberChanged$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 3 | ConditionDefined$ Remembered | ConditionPresent$ Artifact | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let cab = &courier.abilities[0];
    assert!(cab.contains("Effect::PeekAndReveal"), "{cab}");
    assert!(cab.contains("reveal_all: true"), "{cab}");
    assert!(cab.contains("rest: PeekRestDest::Graveyard"), "{cab}");
    assert!(cab.contains("Effect::IfEventObjectMatches"), "{cab}");
    assert!(cab.contains("Effect::gain_life(3)"), "{cab}");
}

// ---------------------------------------------------------------------------
// 2. DigUntil unlocks (permanent ExileWithPermission, Play + bottom-rest)
// ---------------------------------------------------------------------------

#[test]
fn dig_until_permanent_exile_permission_and_play_rest_to_bottom() {
    // Exploratory Mission shape: permanent landfall trigger with DigUntil ->
    // `DB$ Effect` (`ThisTurn` on a permanent!) -> non-remembered `DB$ Mana` -> `Cleanup`.
    let mission = read_w26e(
        "Name:Frontier Survey\nManaCost:2 R R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Land.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ QUEST | CounterNum$ 1 | SubAbility$ DBDigUntil\n\
         SVar:DBDigUntil:DB$ DigUntil | Valid$ Card.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ STPlay | ExileOnMoved$ Exile | RememberObjects$ Remembered | SubAbility$ DBMana\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBMana:DB$ Mana | Produced$ R | Amount$ 2 | ConditionDefined$ Self | ConditionPresent$ Card.counters_GE5_QUEST | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let mab = &mission.abilities[0];
    assert!(mab.contains("Effect::RevealUntilThen"), "{mab}");
    assert!(mab.contains("found: RevealUntilFound::Exile,"), "{mab}");
    assert!(
        mab.contains(
            "Effect::ExileWithPermission { target: TargetSpec::EventObject, until: None, \
             permission: ExilePermission::play(PlayWindow::ThisTurn) }"
        ),
        "{mab}"
    );
    assert!(mab.contains("Effect::IfCondition"), "{mab}");

    // Single-hit `DigUntil | ImprintFound$ True | RememberRevealed$ True` ->
    // `DB$ Play` -> `ChangeZoneAll` moving non-found revealed cards to library bottom.
    let cascade_like = read_w26e(
        "Name:Arcane Cascade\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Card.nonLand+cmcLE3 | FoundDestination$ Exile | RevealedDestination$ Exile | ImprintFound$ True | RememberRevealed$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Imprinted | ValidSA$ Spell | WithoutManaCost$ True | Optional$ True | SubAbility$ DBBottom\n\
         SVar:DBBottom:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | ChangeType$ Card.IsRemembered+!IsImprinted | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    let cab = &cascade_like.abilities[0];
    assert!(cab.contains("Effect::RevealUntilThen"), "{cab}");
    assert!(cab.contains("rest: RevealUntilRest::BottomRandom"), "{cab}");
    assert!(cab.contains("PlayCostMode::WithoutPayingManaCost"), "{cab}");
}

// ---------------------------------------------------------------------------
// 3. Mill.RememberMilled triggers (M1 reflexive ImmediateTrigger, M2 ThisWay)
// ---------------------------------------------------------------------------

#[test]
fn mill_remember_milled_reflexive_and_conditional_triggers() {
    // Primal Witchstalker / Yathan Roadwatcher shape: mill 4, when you do,
    // return target land card from your graveyard to the battlefield tapped.
    let stalker = read_w26e(
        "Name:Graveyard Stalker\nManaCost:1 B G\nTypes:Creature Wolf\nPT:2/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 4 | Defined$ You | RememberMilled$ True | SubAbility$ DBTrigger\n\
         SVar:DBTrigger:DB$ ImmediateTrigger | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ4 | Execute$ ReturnLand | SubAbility$ DBCleanup\n\
         SVar:ReturnLand:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Land.YouOwn | Tapped$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let sab = &stalker.abilities[0];
    assert!(
        sab.contains("Effect::Mill { amount: Amount::Fixed(4), target: PlayerRel::You }"),
        "{sab}"
    );
    assert!(
        sab.contains("Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Milled(&Filter::Any)), 4)"),
        "{sab}"
    );
    assert!(sab.contains("Effect::ReflexiveTrigger"), "{sab}");
    assert!(sab.contains("TargetSpec::CardInGraveyard(&"), "{sab}");

    // Sigil of Myrkul shape: omitted `NumCards$` (defaults to 1) + reflexive
    // trigger with intervening-if `>= 4 creature cards in your graveyard`.
    let sigil = read_w26e(
        "Name:Death Sigil\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | RememberMilled$ True | SubAbility$ DBImmediateTrigger\n\
         SVar:DBImmediateTrigger:DB$ ImmediateTrigger | ConditionPresent$ Creature.YouOwn | ConditionZone$ Graveyard | ConditionCompare$ GE4 | TriggerAmount$ Remembered$Amount | Execute$ TrigPutCounter | SubAbility$ DBCleanup\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature.YouCtrl | CounterType$ P1P1 | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Targeted | KW$ Deathtouch\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    let sig = &sigil.abilities[0];
    assert!(
        sig.contains("Effect::Mill { amount: Amount::Fixed(1), target: PlayerRel::You }"),
        "{sig}"
    );
    assert!(
        sig.contains("when: ReflexiveWhen::ThisWay(ThisWay::Milled(&Filter::Any))"),
        "{sig}"
    );
    assert!(sig.contains("ZoneSel::GraveyardYou"), "{sig}");

    // Aberrant Researcher shape: omitted `NumCards$` (1) -> `SetState | Mode$ Transform`
    // if an instant or sorcery card was milled this way.
    let researcher = read_w26e(
        "Name:Curious Scholar\nManaCost:3 U\nTypes:Creature Human Insect\nPT:3/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | Defined$ You | RememberMilled$ True | SubAbility$ DBTransform\n\
         SVar:DBTransform:DB$ SetState | Defined$ Self | ConditionDefined$ Remembered | ConditionPresent$ Card.Instant,Card.Sorcery | SubAbility$ DBCleanup | Mode$ Transform\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         AlternateMode:DoubleFaced\nALTERNATE\n\
         Name:Transformed Form\nColors:blue\nTypes:Creature Insect Horror\nPT:5/4",
    );
    let rab = &researcher.abilities[0];
    assert!(
        rab.contains("Effect::Mill { amount: Amount::Fixed(1), target: PlayerRel::You }"),
        "{rab}"
    );
    assert!(rab.contains("ThisWay::Milled("), "{rab}");
    assert!(rab.contains("Effect::TransformSource"), "{rab}");
}

// ---------------------------------------------------------------------------
// 4. ChangeZone without target / Defined$ triggers (CZ1 / CZ2)
// ---------------------------------------------------------------------------

#[test]
fn change_zone_without_target_or_defined_triggers() {
    // Quagnoth shape: self-return omitting `Defined$ Self` from `Graveyard,Exile`.
    let quagnoth = read_w26e(
        "Name:Shroud Beast\nManaCost:5 G\nTypes:Creature Beast\nPT:4/5\n\
         T:Mode$ Discarded | ValidCard$ Card.Self | ValidCause$ SpellAbility.OppCtrl | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand",
    );
    assert!(
        quagnoth.abilities[0].contains("Effect::bounce(TargetSpec::ThisObject)"),
        "{:?}",
        quagnoth.abilities
    );

    // Widespread Panic shape: `DefinedPlayer$ TriggeredPlayer | Chooser$ TriggeredPlayer`
    // moving a card from hand to top of library.
    let panic = read_w26e(
        "Name:Shuffle Panic\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ SearchedLibrary | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | DefinedPlayer$ TriggeredPlayer | Chooser$ TriggeredPlayer | ChangeType$ Card | ChangeNum$ 1 | Origin$ Hand | Destination$ Library | LibraryPosition$ 0 | Mandatory$ True",
    );
    let pab = &panic.abilities[0];
    assert!(
        pab.contains("Effect::PutFromHandOnTopFor { player: PlayerRel::EventPlayer, count: 1 }"),
        "{pab}"
    );

    // Errand-Rider of Gondor shape: conditional hand -> bottom of library without `ChangeType$`.
    let rider = read_w26e(
        "Name:Gondor Messenger\nManaCost:2 W\nTypes:Creature Human Soldier\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Hand | Destination$ Library | ChangeNum$ 1 | Mandatory$ True | LibraryPosition$ -1 | ConditionPresent$ Creature.Legendary+YouCtrl | ConditionCompare$ EQ0",
    );
    let rab = &rider.abilities[0];
    assert!(rab.contains("Effect::IfCondition"), "{rab}");
    assert!(
        rab.contains("movement: ChooseMove::BottomOfLibrary"),
        "{rab}"
    );

    // Quest for Ula's Temple shape: Hand -> Battlefield with `Hidden$ True`.
    let temple = read_w26e(
        "Name:Sea Temple\nManaCost:U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.Kraken,Creature.Leviathan,Creature.Octopus,Creature.Serpent | ChangeNum$ 1 | Hidden$ True\n\
         Oracle:At the beginning of each end step, you may put a Kraken, Leviathan, Octopus, or Serpent creature card from your hand onto the battlefield.",
    );
    assert!(
        temple.abilities[0].contains("Effect::PutFromHandOntoBattlefield"),
        "{:?}",
        temple.abilities
    );

    // Yarok's Fenlurker shape: each opponent exiles a card from their hand.
    let fenlurker = read_w26e(
        "Name:Fen Horror\nManaCost:B B\nTypes:Creature Horror\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Player.Opponent | Mandatory$ True | ChangeType$ Card | ChangeNum$ 1 | Hidden$ True | IsCurse$ True",
    );
    let fab = &fenlurker.abilities[0];
    assert!(
        fab.contains("Effect::RepeatEach { domain: RepeatDomain::EachOpponent"),
        "{fab}"
    );
    assert!(fab.contains("movement: ChooseMove::Exile"), "{fab}");

    // Multi-player graveyard choice with `Chooser$ You` moving to library bottom.
    let gy_bottom = read_w26e(
        "Name:Graveyard Tuck\nManaCost:1 U B\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | DefinedPlayer$ Player | Chooser$ You | Origin$ Graveyard | Destination$ Library | LibraryPosition$ -1 | Optional$ True | ChangeType$ Creature,Planeswalker",
    );
    let nab = &gy_bottom.abilities[0];
    assert!(
        nab.contains("Effect::RepeatEach { domain: RepeatDomain::EachPlayer"),
        "{nab}"
    );
    assert!(nab.contains("chooser: PlayerRel::You"), "{nab}");
    assert!(nab.contains("min: 0,"), "{nab}");
    assert!(
        nab.contains("zone: ChooseZone::Graveyard(PlayerRel::EventPlayer)"),
        "{nab}"
    );
    assert!(
        nab.contains("movement: ChooseMove::BottomOfLibrary"),
        "{nab}"
    );
}

// ---------------------------------------------------------------------------
// 5. targets.rs: Wave 14a dynamic Amount filters & ControlledBy/OwnedBy/Targets
// ---------------------------------------------------------------------------

#[test]
fn targets_dynamic_amount_filters_and_controlled_owned_specs() {
    // Mausoleum Secrets: `ChangeType$ Card.Black+cmcLEX` with graveyard creature count.
    let secrets = read_w26e(
        "Name:Crypt Secrets\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Card.Black+cmcLEX | ChangeNum$ 1\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn",
    );
    let s_all = format!("{}\n{}", secrets.statics, secrets.abilities.join("\n"));
    assert!(
        s_all.contains("Filter::CmcAtMostAmount(&Amount::CountOf"),
        "{s_all}"
    );
    assert!(
        s_all.contains("ZoneSel::GraveyardOf(PlayerRel::You)")
            || s_all.contains("ZoneSel::GraveyardYou"),
        "{s_all}"
    );

    // Beseech the Queen: `ChangeType$ Card.cmcLEX` with `Count$Valid Land.YouCtrl`.
    let beseech = read_w26e(
        "Name:Queen's Tutor\nManaCost:2B 2B 2B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Card.cmcLEX | ChangeNum$ 1\n\
         SVar:X:Count$Valid Land.YouCtrl",
    );
    let b_all = format!("{}\n{}", beseech.statics, beseech.abilities.join("\n"));
    assert!(
        b_all.contains("Filter::CmcAtMostAmount(&Amount::CountOf")
            && b_all.contains("ZoneSel::Battlefield")
            && (b_all.contains("Filter::YOUR_LAND") || b_all.contains("Filter::LAND")),
        "{b_all}"
    );

    // Alesha / graveyard reanimation with `cmcLEX` against `Count$CardPower` and `Count$Domain`.
    let bortuk = read_w26e(
        "Name:Domain Reclaimer\nManaCost:4 B G\nTypes:Creature Shaman\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigRet\n\
         SVar:TrigRet:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Permanent.YouOwn+cmcLEX\n\
         SVar:X:Count$Domain",
    );
    let bo_all = format!("{}\n{}", bortuk.statics, bortuk.abilities.join("\n"));
    assert!(
        bo_all
            .contains("Filter::CmcAtMostAmount(&Amount::BasicLandTypesAmong(&Filter::YOUR_LAND))"),
        "{bo_all}"
    );

    // `powerLE<Var>` with dynamic board count (`Filter::PowerAtMostAmount`).
    let pow_le = read_w26e(
        "Name:Land Bound\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Creature.powerLEX\n\
         SVar:X:Count$Valid Plains.YouCtrl",
    );
    let p_all = format!("{}\n{}", pow_le.statics, pow_le.abilities.join("\n"));
    assert!(
        p_all.contains("Filter::PowerAtMostAmount(&Amount::CountOf"),
        "{p_all}"
    );

    // `ValidTgts$ Player.Opponent,Card.Planeswalker` -> `OpponentOrObject(&Filter::PLANESWALKER)`.
    let opp_pw = read_w26e(
        "Name:Spark Strike\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Player.Opponent,Card.Planeswalker | NumDmg$ 3",
    );
    assert!(
        opp_pw.abilities[0].contains("TargetSpec::OpponentOrObject(&Filter::PLANESWALKER)"),
        "{:?}",
        opp_pw.abilities
    );

    // `ControlledBy TriggeredActivator` on `SpellCast` trigger -> `ObjectOfEventPlayer`.
    let spell_ctrl = read_w26e(
        "Name:Cast Punisher\nManaCost:2 W\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigDestroy\n\
         SVar:TrigDestroy:DB$ Destroy | ValidTgts$ Creature.ControlledBy TriggeredActivator",
    );
    assert!(
        spell_ctrl.abilities[0].contains("TargetSpec::ObjectOfEventPlayer(&"),
        "{:?}",
        spell_ctrl.abilities
    );

    // `ControlledBy TargetedOrController` in `DamageAll` (Chandra Nalaar / Heart of Bogardan).
    let chandra = read_w26e(
        "Name:Flame Sweep\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 5 | SubAbility$ DmgAll\n\
         SVar:DmgAll:DB$ DamageAll | NumDmg$ 5 | ValidCards$ Creature.ControlledBy TargetedOrController",
    );
    let ch_all = format!("{}\n{}", chandra.statics, chandra.abilities.join("\n"));
    assert!(
        ch_all.contains("Filter::ControlledBy(PlayerRel::TargetPlayerOrController)"),
        "{ch_all}"
    );

    // `OwnedBy TriggeredActivator` in graveyard target on `SpellCast` trigger.
    let gy_act = read_w26e(
        "Name:Grave Eater\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigEx\n\
         SVar:TrigEx:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card.OwnedBy TriggeredActivator",
    );
    assert!(
        gy_act.abilities[0]
            .contains("TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EventPlayer)"),
        "{:?}",
        gy_act.abilities
    );
}

// ---------------------------------------------------------------------------
// 6. Strict Paper-MTG & semantic-soundness refusals
// ---------------------------------------------------------------------------

#[test]
fn wave26e_strict_refusals_alchemy_unset_planechase_and_soundness() {
    // Pure Paper-MTG ONLY: Alchemy, Un-Set/Acorn/Sticker, and Planechase MUST refuse.
    for bad in [
        "Name:Alchemy Dig Conjure\nManaCost:1 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 3 | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBConjure\n\
         SVar:DBConjure:DB$ MakeCard | Name$ Lightning Bolt | Zone$ Hand",
        "Name:Alchemy Seek\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Seek | Type$ Creature",
        "Name:Alchemy Perpetual\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -1 | NumDef$ -1 | Perpetual$ True",
        "Name:Unset Sticker\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
         K:StickerSheet",
        "Name:Planechase Trigger\nTypes:Plane\n\
         T:Mode$ PlaneswalkedTo | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
        // Cyclic SVar inside dynamic amount filter MUST refuse without stack overflow.
        "Name:Cyclic Filter\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Creature.cmcLEX | ChangeNum$ 1\n\
         SVar:X:Count$Valid Creature.cmcLEX",
        // `ControlledBy TargetedController` as a first cast target MUST stay refused.
        "Name:Bad First Target\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature.ControlledBy TargetedController",
        // `DB$ ChangeZone` without `Mandatory$`, `Optional$`, or optional Oracle text MUST refuse.
        "Name:Bad Ambiguous Hand Move\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigMove\n\
         SVar:TrigMove:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | Hidden$ True | ChangeType$ Card",
        // `DB$ ChangeZone` on Battlefield without `ChangeType$` and non-self destination MUST refuse.
        "Name:Bad Bare Battlefield Move\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigMove\n\
         SVar:TrigMove:DB$ ChangeZone | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1",
        // `DB$ ChangeZone` on Battlefield with unscoped `ChangeType$ Creature` (missing `YouCtrl`) MUST refuse.
        "Name:Bad Unscoped Bounce\nManaCost:2 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigBounce\n\
         SVar:TrigBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Mandatory$ True | ChangeType$ Creature",
        // `DB$ Mill | RememberMilled$ True | SubAbility$ DBImmediateTrigger` with mismatched EQ count MUST refuse.
        "Name:Bad Mismatched Mill Reflexive\nManaCost:2 B\nTypes:Creature Wolf\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 3 | Defined$ You | RememberMilled$ True | SubAbility$ DBTrigger\n\
         SVar:DBTrigger:DB$ ImmediateTrigger | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ4 | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    ] {
        assert!(refused_w26e(bad), "must refuse:\n{bad}");
    }
}
