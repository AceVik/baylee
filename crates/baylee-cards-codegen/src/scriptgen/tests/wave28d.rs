use super::*;

fn cats_w28d() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.extend([
        "Zombie".into(),
        "Snake".into(),
        "Elf".into(),
        "Warrior".into(),
        "Vampire".into(),
        "Saproling".into(),
        "Cat".into(),
    ]);
    c.artifact
        .extend(["Equipment".into(), "Food".into(), "Treasure".into()]);
    c.land.extend(["Plains".into(), "Swamp".into()]);
    c.normalize();
    c
}

fn read_w28d(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &cats_w28d(), Some(&tokens_w28d())).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &cats_w28d(), Some(&tokens_w28d()))
        )
    })
}

fn refused_w28d(text: &str) -> bool {
    transcode(&parse(text), &cats_w28d(), Some(&tokens_w28d())).is_none()
}

fn tokens_w28d() -> TokenLookup {
    TokenLookup::held(&[
        (
            "g_1_1_saproling",
            "Name:Saproling\nTypes:Creature Saproling\nColors:green\nPT:1/1",
        ),
        (
            "b_2_2_zombie",
            "Name:Zombie\nTypes:Creature Zombie\nColors:black\nPT:2/2",
        ),
        (
            "c_a_treasure_sac",
            "Name:Treasure\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any",
        ),
    ])
}

#[test]
fn wave28d_discard_reveal_and_look_lowerings() {
    // 1. `Mode$ RevealYouChoose` with non-ChosenColor `DiscardValidDesc$` (Despise / Inquisition shape).
    let inquisition = read_w28d(
        "Name:Inquisition\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | \
         DiscardValid$ Card.nonLand+cmcLE3 | DiscardValidDesc$ nonland card with mana value 3 or less\n\
         Oracle:Target player reveals their hand. You choose a nonland card from it with mana value 3 or less. That player discards that card.",
    );
    assert!(
        inquisition.abilities[0].contains("Effect::RevealHandDiscard"),
        "{}",
        inquisition.abilities[0]
    );

    // 2. Optional `RevealYouChoose` (Extract the Truth shape) -> RevealHand + ChooseCardEx(0..=1) + DiscardDefined.
    let extract = read_w28d(
        "Name:Extract\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | Mode$ RevealYouChoose | Optional$ True | \
         DiscardValid$ Creature,Enchantment | DiscardValidDesc$ creature or enchantment\n\
         Oracle:Target opponent reveals their hand. You may choose a creature or enchantment card from it. If you do, that player discards that card.",
    );
    assert!(
        extract.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && extract.abilities[0].contains("min: 0, max: Amount::Fixed(1)")
            && extract.abilities[0]
                .contains("Effect::DiscardDefined { cards: DefinedObjects::Found }"),
        "{}",
        extract.abilities[0]
    );

    // 3. `RevealYouChoose | Optional$ True | RememberDiscarded$ True` (Traumatic Revelation shape):
    //    `RevealHand` MUST be hoisted BEFORE `CaptureAffected` so `harvest_captured` never
    //    conflates revealed cards with discarded cards when 0 cards are discarded.
    let traumatic = read_w28d(
        "Name:Traumatic\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | Mode$ RevealYouChoose | Optional$ True | \
         DiscardValid$ Creature | DiscardValidDesc$ creature | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Target opponent reveals their hand. You may choose a creature card from it. If you do, that player discards that card. If you don't, draw a card.",
    );
    let idx_reveal = traumatic.abilities[0].find("Effect::RevealHand").unwrap();
    let idx_capture = traumatic.abilities[0]
        .find("Effect::CaptureAffected")
        .unwrap();
    assert!(
        idx_reveal < idx_capture,
        "RevealHand must precede CaptureAffected: {}",
        traumatic.abilities[0]
    );

    // 4. `RevealYouChoose` with `NumCards$ 2` and `Defined$ Targeted` (Gruesome Discovery Morbid branch).
    let gruesome = read_w28d(
        "Name:Gruesome\nManaCost:2 B B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ TgtChoose | NumCards$ 2 | ConditionCheckSVar$ Morbid | ConditionSVarCompare$ EQ0 | SubAbility$ MorbidDiscard\n\
         SVar:MorbidDiscard:DB$ Discard | Defined$ Targeted | Mode$ RevealYouChoose | NumCards$ 2 | ConditionCheckSVar$ Morbid\n\
         SVar:Morbid:Count$Morbid.1.0\n\
         Oracle:Target player discards two cards.\\nMorbid — If a creature died this turn, instead that player reveals their hand, you choose two cards from it, then that player discards those cards.",
    );
    assert!(
        gruesome.abilities[0].contains("min: 2, max: Amount::Fixed(2)")
            && gruesome.abilities[0].contains("Condition::CreaturesDiedThisTurnAtLeast(1)"),
        "{}",
        gruesome.abilities[0]
    );

    // 5. `RevealYouChoose` on private "look at ... hand" Oracle (Doomsday Specter) -> LookHandDiscard!
    let specter = read_w28d(
        "Name:Specter\nManaCost:2 U B\nTypes:Creature\nPT:2/3\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDiscard | TriggerZones$ Battlefield\n\
         SVar:TrigDiscard:DB$ Discard | Defined$ TriggeredTarget | NumCards$ 1 | Mode$ RevealYouChoose\n\
         Oracle:Whenever Specter deals combat damage to a player, look at that player's hand and choose a card from it. That player discards that card.",
    );
    assert!(
        specter.abilities[0].contains(
            "Effect::LookHandDiscard { player: PlayerRel::DamagedPlayer, count: Amount::Fixed(1), filter: &Filter::Any }"
        ),
        "{}",
        specter.abilities[0]
    );

    // 6. `Mode$ LookYouChoose` with `NumCards$ X` (Abandon Hope).
    let abandon = read_w28d(
        "Name:Abandon\nManaCost:X 1 B\nTypes:Sorcery\n\
         A:SP$ Discard | Cost$ X 1 B Discard<X/Card/card> | ValidTgts$ Opponent | Mode$ LookYouChoose | NumCards$ X\n\
         SVar:X:Count$xPaid\n\
         Oracle:As an additional cost to cast this spell, discard X cards.\\nLook at target opponent's hand and choose X cards from it. That player discards those cards.",
    );
    assert!(
        abandon.abilities[0].contains(
            "Effect::LookHandDiscard { player: PlayerRel::Chosen, count: Amount::X, filter: &Filter::Any }"
        ),
        "{}",
        abandon.abilities[0]
    );
}

#[test]
fn wave28d_discard_tgt_choose_and_refusals() {
    // 7. `Mode$ TgtChoose | AnyNumber$ True | Optional$ True | RememberDiscarded$ True` (Sacred Rites).
    let sacred = read_w28d(
        "Name:Sacred Rites\nManaCost:W\nTypes:Instant\n\
         A:SP$ Discard | Defined$ You | AnyNumber$ True | Mode$ TgtChoose | Optional$ True | RememberDiscarded$ True | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Creature.YouCtrl | NumDef$ +X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:Discard any number of cards. Creatures you control get +0/+1 until end of turn for each card discarded this way.",
    );
    assert!(
        sacred.abilities[0]
            .contains("Effect::DiscardUpToForPlayers { who: PlayerRel::You, max: u8::MAX }")
            && sacred.abilities[0].contains("Amount::ThisWay(ThisWay::Discarded)"),
        "{}",
        sacred.abilities[0]
    );

    // 8. `Mode$ TgtChoose | Optional$ True | NumCards$ X` with "up to" Oracle (Mishra's Command mode 1).
    let mishra_disc = read_w28d(
        "Name:UpToDisc\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ Discard | Optional$ True | ValidTgts$ Player | NumCards$ X | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ Y | Defined$ Targeted | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$RememberedSize\n\
         Oracle:Target player may discard up to X cards. Then they draw a card for each card discarded this way.",
    );
    assert!(
        mishra_disc.abilities[0].contains("min: 0, max: Amount::X")
            && mishra_disc.abilities[0]
                .contains("Effect::DiscardDefined { cards: DefinedObjects::Found }"),
        "{}",
        mishra_disc.abilities[0]
    );

    // 9. `Defined$ TargetedOwner` (Recoil) -> `PlayerRel::OwnerOfTarget`.
    let recoil = read_w28d(
        "Name:Recoil\nManaCost:1 U B\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ TargetedOwner | NumCards$ 1 | Mode$ TgtChoose\n\
         Oracle:Return target permanent to its owner's hand. Then that player discards a card.",
    );
    assert!(
        recoil.abilities[0]
            .contains("Effect::DiscardForPlayers { who: PlayerRel::OwnerOfTarget, count: 1 }"),
        "{}",
        recoil.abilities[0]
    );

    // 10. Blightning two-step collapse (`TargetedPlayer` + `TargetedController`).
    let blightning = read_w28d(
        "Name:Blightning\nManaCost:1 B R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 3 | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ TargetedPlayer | NumCards$ 2 | Mode$ TgtChoose | SubAbility$ DBDiscard2\n\
         SVar:DBDiscard2:DB$ Discard | Defined$ TargetedController | NumCards$ 2 | Mode$ TgtChoose\n\
         Oracle:Blightning deals 3 damage to target player or planeswalker. That player or that planeswalker's controller discards two cards.",
    );
    assert!(
        blightning.abilities[0].contains(
            "Effect::DiscardForPlayers { who: PlayerRel::TargetPlayerOrController, count: 2 }"
        ),
        "{}",
        blightning.abilities[0]
    );

    // Refusals:
    for bad in [
        // Optional RevealYouChoose on private "look at" Oracle (Cracked Skull / Oildeep Gearhulk) MUST refuse!
        "Name:CrackedSkull\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | Optional$ True | DiscardValid$ Card.nonLand | DiscardValidDesc$ nonland card\n\
         Oracle:Look at target player's hand. You may choose a nonland card from it. That player discards that card.",
        // RevealNumber$ subset reveal (Blackmail) MUST refuse!
        "Name:Blackmail\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | Mode$ RevealYouChoose | RevealNumber$ 3 | NumCards$ 1",
        // Optional LookYouChoose MUST refuse!
        "Name:OptLook\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Opponent | Mode$ LookYouChoose | Optional$ True | NumCards$ 1",
        // Optional TgtChoose with NumCards$ > 1 and NO "up to" in Oracle MUST refuse!
        "Name:OptTwoNoUpTo\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ You | Mode$ TgtChoose | Optional$ True | NumCards$ 2\n\
         Oracle:You may discard two cards.",
        // Converge amount MUST refuse!
        "Name:ConvergeDisc\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Discard | ValidTgts$ Player | NumCards$ X | Mode$ TgtChoose\n\
         SVar:X:Count$Converge",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_mill_optional_and_imprint_capture() {
    // 1. `Mill | Optional$ True` -> MayDo / PlayerMayDo.
    let may_mill = read_w28d(
        "Name:MayMill\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 3 | Defined$ You | Optional$ True",
    );
    assert!(
        may_mill.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::Mill { amount: Amount::Fixed(3), target: PlayerRel::You }] }"),
        "{}",
        may_mill.abilities[0]
    );

    // 2. Targeted `Mill | RememberMilled$ True` -> `ChangeZoneAll(Graveyard,Exile -> Hand)` (Grub's Command mode 4).
    let grub = read_w28d(
        "Name:GrubMill\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 5 | ValidTgts$ Player | RememberMilled$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Goblin.IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        grub.abilities[0].contains("Effect::CaptureAffected")
            && grub.abilities[0].contains("Effect::ForEachRemembered")
            && grub.abilities[0]
                .contains("Effect::ReturnToHand { target: TargetSpec::EventObject }"),
        "{}",
        grub.abilities[0]
    );

    // 3. Single-resolution `Mill | Imprint$ True` -> nested CaptureAffected with EQ0 fallback (Blanchwood Prowler).
    let prowler = read_w28d(
        "Name:Prowler\nManaCost:1 G\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 3 | Defined$ You | Imprint$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Land.YouOwn+IsImprinted | RememberChanged$ True | Hidden$ True | Optional$ True | SubAbility$ DBCounter\n\
         SVar:DBCounter:DB$ PutCounter | CounterType$ P1P1 | Defined$ Self | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        prowler.abilities[0].contains("Effect::CaptureAffected")
            && prowler.abilities[0].contains("zone: ChooseZone::Remembered")
            && prowler.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        prowler.abilities[0]
    );

    // Refusals: multi-seat optional mill; persistent imprint with external reader.
    for bad in [
        "Name:AllMayMill\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 2 | Defined$ Player | Optional$ True",
        "Name:PersistentImprintMill\nManaCost:2 U\nTypes:Artifact\n\
         A:AB$ Mill | Cost$ T | NumCards$ 1 | Defined$ You | Imprint$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearImprinted$ True\n\
         A:AB$ ChangeZone | Cost$ 1 | Defined$ Imprinted | Origin$ Graveyard | Destination$ Hand",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_change_zone_all_and_refusals() {
    // 1. Filtered Graveyard -> Exile with RememberChanged$ True (Crypt Incursion).
    let incursion = read_w28d(
        "Name:Crypt Incursion\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Player | ChangeType$ Creature | RememberChanged$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount/Thrice",
    );
    assert!(
        incursion.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::Chosen)")
            && incursion.abilities[0].contains("not_chosen_movement: ChooseMove::Exile")
            && incursion.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)"),
        "{}",
        incursion.abilities[0]
    );

    // 2. Multi-player Graveyard -> Hand (Empty the Catacombs).
    let catacombs = read_w28d(
        "Name:Empty the Catacombs\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        catacombs.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::EachPlayer)")
            && catacombs.abilities[0].contains("not_chosen_movement: ChooseMove::Hand"),
        "{}",
        catacombs.abilities[0]
    );

    // 3. Face-up Hand -> Exile (Worldfire / Decree of Annihilation step).
    let exile_hands = read_w28d(
        "Name:ExileHands\nManaCost:6 R\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Hand | Destination$ Exile | ChangeType$ Card",
    );
    assert!(
        exile_hands.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::EachPlayer)")
            && exile_hands.abilities[0].contains("not_chosen_movement: ChooseMove::Exile"),
        "{}",
        exile_hands.abilities[0]
    );

    // 4. Non-discard Hand -> Graveyard + NotRemembered Graveyard -> Hand (Harness Infinity).
    let harness = read_w28d(
        "Name:Harness Infinity\nManaCost:1 B B B G G G\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card.YouCtrl | Origin$ Hand | Destination$ Graveyard | RememberChanged$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | ChangeType$ Card.YouCtrl+!IsRemembered | Origin$ Graveyard | Destination$ Hand | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        harness.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && harness.abilities[0].contains("not_chosen_movement: ChooseMove::Graveyard")
            && harness.abilities[0].contains("Effect::GraveyardAllToHand"),
        "{}",
        harness.abilities[0]
    );

    // 5. Hand -> Battlefield with "reveal your hand" Oracle (Manabond).
    let manabond = read_w28d(
        "Name:Manabond\nManaCost:G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigChangeAll\n\
         SVar:TrigChangeAll:DB$ ChangeZoneAll | ChangeType$ Land.YouCtrl | Origin$ Hand | Destination$ Battlefield | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Mode$ Hand\n\
         Oracle:At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand.",
    );
    assert!(
        manabond.abilities[0].contains("Effect::RevealHand { who: PlayerRel::You }")
            && manabond.abilities[0]
                .contains("not_chosen_movement: ChooseMove::Battlefield { tapped: false }")
            && manabond.abilities[0].contains("Effect::DiscardHand { who: PlayerRel::You }"),
        "{}",
        manabond.abilities[0]
    );

    // 6. Battlefield -> Library bottom with RandomOrder$ True.
    let rand_bottom = read_w28d(
        "Name:RandBottom\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True",
    );
    assert!(
        rand_bottom.abilities[0].contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        rand_bottom.abilities[0]
    );

    // Refusals:
    for bad in [
        // Face-down exile (Memory Jar / Ghastly Conscription) MUST refuse!
        "Name:FaceDownCza\nManaCost:5\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand | Destination$ Exile | ExileFaceDown$ True",
        // Multi-owner chosen-order library bottom (Hallowed Burial) MUST refuse (CR 401.4)!
        "Name:HallowedBurial\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1",
        // Multi-owner library top (Harmonic Convergence) MUST refuse!
        "Name:HarmonicConvergence\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Enchantment | Origin$ Battlefield | Destination$ Library | LibraryPosition$ 0",
        // Duration$ UntilHostLeavesPlay (Banishment) MUST refuse!
        "Name:BanishLike\nManaCost:3 W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ E\n\
         SVar:E:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature | Duration$ UntilHostLeavesPlay",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_peek_and_reveal_and_dig_until() {
    // 1. Optional top-1 take with optional graveyard rest (Archghoul of Thraben) -> LookAtTopMayTake.
    let archghoul = read_w28d(
        "Name:Archghoul\nManaCost:2 B\nTypes:Creature Zombie\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self,Zombie.Other+YouCtrl | Execute$ TrigPeek\n\
         SVar:TrigPeek:DB$ PeekAndReveal | PeekAmount$ 1 | RevealValid$ Zombie | RevealOptional$ True | RememberRevealed$ True | SubAbility$ DBToHand\n\
         SVar:DBToHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand | SubAbility$ DBToGrave\n\
         SVar:DBToGrave:DB$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Graveyard | Optional$ True | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        archghoul.abilities[0].contains("Effect::LookAtTopMayTake")
            && archghoul.abilities[0].contains("rest: LookRest::MayGraveyard"),
        "{}",
        archghoul.abilities[0]
    );

    // 2. Multi-seat private look (Case the Joint).
    let case_joint = read_w28d(
        "Name:Case the Joint\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 2 | SubAbility$ DBLook\n\
         SVar:DBLook:DB$ PeekAndReveal | Defined$ Player | NoReveal$ True",
    );
    assert!(
        case_joint.abilities[0].contains(
            "Effect::RepeatEach { domain: RepeatDomain::EachPlayer, effects: &[Effect::LookAtTopOf { player: PlayerRel::EventPlayer, count: 1 }] }"
        ),
        "{}",
        case_joint.abilities[0]
    );

    // 3. Optional public reveal (Elven Farsight).
    let farsight = read_w28d(
        "Name:Elven Farsight\nManaCost:G\nTypes:Sorcery\n\
         A:SP$ Scry | ScryNum$ 3 | SubAbility$ DBPeek\n\
         SVar:DBPeek:DB$ PeekAndReveal | RevealOptional$ True | NoPeek$ True | RememberRevealed$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        farsight.abilities[0].contains("Effect::MayDo { effects: &[Effect::RevealTop"),
        "{}",
        farsight.abilities[0]
    );

    // 4. Multi-filter single-card public reveal without `NoPeek$ True` (Druidic Satchel).
    let satchel = read_w28d(
        "Name:Druidic Satchel\nManaCost:3\nTypes:Artifact\n\
         A:AB$ PeekAndReveal | Cost$ 2 T | RememberRevealed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | TokenScript$ g_1_1_saproling | TokenOwner$ You | SubAbility$ DBMove\n\
         SVar:DBMove:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Card.Land | Defined$ Remembered | Origin$ Library | Destination$ Battlefield | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card.nonLand+nonCreature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        satchel.abilities[0].contains("Effect::CaptureAffected")
            && satchel.abilities[0].contains("Effect::RevealTop")
            && satchel.abilities[0].contains("Effect::ForEachRemembered")
            && satchel.abilities[0].contains("Effect::MoveAndModify"),
        "{}",
        satchel.abilities[0]
    );

    // 5. `DigUntil` exile-while-revealing -> Play(free, optional, ForgetPlayed) -> ChangeZoneAll(Exile->Bottom Random) (Chaos Wand).
    let wand = read_w28d(
        "Name:Chaos Wand\nManaCost:3\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 4 T | ValidTgts$ Opponent | Valid$ Instant,Sorcery | ValidDescription$ instant or sorcery | \
         FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | RememberRevealed$ True | IsCurse$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidSA$ Spell.Instant,Spell.Sorcery | WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True | SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        wand.abilities[0].contains("Effect::CaptureAffected")
            && wand.abilities[0].contains("Effect::RevealUntilThen")
            && wand.abilities[0].contains("Effect::Play {")
            && wand.abilities[0].contains("zone: ChooseZone::Remembered")
            && wand.abilities[0].contains("Filter::InZone(ZoneRef::Exile)")
            && wand.abilities[0].contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom"),
        "{}",
        wand.abilities[0]
    );

    // Refusals: Inscribed Tablet (bottomed cards would be conflated with picked cards when 0 picked);
    // DigUntil rest without RandomOrder$ True.
    for bad in [
        "Name:InscribedTablet\nManaCost:1\nTypes:Artifact\n\
         A:AB$ Dig | Cost$ 1 T Sac<1/CARDNAME> | DigNum$ 5 | Reveal$ True | ChangeNum$ 1 | RememberChanged$ True | ChangeValid$ Land | RestRandomOrder$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0",
        "Name:NonRandomWand\nManaCost:3\nTypes:Artifact\n\
         A:AB$ DigUntil | Cost$ 4 T | ValidTgts$ Opponent | Valid$ Instant,Sorcery | \
         FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | RememberRevealed$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ Remembered | ValidSA$ Spell.Instant,Spell.Sorcery | WithoutManaCost$ True | Optional$ True | ForgetPlayed$ True | SubAbility$ DBRest\n\
         SVar:DBRest:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn wave28d_remember_sacrificed_and_copy_permanent() {
    // 1. `Sacrifice | Amount$ SacX | Optional$ True | RememberSacrificed$ True` (Mana Seism).
    let seism = read_w28d(
        "Name:Mana Seism\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | Amount$ SacX | SacValid$ Land | RememberSacrificed$ True | Optional$ True | SubAbility$ DBMana\n\
         SVar:DBMana:DB$ Mana | Produced$ C | Amount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:SacX:Count$Valid Land.YouCtrl\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        seism.abilities[0].contains("Effect::CaptureAffected")
            && seism.abilities[0].contains("Effect::SacrificeAnyNumber"),
        "{}",
        seism.abilities[0]
    );

    // 2. Conditional self-sacrifice with RememberSacrificed (Daredevil Dragster shape).
    let dragster = read_w28d(
        "Name:Dragster\nManaCost:3\nTypes:Artifact\n\
         A:AB$ PutCounter | Cost$ 1 | CounterType$ CHARGE | CounterNum$ 1 | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | SacValid$ Self | RememberSacrificed$ True | ConditionPresent$ Card.Self+counters_GE2_CHARGE | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        dragster.abilities[0].contains("Effect::CaptureAffected { first: &[Effect::SacrificeSelf]"),
        "{}",
        dragster.abilities[0]
    );

    // 3. `Count$Compare Y GE1.5.3` + `Remembered$CardManaCost/Plus.1` (Insatiable Appetite & Iron Man).
    let appetite = read_w28d(
        "Name:Appetite\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Sacrifice | SacValid$ Food | Optional$ True | RememberSacrificed$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Compare Y GE1.5.3\n\
         SVar:Y:Remembered$Amount",
    );
    assert!(
        appetite.abilities[0].contains("Amount::IfCondition")
            && appetite.abilities[0].contains("Amount::RememberedCount"),
        "{}",
        appetite.abilities[0]
    );

    let iron_man = read_w28d(
        "Name:IronMan\nManaCost:3 U R\nTypes:Artifact Creature\nPT:4/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | SacValid$ Artifact.nonCreature | Optional$ True | RememberSacrificed$ True | SubAbility$ DBSearch\n\
         SVar:DBSearch:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Card | Origin$ Library | Destination$ Battlefield | ChangeType$ Artifact.cmcEQX | ChangeNum$ 1 | Tapped$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost/Plus.1",
    );
    assert!(
        iron_man
            .statics
            .contains("Amount::Plus { base: &Amount::RememberedManaValue, offset: 1 }"),
        "{}",
        iron_man.statics
    );

    // 4. `CopyPermanent`: Defined$ Valid, TargetsForEachPlayer, unused RememberTokens.
    let clone_legion = read_w28d(
        "Name:Clone Legion\nManaCost:7 U U\nTypes:Sorcery\n\
         A:SP$ CopyPermanent | ValidTgts$ Player | Defined$ Valid Creature.TargetedPlayerCtrl",
    );
    assert!(
        clone_legion.abilities[0].contains("Effect::RepeatEachObject")
            && clone_legion.abilities[0].contains("Effect::CreateTokenCopy")
            && clone_legion
                .statics
                .contains("Filter::ControlledBy(PlayerRel::Chosen)"),
        "{}\n{}",
        clone_legion.abilities[0],
        clone_legion.statics
    );

    let simulacrum = read_w28d(
        "Name:Simulacrum\nManaCost:4 U U\nTypes:Instant\n\
         A:SP$ CopyPermanent | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        simulacrum.abilities[0].contains("TargetSpec::ObjectOfEachOpponent("),
        "{}",
        simulacrum.abilities[0]
    );

    let flameshadow = read_w28d(
        "Name:Flameshadow\nManaCost:3 R\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | ValidCard$ Creature.!token+YouCtrl | Origin$ Any | Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:AB$ CopyPermanent | Cost$ R | Defined$ TriggeredCardLKICopy | PumpKeywords$ Haste | AtEOT$ Exile | RememberTokens$ True",
    );
    assert!(
        flameshadow.abilities[0].contains("Effect::PlayerMayPayManaThen")
            && flameshadow.abilities[0].contains("Effect::CreateTokenCopy"),
        "{}",
        flameshadow.abilities[0]
    );
}

#[test]
fn wave28d_pump_and_targeted_player_ctrl() {
    // 5. `Pump | Defined$ Targeted.<Subtype>` (Kaseto) & Slot-2 `SuppressUntap` (Chandra's Revolution).
    let kaseto = read_w28d(
        "Name:Kaseto\nManaCost:1 G U\nTypes:Creature Snake\nPT:2/2\n\
         A:AB$ Tap | Cost$ G U | ValidTgts$ Creature | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Targeted.Snake | NumAtt$ +2 | NumDef$ +2",
    );
    assert!(
        kaseto.abilities[0].contains("Effect::IfTargetMatches")
            && kaseto.statics.contains("subtypes::creature::SNAKE"),
        "{}\n{}",
        kaseto.abilities[0],
        kaseto.statics
    );

    let chandra_rev = read_w28d(
        "Name:ChandraRev\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | SubAbility$ DBTapLand\n\
         SVar:DBTapLand:DB$ Tap | ValidTgts$ Land | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ ParentTarget | KW$ HIDDEN This card doesn't untap during your next untap step. | Duration$ Permanent",
    );
    assert!(
        chandra_rev.abilities[0].contains("Effect::TapTargetSlot(TargetSlot::Second)")
            && chandra_rev.abilities[0]
                .contains("Effect::WithSecondTarget(&[Effect::SuppressUntap"),
        "{}",
        chandra_rev.abilities[0]
    );

    // 6. `TargetedPlayerCtrl` on Slot 1 (Anathemancer) AND Slot 2 (Dark Salvation -> FirstTargetPlayerOrController!).
    let anathemancer = read_w28d(
        "Name:Anathemancer\nManaCost:1 B R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Player | NumDmg$ X\n\
         SVar:X:Count$Valid Land.nonBasic+TargetedPlayerCtrl",
    );
    assert!(
        anathemancer.abilities[0].contains("Amount::ControlledCount { who: PlayerRel::Chosen,"),
        "{}",
        anathemancer.abilities[0]
    );

    let dark_salvation = read_w28d(
        "Name:Dark Salvation\nManaCost:X X B\nTypes:Sorcery\n\
         A:SP$ Token | ValidTgts$ Player | TokenAmount$ X | TokenScript$ b_2_2_zombie | TokenOwner$ TargetedPlayer | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | NumAtt$ -Y | NumDef$ -Y | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$Valid Zombie.TargetedPlayerCtrl",
    );
    assert!(
        dark_salvation.abilities[0]
            .contains("Amount::ControlledCount { who: PlayerRel::FirstTargetPlayerOrController,"),
        "{}",
        dark_salvation.abilities[0]
    );
}

#[test]
fn wave28d_repeat_player_and_targeted_base_filters() {
    // 7. `RememberedPlayerCtrl` in `RepeatEach` (Biorhythm & Omen of Fire).
    let biorhythm = read_w28d(
        "Name:Biorhythm\nManaCost:6 G G\nTypes:Sorcery\n\
         A:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBSetLife\n\
         SVar:DBSetLife:DB$ SetLife | Defined$ Player.IsRemembered | LifeAmount$ X\n\
         SVar:X:Count$Valid Creature.RememberedPlayerCtrl",
    );
    assert!(
        biorhythm.abilities[0].contains(
            "Effect::SetLifeTotal { who: PlayerRel::EventPlayer, amount: Amount::ControlledCount { who: PlayerRel::EventPlayer,"
        ),
        "{}",
        biorhythm.abilities[0]
    );

    let omen_of_fire = read_w28d(
        "Name:Omen of Fire\nManaCost:3 R R\nTypes:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Island | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBSacEach\n\
         SVar:DBSacEach:DB$ Sacrifice | Defined$ Player.IsRemembered | Amount$ OppX | SacValid$ Plains,Permanent.White\n\
         SVar:OppX:Count$Valid Permanent.White+RememberedPlayerCtrl",
    );
    assert!(
        omen_of_fire.abilities[0]
            .contains("Effect::SacrificeAmountForPlayers { who: PlayerRel::EachPlayer,")
            && omen_of_fire.abilities[0]
                .contains("Amount::ControlledCount { who: PlayerRel::Recipient,")
            && !omen_of_fire.abilities[0].contains("Effect::RepeatEach"),
        "{}",
        omen_of_fire.abilities[0]
    );

    // 8. `filter base Targeted` (`Targeted.Equipment+Attached` & `Targeted.Other+YouCtrl+Creature`).
    let blastfire = read_w28d(
        "Name:Blastfire Bolt\nManaCost:5 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 5 | SubAbility$ ChompEquip\n\
         SVar:ChompEquip:DB$ DestroyAll | ValidCards$ Targeted.Equipment+Attached",
    );
    let blastfire_all = format!("{}\n{}", blastfire.abilities[0], blastfire.statics);
    assert!(
        blastfire.abilities[0].contains("Effect::destroy_all")
            && blastfire_all.contains("Filter::AttachedTo(&Filter::Reference)")
            && blastfire_all.contains("subtypes::artifact::EQUIPMENT"),
        "{blastfire_all}"
    );

    let phalanx = read_w28d(
        "Name:Phalanx Tactics\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +1 | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Targeted.Other+YouCtrl+Creature | NumAtt$ +1 | NumDef$ +1",
    );
    let phalanx_all = format!("{}\n{}", phalanx.abilities[0], phalanx.statics);
    assert!(
        phalanx.abilities[0].contains("Effect::PumpFilter")
            && phalanx_all.contains("Filter::Not(&Filter::Reference)"),
        "{phalanx_all}"
    );
}

#[test]
fn wave28d_filter_copy_and_pump_refusals() {
    for bad in [
        // Multi-player target with TargetedPlayerCtrl (Call the Coppercoats) MUST refuse!
        "Name:MultiPlayerCtrl\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Draw | Defined$ You | ValidTgts$ Opponent | TargetMin$ 0 | TargetMax$ MaxTargets | NumCards$ X\n\
         SVar:MaxTargets:PlayerCountOpponents$Amount\n\
         SVar:X:Count$Valid Creature.TargetedPlayerCtrl",
        // Targeted.sameName (Echoing Courage) and Targeted.PairedWith (Joint Assault) MUST refuse!
        "Name:EchoingCourage\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Targeted.sameName+Other | NumAtt$ +2 | NumDef$ +2",
        "Name:JointAssault\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBPumpAll\n\
         SVar:DBPumpAll:DB$ PumpAll | ValidCards$ Targeted.PairedWith | NumAtt$ +2 | NumDef$ +2",
        // CopyPermanent DefinedName$ MUST refuse!
        "Name:BadDefinedName\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ CopyPermanent | DefinedName$ Giant Badger",
        // CopySpellAbility on AbilityCast (engine EventAbility not wired to CopySpellOrAbility) MUST refuse!
        "Name:BracersLike\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ AbilityCast | ValidCard$ Creature.YouCtrl | ValidSA$ SpellAbility.!ManaAbility | TriggerZones$ Battlefield | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
        // Pump "must be blocked if able" (NOT Lure!) MUST refuse!
        "Name:CompelledDuel\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | KW$ HIDDEN CARDNAME must be blocked if able.",
        // Coven (DifferentCardPower) MUST refuse!
        "Name:CovenLike\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ X\n\
         SVar:X:Count$Valid Creature.YouCtrl$DifferentCardPower",
        // Gravitic Punch (DamageEqualToPower only supports battlefield objects, not players) MUST refuse!
        "Name:GraviticPunch\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouCtrl | SubAbility$ SoulsDamage\n\
         SVar:SoulsDamage:DB$ DealDamage | ValidTgts$ Player | TargetUnique$ True | NumDmg$ X | DamageSource$ ParentTarget\n\
         SVar:X:ParentTargeted$CardPower",
    ] {
        assert!(refused_w28d(bad), "must refuse:\n{bad}");
    }
}
