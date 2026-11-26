//! Wave 18b `scriptgen` tests:
//! 1. `Waterbend` costs (`Waterbend<N>`, `Waterbend<X>`), `Ward:Waterbend<N>`,
//!    and `UnlessCost$ Waterbend<...>` (*Aang, at the Crossroads* / Avatar cards).
//! 2. `keyword Assist` (*Play of the Game* / Battlebond cards).
//! 3. `GenericChoice` patterns:
//!    - ETB `SetChosenMode$ True` (*Outpost Siege* / Sieges) & `EachPlayerChooseMode`
//!      (*Archangel of Strife*).
//!    - `PayUnlessCost` multi-cost unless (*Torment of Venom*, *Zoyowa Lava-Tongue*).
//!    - Per-player modal & special chains (*Prisoner's Dilemma*, *Master of Ceremonies*,
//!      *Borderland Explorer*, *Imposing Grandeur*, *Wheel of Potential*).
//! 4. `Bookkeeping` patterns (*Day of Black Sun*, *Aetherspouts*, *Corrosive Ooze*,
//!    *Creeping Dread*, *Faerie Artisans*, *Fiendlash*, *Flickerform*, *Gaze of the Gorgon*,
//!    *Sovereigns of Lost Alara*, *Triton Tactics*, *Unforge*, *Eye of Singularity*,
//!    *Phyrexian Grimoire*, *Tainted Pact*).

#![allow(clippy::too_many_lines)]

use super::*;

fn w18b_cats() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Angel".into(),
            "Artificer".into(),
            "Bird".into(),
            "Citizen".into(),
            "Cleric".into(),
            "Dragon".into(),
            "Elf".into(),
            "Faerie".into(),
            "Goblin".into(),
            "Human".into(),
            "Illusion".into(),
            "Knight".into(),
            "Merfolk".into(),
            "Ooze".into(),
            "Rogue".into(),
            "Scout".into(),
            "Servo".into(),
            "Soldier".into(),
            "Spirit".into(),
            "Warlock".into(),
            "Warrior".into(),
            "Wizard".into(),
        ],
        artifact: vec![
            "Book".into(),
            "Clue".into(),
            "Equipment".into(),
            "Treasure".into(),
        ],
        enchantment: vec!["Aura".into()],
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

fn w18b_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "c_1_1_a_servo",
            "Name:Servo Token\nTypes:Artifact Creature Servo\nPT:1/1",
        ),
        (
            "g_1_1_citizen_all",
            "Name:Citizen Token\nTypes:Creature Citizen\nColors:green,white\nPT:1/1",
        ),
        (
            "c_a_treasure",
            "Name:Treasure Token\nTypes:Artifact Treasure\nA:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any",
        ),
        (
            "w_1_1_human_soldier",
            "Name:Human Soldier Token\nTypes:Creature Human Soldier\nColors:white\nPT:1/1",
        ),
        (
            "r_3_1_dragon",
            "Name:Dragon Token\nTypes:Creature Dragon\nColors:red\nPT:3/1\nK:Flying",
        ),
    ])
}

fn read_w18b(text: &str) -> CardBody {
    let parsed = parse(text);
    let cats = w18b_cats();
    let toks = w18b_tokens();
    transcode(&parsed, &cats, Some(&toks)).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats, Some(&toks))
        )
    })
}

fn refused_w18b(text: &str) -> bool {
    let cats = w18b_cats();
    let toks = w18b_tokens();
    transcode(&parse(text), &cats, Some(&toks)).is_none()
}

#[test]
fn wave18b_assist_and_waterbend() {
    let assist = read_w18b(
        "Name:Play of the Game\nManaCost:6 W W\nTypes:Sorcery\n\
         K:Assist\n\
         A:SP$ ChangeZoneAll | ChangeType$ Permanent.nonLand | Origin$ Battlefield | Destination$ Exile",
    );
    assert!(
        assist.keywords.iter().any(|k| k == "KeywordSet::ASSIST"),
        "expected Assist keyword: {:?}",
        assist.keywords
    );

    let wb_spell = read_w18b(
        "Name:Waterbend Spell\nManaCost:U\nTypes:Instant\n\
         A:SP$ Draw | Cost$ Waterbend<3> | NumCards$ 2",
    );
    assert!(
        wb_spell
            .mandatory_additional_costs
            .iter()
            .any(|c| c.contains("Waterbend(3)")),
        "expected Waterbend(3): {:?}",
        wb_spell.mandatory_additional_costs
    );

    let wb_x = read_w18b(
        "Name:Waterbend X\nManaCost:U\nTypes:Creature Wizard\nPT:1/2\n\
         A:AB$ Draw | Cost$ Waterbend<X> | NumCards$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        wb_x.abilities[0].contains("WaterbendX"),
        "expected WaterbendX: {:?}",
        wb_x.abilities
    );

    let wb_ward = read_w18b(
        "Name:Waterbend Warder\nManaCost:2 U\nTypes:Creature Merfolk\nPT:2/3\n\
         K:Ward:Waterbend<2>",
    );
    assert!(
        wb_ward
            .abilities
            .iter()
            .any(|a| a == "AbilityDef::WardWaterbend { mana: 2 }"),
        "expected WardWaterbend: {:?}",
        wb_ward.abilities
    );

    let wb_unless = read_w18b(
        "Name:Waterbend Tax\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ Waterbend<3>",
    );
    assert!(
        wb_unless.abilities[0].contains("PlayerMayWaterbendOr"),
        "expected unless waterbend: {:?}",
        wb_unless.abilities
    );

    assert!(refused_w18b(
        "Name:Bad Waterbend\nManaCost:U\nTypes:Instant\nA:SP$ Draw | Cost$ Waterbend<0> | NumCards$ 1"
    ));
    assert!(refused_w18b(
        "Name:Bad Ward\nManaCost:U\nTypes:Creature Merfolk\nPT:1/1\nK:Ward:Waterbend<0>"
    ));
}

#[test]
fn wave18b_generic_choice_etb_and_modal() {
    let siege = read_w18b(
        "Name:Outpost Siege\nManaCost:3 R\nTypes:Enchantment\n\
         K:ETBReplacement:Other:SiegeChoice\n\
         SVar:SiegeChoice:DB$ GenericChoice | Choices$ Khans,Dragons | Defined$ You | SetChosenMode$ True | AILogic$ Khans | ShowChoice$ ExceptSelf\n\
         SVar:Khans:DB$ Nothing | SpellDescription$ Khans\n\
         SVar:Dragons:DB$ Nothing | SpellDescription$ Dragons\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.ChosenModeKhans | AddTrigger$ KhansTrigger\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Affected$ Card.ChosenModeDragons | AddTrigger$ DragonsTrigger\n\
         SVar:KhansTrigger:Mode$ Phase | Phase$ Upkeep | TriggerZones$ Battlefield | ValidPlayer$ You | Execute$ PseudoDraw | Secondary$ True\n\
         SVar:PseudoDraw:DB$ Dig | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ Play | SubAbility$ DBCleanup | ExileOnMoved$ Exile\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:DragonsTrigger:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ SmallBurnination | Secondary$ True\n\
         SVar:SmallBurnination:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1",
    );
    assert!(
        siege
            .enter_modifiers
            .iter()
            .any(|m| m.contains("EnterModifier::ChooseMode")),
        "expected ChooseMode: {:?}",
        siege.enter_modifiers
    );

    let archangel = read_w18b(
        "Name:Archangel of Strife\nManaCost:5 W W\nTypes:Creature Angel\nPT:6/6\n\
         K:Flying\n\
         K:ETBReplacement:Other:ChooseMode\n\
         SVar:ChooseMode:DB$ GenericChoice | Defined$ Player | TempRemember$ Chooser | Choices$ War,Peace\n\
         SVar:War:DB$ Bookkeeping | Defined$ Remembered | NoteCards$ Self | NoteCardsFor$ War | SpellDescription$ War\n\
         SVar:Peace:DB$ Bookkeeping | Defined$ Remembered | NoteCards$ Self | NoteCardsFor$ Peace | SpellDescription$ Peace\n\
         S:Mode$ Continuous | Affected$ Creature.ControlledBy Player.NotedForWar | AddPower$ 3 | Description$ War\n\
         S:Mode$ Continuous | Affected$ Creature.ControlledBy Player.NotedForPeace | AddToughness$ 3 | Description$ Peace",
    );
    assert!(
        archangel
            .enter_modifiers
            .iter()
            .any(|m| m.contains("EnterModifier::EachPlayerChooseMode")),
        "expected EachPlayerChooseMode: {:?}",
        archangel.enter_modifiers
    );

    let prisoners = read_w18b(
        "Name:Prisoner's Dilemma\nManaCost:3 R R\nTypes:Sorcery\n\
         A:SP$ GenericChoice | Defined$ Opponent | TempRemember$ Chooser | Secretly$ True | Choices$ Silence,Snitch | SubAbility$ DBSilenceBranch | StackDescription$ SpellDescription\n\
         SVar:Silence:DB$ Bookkeeping | Defined$ Remembered | NoteCards$ Self | NoteCardsFor$ Silence | SpellDescription$ Silence\n\
         SVar:Snitch:DB$ Bookkeeping | Defined$ Remembered | NoteCards$ Self | NoteCardsFor$ Snitch | SpellDescription$ Snitch\n\
         SVar:DBSilenceBranch:DB$ Branch | BranchConditionSVar$ PlayerCountOpponents$HasPropertyNotedForSilence | BranchConditionSVarCompare$ EQPlayerCountOpponents$Amount | TrueSubAbility$ Damage4 | FalseSubAbility$ DBSnitchBranch\n\
         SVar:DBSnitchBranch:DB$ Branch | BranchConditionSVar$ PlayerCountOpponents$HasPropertyNotedForSnitch | BranchConditionSVarCompare$ EQPlayerCountOpponents$Amount | TrueSubAbility$ Damage8 | FalseSubAbility$ Damage12\n\
         SVar:Damage4:DB$ DealDamage | Defined$ Opponent | NumDmg$ 4 | SubAbility$ DBCleanup\n\
         SVar:Damage8:DB$ DealDamage | Defined$ Opponent | NumDmg$ 8 | SubAbility$ DBCleanup\n\
         SVar:Damage12:DB$ DealDamage | Defined$ Opponent.NotedForSilence | NumDmg$ 12 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Bookkeeping | Defined$ Player | ClearNotedCardsFor$ Silence,Snitch\n\
         K:Flashback:5 R R",
    );
    assert!(
        prisoners.abilities[0].contains("MultiPlayerChoiceAggregate::PrisonersDilemma"),
        "expected PrisonersDilemma: {:?}",
        prisoners.abilities
    );

    let torment_of_venom = read_w18b(
        "Name:Torment of Venom\nManaCost:2 B B\nTypes:Instant\n\
         A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ M1M1 | CounterNum$ 3 | IsCurse$ True | RememberTargets$ True | SubAbility$ DBGenericChoice\n\
         SVar:DBGenericChoice:DB$ GenericChoice | Choices$ PaySac,PayDiscard | Defined$ TargetedController | FallbackAbility$ LoseLifeFallback | AILogic$ PayUnlessCost | SubAbility$ DBCleanup\n\
         SVar:PaySac:DB$ LoseLife | LifeAmount$ 3 | Defined$ TargetedController | UnlessCost$ Sac<1/Permanent.!IsRemembered+nonLand/another nonland permanent> | UnlessPayer$ TargetedController\n\
         SVar:PayDiscard:DB$ LoseLife | LifeAmount$ 3 | Defined$ TargetedController | UnlessCost$ Discard<1/Card> | UnlessPayer$ TargetedController\n\
         SVar:LoseLifeFallback:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ 3\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        torment_of_venom.abilities[0].contains("Effect::PlayerMayPayOneOfCostsOr"),
        "expected Torment of Venom: {:?}",
        torment_of_venom.abilities
    );

    assert!(refused_w18b(
        "Name:Bad Siege\nManaCost:2 R\nTypes:Enchantment\n\
         K:ETBReplacement:Other:ChooseMode\n\
         SVar:ChooseMode:DB$ GenericChoice | Defined$ You | SetChosenMode$ True | Choices$ OnlyOne\n\
         SVar:OnlyOne:DB$ Nothing | SpellDescription$ Only"
    ));
}

#[test]
fn wave18b_zoyowa_pay_unless_choices() {
    let zoyowa = read_w18b(
        "Name:Zoyowa Lava-Tongue\nManaCost:B R\nTypes:Legendary Creature Goblin Warlock\nPT:2/2\n\
         K:Deathtouch\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You.descended | TriggerZones$ Battlefield | Execute$ TrigMaySacorDiscard | TriggerDescription$ At the beginning of your end step, if you descended this turn, each opponent may discard a card or sacrifice a permanent. CARDNAME deals 3 damage to each opponent who didn't. (You descended if a permanent card was put into your graveyard from anywhere.)\n\
         SVar:TrigMaySacorDiscard:DB$ GenericChoice | Choices$ PaySac,PayDiscard | TempRemember$ Chooser | Defined$ Opponent | AILogic$ PayUnlessCost | DamageMap$ True\n\
         SVar:PaySac:DB$ DealDamage | NumDmg$ 3 | Defined$ Remembered | UnlessCost$ Sac<1/Permanent> | UnlessPayer$ Remembered | SpellDescription$ CARDNAME deals 3 damage to you unless you sacrifice a permanent.\n\
         SVar:PayDiscard:DB$ DealDamage | NumDmg$ 3 | Defined$ Remembered | UnlessCost$ Discard<1/Card> | UnlessPayer$ Remembered | SpellDescription$ CARDNAME deals 3 damage to you unless you discard a card.",
    );
    assert!(
        zoyowa.abilities[0].contains("Effect::PlayerMayPayOneOfCostsOr"),
        "expected Zoyowa PlayerMayPayOneOfCostsOr: {:?}",
        zoyowa.abilities
    );
}

#[test]
fn wave18b_bookkeeping_cards_read() {
    let day_of_black_sun = read_w18b(
        "Name:Day of Black Sun\nManaCost:X B B\nTypes:Sorcery\n\
         A:SP$ Bookkeeping | RememberAll$ Creature.cmcLEX | StackDescription$ None | SubAbility$ DBAnimateAll\n\
         SVar:DBAnimateAll:DB$ AnimateAll | ValidCards$ Card.IsRemembered | RemoveAllAbilities$ True | SubAbility$ DBDestroyAll\n\
         SVar:DBDestroyAll:DB$ DestroyAll | ValidCards$ Card.IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        day_of_black_sun.abilities[0].contains("Effect::CaptureMatching"),
        "expected Day of Black Sun CaptureMatching: {:?}",
        day_of_black_sun.abilities
    );

    let gaze = read_w18b(
        "Name:Gaze of the Gorgon\nManaCost:3 BG\nTypes:Instant\n\
         A:SP$ Regenerate | ValidTgts$ Creature | SubAbility$ TrigGorgonEffect | SpellDescription$ Regenerate target creature.\n\
         SVar:TrigGorgonEffect:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | ThisTurn$ True | RememberObjects$ Targeted | Execute$ TrigRem\n\
         SVar:TrigRem:DB$ Bookkeeping | RememberObjects$ DelayTriggerRemembered | SubAbility$ TrigGorgonDestroy\n\
         SVar:TrigGorgonDestroy:DB$ DestroyAll | ValidCards$ Creature.blockedRemembered,Creature.blockedByRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        gaze.statics.contains("BlockedReferenceThisTurn"),
        "expected Gaze BlockedReferenceThisTurn in statics: {:?}",
        gaze.statics
    );

    let faerie = read_w18b(
        "Name:Faerie Artisans\nManaCost:3 U\nTypes:Creature Faerie Artificer\nPT:2/2\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.!token+OppCtrl | TriggerZones$ Battlefield | Execute$ TrigImprint | TriggerDescription$ Whenever a nontoken creature enters under an opponent's control, create a token that's a copy of that creature except it's an artifact in addition to its other types. Then exile all other tokens created with CARDNAME.\n\
         SVar:TrigImprint:DB$ Bookkeeping | ImprintCards$ Remembered | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ TriggeredCardLKICopy | Controller$ You | AddTypes$ Artifact | RememberTokens$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Card.IsImprinted\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ DBCleanup | Static$ True\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        faerie.statics.contains("CreatedBySource"),
        "expected Faerie Artisans CreatedBySource in statics: {:?}",
        faerie.statics
    );

    let flickerform = read_w18b(
        "Name:Flickerform\nManaCost:1 W\nTypes:Enchantment Aura\n\
         K:Enchant:Creature\n\
         A:AB$ Bookkeeping | Cost$ 2 W W | Defined$ Enchanted | ImprintCards$ Enchanted | StackDescription$ SpellDescription | SubAbility$ DBRemAura | SpellDescription$ Exile enchanted creature and all Auras attached to it.\n\
         SVar:DBRemAura:DB$ Bookkeeping | RememberAll$ Card.IsImprinted,Aura.AttachedTo Creature.IsImprinted | StackDescription$ None | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Card.IsRemembered | SubAbility$ DelayedEffect\n\
         SVar:DelayedEffect:DB$ Effect | Triggers$ TrigEOT | RememberObjects$ Remembered | ImprintCards$ Imprinted | Duration$ Permanent | SubAbility$ DBCleanup\n\
         SVar:TrigEOT:Mode$ Phase | Phase$ End of Turn | Execute$ FlickerformReturn | OneOff$ True | TriggerZones$ Command\n\
         SVar:FlickerformReturn:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered+IsImprinted | Origin$ Exile | Destination$ Battlefield | SubAbility$ ClearImprint\n\
         SVar:ClearImprint:DB$ Cleanup | ClearImprinted$ True | SubAbility$ FlickerformAttach\n\
         SVar:FlickerformAttach:DB$ RepeatEach | UseImprinted$ True | RepeatCards$ Aura.IsRemembered | Zone$ Exile | RepeatSubAbility$ DBAllAuraAttach\n\
         SVar:DBAllAuraAttach:DB$ ChangeZone | Defined$ Imprinted | Origin$ Exile | Destination$ Battlefield | AttachedTo$ Valid Creature.IsRemembered\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        flickerform
            .abilities
            .iter()
            .any(|a| a.contains("Effect::Flickerform")),
        "expected Effect::Flickerform: {:?}",
        flickerform.abilities
    );

    let tainted_bk = read_w18b(
        "Name:Tainted Pact\nManaCost:1 B\nTypes:Instant\n\
         A:SP$ Repeat | RepeatSubAbility$ DBRepeat | RepeatCheckSVar$ TaintedCheck | RepeatSVarCompare$ EQ0 | RepeatDefined$ Imprinted | RepeatPresent$ Card.sharesNameWith Remembered | RepeatCompare$ EQ0 | SubAbility$ DBCleanup | StackDescription$ SpellDescription\n\
         SVar:DBRepeat:DB$ Bookkeeping | RememberObjects$ Imprinted | SubAbility$ DBForget\n\
         SVar:DBForget:DB$ Cleanup | ClearImprinted$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ Dig | Defined$ You | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | Imprint$ True | SubAbility$ DBReturnToHand\n\
         SVar:DBReturnToHand:DB$ ChangeZone | ConditionCheckSVar$ CheckName | ConditionSVarCompare$ EQ0 | Origin$ Exile | Destination$ Hand | Defined$ Imprinted | Optional$ True | SubAbility$ LibraryCheck\n\
         SVar:LibraryCheck:DB$ StoreSVar | SVar$ TaintedCheck | Type$ Number | Expression$ 1 | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True | SubAbility$ DBReset\n\
         SVar:DBReset:DB$ StoreSVar | SVar$ TaintedCheck | Type$ Number | Expression$ 0\n\
         SVar:TaintedCheck:Count$ValidHand Card.IsImprinted\n\
         SVar:CheckName:Imprinted$Valid Card.sharesNameWith Remembered\n\
         SVar:X:Count$ValidLibrary Card.YouOwn",
    );
    assert!(
        tainted_bk.abilities[0].contains("Effect::TaintedPact"),
        "expected Effect::TaintedPact: {:?}",
        tainted_bk.abilities
    );

    let phyrexian_grimoire = read_w18b(
        "Name:Phyrexian Grimoire\nManaCost:3\nTypes:Artifact Book\n\
         A:AB$ Bookkeeping | Cost$ 4 T | RememberAll$ Card.TopGraveyard2+YouCtrl | RememberZone$ Graveyard | SubAbility$ DBChoose | StackDescription$ None\n\
         SVar:DBChoose:DB$ ChooseCard | ValidTgts$ Opponent | Choices$ Card.IsRemembered | ChoiceZone$ Graveyard | Mandatory$ True | ForgetChosen$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Graveyard | Destination$ Exile | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Graveyard | Destination$ Hand | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        phyrexian_grimoire.abilities[0].contains("Effect::PhyrexianGrimoire"),
        "expected Effect::PhyrexianGrimoire: {:?}",
        phyrexian_grimoire.abilities
    );

    let aetherspouts = read_w18b(
        "Name:Aetherspouts\nManaCost:3 U U\nTypes:Instant\n\
         A:SP$ Bookkeeping | ImprintCards$ Valid Creature.attacking | StackDescription$ None | SubAbility$ DBRepeatPlayer\n\
         SVar:DBRepeatPlayer:DB$ RepeatEach | RepeatPlayers$ ImprintedOwner | RepeatSubAbility$ DBRepeatCard | SubAbility$ DBCleanup\n\
         SVar:DBRepeatCard:DB$ RepeatEach | RepeatCards$ Card.IsImprinted+RememberedPlayerOwn | ChooseOrder$ RememberedPlayer | RepeatSubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ GenericChoice | Defined$ RememberedOwner | ShowCurrentCard$ Remembered | Choices$ DBTop,DBBottom\n\
         SVar:DBTop:DB$ ChangeZone | Defined$ RememberedCard | Origin$ Battlefield | Destination$ Library | LibraryPosition$ 0 | SpellDescription$ Top of library\n\
         SVar:DBBottom:DB$ ChangeZone | Defined$ RememberedCard | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1 | SpellDescription$ Bottom of library\n\
         SVar:DBCleanup:DB$ Cleanup | ClearImprinted$ True",
    );
    assert!(
        aetherspouts.abilities[0].contains("OwnerPutsOnTopOrBottom"),
        "expected Aetherspouts OwnerPutsOnTopOrBottom: {:?}",
        aetherspouts.abilities
    );

    let creeping_dread = read_w18b(
        "Name:Creeping Dread\nManaCost:3 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDiscard\n\
         SVar:TrigDiscard:DB$ Discard | Defined$ Player | NumCards$ 1 | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ DBImprint\n\
         SVar:DBImprint:DB$ Bookkeeping | ImprintCards$ ValidAll Card.IsRemembered+YouCtrl | SubAbility$ DBForget\n\
         SVar:DBForget:DB$ Bookkeeping | ForgetObjects$ Imprinted | SubAbility$ DBRepeat\n\
         SVar:DBRepeat:DB$ RepeatEach | DefinedCards$ Remembered | ClearRemembered$ True | RepeatSubAbility$ DBDmg | SubAbility$ DBCleanup\n\
         SVar:DBDmg:DB$ LoseLife | Defined$ RememberedController | LifeAmount$ 3 | ConditionDefined$ Remembered | ConditionPresent$ Card.sharesCardTypeWith Imprinted+OppOwn\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        creeping_dread.abilities[0].contains("Effect::CreepingDread"),
        "expected Effect::CreepingDread: {:?}",
        creeping_dread.abilities
    );

    let unforge = read_w18b(
        "Name:Unforge\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Equipment | IsCurse$ True | SubAbility$ DBRem | StackDescription$ SpellDescription\n\
         SVar:DBRem:DB$ Bookkeeping | RememberAll$ Creature.EquippedByTargeted | StackDescription$ None | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | Defined$ Targeted | SubAbility$ DBDmg | StackDescription$ None\n\
         SVar:DBDmg:DB$ DealDamage | Defined$ Remembered | NumDmg$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ1 | StackDescription$ None | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        unforge.abilities[0].contains("Effect::RememberTargetsThen"),
        "expected Unforge RememberTargetsThen: {:?}",
        unforge.abilities
    );

    let eye = read_w18b(
        "Name:Eye of Singularity\nManaCost:3 W\nTypes:World Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigRepeat\n\
         SVar:TrigRepeat:DB$ RepeatEach | RepeatCards$ Permanent.nonBasic | RepeatSubAbility$ DBRem | SubAbility$ DBDestroy\n\
         SVar:DBRem:DB$ Bookkeeping | ImprintCards$ Valid Permanent.sharesNameWith Remembered+!IsRemembered\n\
         SVar:DBDestroy:DB$ DestroyAll | ValidCards$ Card.IsImprinted | NoRegen$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearImprinted$ True\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Permanent.nonBasic | TriggerZones$ Battlefield | Execute$ TrigDestroy\n\
         SVar:TrigDestroy:DB$ DestroyAll | ValidCards$ Permanent.!TriggeredCard+sharesNameWith TriggeredCard | NoRegen$ True",
    );
    assert_eq!(
        eye.abilities.len(),
        2,
        "expected 2 abilities on Eye of Singularity"
    );
}
