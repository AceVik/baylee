//! The sentences: each entry of the book as one reader reads it.

#[allow(clippy::wildcard_imports)] // the log's own vocabulary
use super::*;

/// Writes lines for one reader.
pub(super) struct Writer<'a> {
    pub(super) wording: &'a Wording<'a>,
    pub(super) defenders: &'a BTreeMap<ObjectId, Named>,
}

impl Writer<'_> {
    pub(super) fn line(
        &self,
        index: usize,
        entry: &LogEntry,
        before: Option<&LogEntry>,
    ) -> LogLine {
        let follows = follows(entry, before);
        let mut piece = if follows {
            self.short(entry)
        } else {
            self.sentence(entry)
        };
        if piece.lower {
            // ASCII only, so no span moves.
            if let Some(first) = piece.text.get_mut(..1) {
                first.make_ascii_uppercase();
            }
        }
        let times = match entry.event {
            LogEvent::Life { .. } | LogEvent::Counters { .. } | LogEvent::PlayerCounters { .. } => {
                1
            }
            _ => entry.repeat.max(1),
        };
        LogLine {
            index,
            turn: entry.turn,
            times,
            header: matches!(entry.event, LogEvent::TurnStarted { .. }),
            at: entry.at,
            text: piece.text,
            names: piece.names,
            players: piece.players,
            ability: match entry.event {
                LogEvent::Ability { ability, .. } => ability,
                _ => None,
            },
            subject: piece.subject,
            crowned: matches!(entry.event, LogEvent::BecameMonarch { .. }),
            phase: entry.phase,
            follows,
        }
    }

    /// What an [`LogEvent::Aimed`] line says after its spell or ability:
    /// "mode: …", "targeting …", or both, joined.
    fn aim_parts(&self, entry: &LogEntry) -> Piece {
        let LogEvent::Aimed {
            rules,
            modes,
            targets,
            ..
        } = &entry.event
        else {
            return Piece::default();
        };
        let mut parts = Vec::new();
        if let Some(rules) = rules.filter(|_| *modes != 0) {
            let chosen: Vec<String> = (0..8_usize)
                .filter(|i| modes & (1 << i) != 0)
                .map(|i| {
                    self.wording.texts.mode(rules, i).map_or_else(
                        || Phrase::CastModeNumber.fill(self.wording.lang, &[&(i + 1).to_string()]),
                        |words| Phrase::LogQuoted.fill(self.wording.lang, &[&words]),
                    )
                })
                .collect();
            let phrase = if chosen.len() > 1 {
                Phrase::LogAimedModes
            } else {
                Phrase::LogAimedMode
            };
            parts.push(self.phrase(
                phrase,
                &[Piece::default(), Piece::plain(chosen.join(" + "))],
            ));
        }
        if !targets.is_empty() {
            let named: Vec<Piece> = targets
                .iter()
                .map(|target| match target {
                    LogTarget::Player(player) if *player == self.wording.seat => {
                        self.you(Phrase::LogYouSubject)
                    }
                    LogTarget::Player(player) => self.seat(*player),
                    LogTarget::Object(object) => self.object(object),
                })
                .collect();
            let phrase = if named.len() > 1 {
                Phrase::LogAimedTargets
            } else {
                Phrase::LogAimedTarget
            };
            parts.push(self.phrase(phrase, &[Piece::default(), self.list(named)]));
        }
        let mut out = Piece::default();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                out.text.push_str("; ");
            }
            out.push(part);
        }
        out.lower = false;
        out
    }

    /// A line under the line it carries on ([`LogLine::follows`]), without
    /// naming again what that line named: an [`LogEvent::Aimed`] line's
    /// parts, about the spell's or the ability's controller, or the life
    /// total that damage left a player at.
    fn short(&self, entry: &LogEntry) -> Piece {
        match entry.event {
            LogEvent::Aimed { controller, .. } => Piece {
                subject: Some(controller),
                ..self.aim_parts(entry)
            },
            LogEvent::Life { player, old, new } => {
                let n = usize::try_from(new).unwrap_or(0);
                Piece {
                    subject: Some(player),
                    ..self.counted(
                        Phrase::LogLifeNowPoints,
                        n,
                        &[Piece::default(), Self::count(&old), Self::count(&new)],
                    )
                }
            }
            _ => self.sentence(entry),
        }
    }

    /// `phrase` filled with `args` from `{0}` on, and with the reading seat
    /// as `{7}`, `{8}` and `{9}`: "you" as a subject, a direct object and an
    /// indirect object, which German says three ways.
    fn phrase(&self, phrase: Phrase, args: &[Piece]) -> Piece {
        self.filled(phrase.text(self.wording.lang), args)
    }

    /// [`Self::phrase`] for a counted phrase: `n` picks its form.
    fn counted(&self, phrase: Phrase, n: usize, args: &[Piece]) -> Piece {
        self.filled(&phrase.text_count(self.wording.lang, n), args)
    }

    fn filled(&self, text: &str, args: &[Piece]) -> Piece {
        let mut all: Vec<Piece> = args.iter().map(Piece::copy).collect();
        all.resize_with(YOU_SLOT, Piece::default);
        all.extend(
            [
                Phrase::LogYouSubject,
                Phrase::LogYouObject,
                Phrase::LogYouIndirect,
            ]
            .map(|you| self.you(you)),
        );
        Piece::fill(text, &all)
    }

    /// The reading seat as `you` says it, marked as that seat.
    fn you(&self, you: Phrase) -> Piece {
        let text = you.text(self.wording.lang);
        Piece {
            players: vec![PlayerSpan {
                range: 0..text.len(),
                player: self.wording.seat,
            }],
            lower: true,
            ..Piece::plain(text)
        }
    }

    fn count(n: &impl ToString) -> Piece {
        Piece::plain(n.to_string())
    }

    /// Another seat's name, or nothing for the reading seat, whose sentence
    /// says "you" and has no `{0}`.
    fn seat(&self, player: PlayerId) -> Piece {
        if player == self.wording.seat {
            Piece::default()
        } else {
            let name = seat_name(self.wording.lang, self.wording.statics, player);
            Piece {
                players: vec![PlayerSpan {
                    range: 0..name.len(),
                    player,
                }],
                ..Piece::plain(name)
            }
        }
    }

    /// A sentence about `player`: `you` for the reading seat, `other` with
    /// its name as `{0}` for any other. `rest` fills `{1}` on.
    fn about(&self, player: PlayerId, you: Phrase, other: Phrase, rest: Vec<Piece>) -> Piece {
        self.about_in(player, you, other, None, rest)
    }

    /// [`Self::about`] for a counted sentence: `n` picks its form.
    fn about_counted(
        &self,
        player: PlayerId,
        you: Phrase,
        other: Phrase,
        n: usize,
        rest: Vec<Piece>,
    ) -> Piece {
        self.about_in(player, you, other, Some(n), rest)
    }

    fn about_in(
        &self,
        player: PlayerId,
        you: Phrase,
        other: Phrase,
        n: Option<usize>,
        rest: Vec<Piece>,
    ) -> Piece {
        let phrase = if player == self.wording.seat {
            you
        } else {
            other
        };
        let mut args = vec![self.seat(player)];
        args.extend(rest);
        let said = match n {
            Some(n) => self.counted(phrase, n, &args),
            None => self.phrase(phrase, &args),
        };
        Piece {
            subject: Some(player),
            ..said
        }
    }

    /// A sentence with no player in it: `{0}` is empty.
    fn about_nobody(&self, phrase: Phrase, rest: Vec<Piece>) -> Piece {
        let mut args = vec![Piece::default()];
        args.extend(rest);
        self.phrase(phrase, &args)
    }

    fn neutral(&self, phrase: Phrase) -> Piece {
        Piece {
            lower: true,
            ..Piece::plain(phrase.text(self.wording.lang))
        }
    }

    /// An object the seat may know, by the name its card has in the reader's
    /// language ([`shown_name`]), else by the name the line carries.
    fn known(
        &self,
        id: ObjectId,
        card: Option<CardIdentity>,
        token: Option<u16>,
        name: &str,
    ) -> Piece {
        let text = card.and_then(|card| self.wording.texts.text(card.index, card.face));
        let shown = shown_name(name, text.as_ref());
        let shown = if shown.trim().is_empty() { name } else { shown };
        if shown.trim().is_empty() {
            return self.neutral(Phrase::LogACard);
        }
        Piece {
            text: shown.to_string(),
            names: vec![NameSpan {
                range: 0..shown.len(),
                id,
                card,
                token,
            }],
            players: Vec::new(),
            lower: false,
            subject: None,
        }
    }

    fn object(&self, object: &LogObject) -> Piece {
        match object {
            LogObject::Known {
                id,
                card,
                token,
                name,
            } => self.known(*id, *card, *token, name),
            LogObject::FaceDown { id } => {
                let mut piece = self.neutral(Phrase::LogAFaceDownCard);
                piece.names.push(NameSpan {
                    range: 0..piece.text.len(),
                    id: *id,
                    card: None,
                    token: None,
                });
                piece
            }
            LogObject::Hidden => self.neutral(Phrase::LogACard),
        }
    }

    /// Names joined as the language joins a list: "A, B and C".
    fn list(&self, items: Vec<Piece>) -> Piece {
        let mut items = items;
        let Some(last) = items.pop() else {
            return Piece::default();
        };
        if items.is_empty() {
            return last;
        }
        let mut head = Piece::default();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                head.text.push_str(", ");
            }
            head.push(item);
        }
        self.phrase(Phrase::LogAnd, &[head, last])
    }

    fn objects(&self, objects: &[LogObject]) -> Piece {
        self.list(objects.iter().map(|o| self.object(o)).collect())
    }

    fn zone(&self, owner: PlayerId, zone: LogZone, into: bool) -> Piece {
        let yours = owner == self.wording.seat;
        let phrase = match (zone, into) {
            (LogZone::Library, false) if yours => Phrase::LogFromLibraryYou,
            (LogZone::Library, false) => Phrase::LogFromLibrary,
            (LogZone::Hand, false) if yours => Phrase::LogFromHandYou,
            (LogZone::Hand, false) => Phrase::LogFromHand,
            (LogZone::Graveyard, false) if yours => Phrase::LogFromGraveyardYou,
            (LogZone::Graveyard, false) => Phrase::LogFromGraveyard,
            (LogZone::Battlefield, false) => Phrase::LogFromBattlefield,
            (LogZone::Exile, false) => Phrase::LogFromExile,
            (LogZone::Command, false) => Phrase::LogFromCommand,
            (LogZone::Stack, false) => Phrase::LogFromStack,
            (LogZone::Library, true) if yours => Phrase::LogIntoLibraryYou,
            (LogZone::Library, true) => Phrase::LogIntoLibrary,
            (LogZone::Hand, true) if yours => Phrase::LogIntoHandYou,
            (LogZone::Hand, true) => Phrase::LogIntoHand,
            (LogZone::Graveyard, true) if yours => Phrase::LogIntoGraveyardYou,
            (LogZone::Graveyard, true) => Phrase::LogIntoGraveyard,
            (LogZone::Battlefield, true) => Phrase::LogIntoBattlefield,
            (LogZone::Exile, true) => Phrase::LogIntoExile,
            (LogZone::Command, true) => Phrase::LogIntoCommand,
            (LogZone::Stack, true) => Phrase::LogIntoStack,
        };
        self.phrase(phrase, &[self.seat(owner)])
    }

    /// Where in `owner`'s library a card went, or which library when the
    /// line cannot say where in it.
    fn library_place(&self, owner: PlayerId, place: Option<LogPlace>) -> Piece {
        let yours = owner == self.wording.seat;
        let (phrase, at) = match place {
            Some(LogPlace::Top) if yours => (Phrase::LogOntoLibraryTopYou, None),
            Some(LogPlace::Top) => (Phrase::LogOntoLibraryTop, None),
            Some(LogPlace::Bottom) if yours => (Phrase::LogOntoLibraryBottomYou, None),
            Some(LogPlace::Bottom) => (Phrase::LogOntoLibraryBottom, None),
            Some(LogPlace::FromTop(n)) if yours => (Phrase::LogIntoLibraryAtYou, Some(n)),
            Some(LogPlace::FromTop(n)) => (Phrase::LogIntoLibraryAt, Some(n)),
            Some(LogPlace::Shuffled) | None => return self.zone(owner, LogZone::Library, true),
        };
        let mut args = vec![self.seat(owner)];
        args.extend(at.map(|n| Piece::plain(ordinal(self.wording.lang, n))));
        self.phrase(phrase, &args)
    }

    /// Where `player` played or cast something from, with the space before
    /// it, or nothing when the line does not say.
    fn played_from(&self, player: PlayerId, from: Option<LogFrom>) -> Piece {
        let Some(LogFrom { zone, owner }) = from else {
            return Piece::default();
        };
        let own = match zone {
            LogZone::Hand if owner == player && owner != self.wording.seat => {
                Some(Phrase::LogFromOwnHand)
            }
            LogZone::Library if owner == player && owner != self.wording.seat => {
                Some(Phrase::LogFromOwnLibrary)
            }
            LogZone::Graveyard if owner == player && owner != self.wording.seat => {
                Some(Phrase::LogFromOwnGraveyard)
            }
            _ => None,
        };
        let mut out = Piece::plain(" ");
        out.push(&own.map_or_else(
            || self.zone(owner, zone, false),
            |own| self.phrase(own, &[]),
        ));
        out
    }

    fn counters(&self, kind: CounterKind, n: u16) -> Piece {
        let n = usize::from(n);
        let (phrase, sizes) = match kind {
            CounterKind::Plus { power, toughness } => {
                (Phrase::LogCountersPlus, Some((power, toughness)))
            }
            CounterKind::Minus { power, toughness } => {
                (Phrase::LogCountersMinus, Some((power, toughness)))
            }
            CounterKind::Loyalty => (Phrase::LogCountersLoyalty, None),
            CounterKind::Lore => (Phrase::LogCountersLore, None),
            CounterKind::Bore => (Phrase::LogCountersBore, None),
            CounterKind::Shield => (Phrase::LogCountersShield, None),
            CounterKind::Defense => (Phrase::LogCountersDefense, None),
            CounterKind::Time => (Phrase::LogCountersTime, None),
            CounterKind::Charge => (Phrase::LogCountersCharge, None),
            CounterKind::Poison => (Phrase::LogCountersPoison, None),
            CounterKind::Energy => (Phrase::LogCountersEnergy, None),
            CounterKind::Rad => (Phrase::LogCountersRad, None),
            CounterKind::Lifelink => (Phrase::LogCountersLifelink, None),
            CounterKind::Level => (Phrase::LogCountersLevel, None),
            CounterKind::Hone => (Phrase::LogCountersHone, None),
            CounterKind::Stun => (Phrase::LogCountersStun, None),
            CounterKind::Finality => (Phrase::LogCountersFinality, None),
            CounterKind::Flying => (Phrase::LogCountersFlying, None),
            CounterKind::Shadow => (Phrase::LogCountersShadow, None),
            CounterKind::Indestructible => (Phrase::LogCountersIndestructible, None),
            CounterKind::Custom(10) => (Phrase::LogCountersAge, None),
            CounterKind::Custom(_) => (Phrase::LogCountersOther, None),
        };
        let sizes = sizes.map_or_else(Vec::new, |(power, toughness)| {
            vec![Self::count(&power), Self::count(&toughness)]
        });
        self.counted(phrase, n, &sizes)
    }

    fn defender(&self, defending: Defender) -> (bool, Piece) {
        match defending {
            Defender::Player(player) => (player == self.wording.seat, self.seat(player)),
            Defender::Planeswalker(id) => (
                false,
                self.defenders.get(&id).map_or_else(
                    || self.neutral(Phrase::LogAPlaneswalker),
                    |named| self.known(id, named.card, None, &named.name),
                ),
            ),
            Defender::Battle(id) => (
                false,
                self.defenders.get(&id).map_or_else(
                    || self.neutral(Phrase::LogABattle),
                    |named| self.known(id, named.card, None, &named.name),
                ),
            ),
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per event, and no `_`
    fn sentence(&self, entry: &LogEntry) -> Piece {
        match &entry.event {
            LogEvent::TurnStarted { active } => self.about(
                *active,
                Phrase::LogTurnYou,
                Phrase::LogTurn,
                vec![Self::count(&entry.turn)],
            ),
            LogEvent::Mulliganed { player } => self.about(
                *player,
                Phrase::LogMulliganYou,
                Phrase::LogMulligan,
                Vec::new(),
            ),
            LogEvent::Kept { player, cards } => {
                let n = usize::from(*cards);
                self.about_counted(
                    *player,
                    Phrase::LogKeptCardsYou,
                    Phrase::LogKeptCards,
                    n,
                    Vec::new(),
                )
            }
            LogEvent::TimedOut { player, answer } => {
                let (you, other) = match answer {
                    ClockAnswer::Passed => (Phrase::LogTimedPassedYou, Phrase::LogTimedPassed),
                    ClockAnswer::Kept => (Phrase::LogTimedKeptYou, Phrase::LogTimedKept),
                    ClockAnswer::NoAttackers => {
                        (Phrase::LogTimedNoAttackersYou, Phrase::LogTimedNoAttackers)
                    }
                    ClockAnswer::NoBlockers => {
                        (Phrase::LogTimedNoBlockersYou, Phrase::LogTimedNoBlockers)
                    }
                    ClockAnswer::Declined => {
                        (Phrase::LogTimedDeclinedYou, Phrase::LogTimedDeclined)
                    }
                    ClockAnswer::ChosenForThem => {
                        (Phrase::LogTimedChosenYou, Phrase::LogTimedChosen)
                    }
                };
                self.about(*player, you, other, Vec::new())
            }
            LogEvent::StandIn { player } => self.about(
                *player,
                Phrase::LogStandInYou,
                Phrase::LogStandIn,
                Vec::new(),
            ),
            LogEvent::ConnectionLost { player, wait_secs } => match wait_secs {
                Some(secs) => self.about(
                    *player,
                    Phrase::LogConnectionLostYou,
                    Phrase::LogConnectionLost,
                    vec![Piece::plain(format!("{}:{:02}", secs / 60, secs % 60))],
                ),
                None => self.about(
                    *player,
                    Phrase::LogConnectionLostPausedYou,
                    Phrase::LogConnectionLostPaused,
                    Vec::new(),
                ),
            },
            LogEvent::Returned { player } => self.about(
                *player,
                Phrase::LogReturnedYou,
                Phrase::LogReturned,
                Vec::new(),
            ),
            LogEvent::LandPlayed { player, land, from } => self.about(
                *player,
                Phrase::LogLandPlayedYou,
                Phrase::LogLandPlayed,
                vec![self.object(land), self.played_from(*player, *from)],
            ),
            LogEvent::Cast {
                player,
                spell,
                from,
            } => self.about(
                *player,
                Phrase::LogCastYou,
                Phrase::LogCast,
                vec![self.object(spell), self.played_from(*player, *from)],
            ),
            LogEvent::Ability {
                controller,
                source,
                activated,
                ..
            } => {
                let (you, other) = match activated {
                    Some(true) => (Phrase::LogActivatedYou, Phrase::LogActivated),
                    Some(false) => (Phrase::LogTriggeredYou, Phrase::LogTriggered),
                    None => (Phrase::LogAbilityYou, Phrase::LogAbility),
                };
                self.about(*controller, you, other, vec![self.object(source)])
            }
            LogEvent::Countered { spell } => {
                self.about_nobody(Phrase::LogCountered, vec![self.object(spell)])
            }
            LogEvent::DidNotResolve { object } => {
                self.about_nobody(Phrase::LogDidNotResolve, vec![self.object(object)])
            }
            LogEvent::Drew { player, cards } => {
                if cards.iter().all(|card| *card == LogObject::Hidden) {
                    let n = cards.len();
                    self.about_counted(
                        *player,
                        Phrase::LogDrewCardsYou,
                        Phrase::LogDrewCards,
                        n,
                        Vec::new(),
                    )
                } else {
                    self.about(
                        *player,
                        Phrase::LogDrewYou,
                        Phrase::LogDrew,
                        vec![self.objects(cards)],
                    )
                }
            }
            LogEvent::Discarded { player, card } => self.about(
                *player,
                Phrase::LogDiscardedYou,
                Phrase::LogDiscarded,
                vec![self.object(card)],
            ),
            LogEvent::Moved {
                object,
                owner,
                from,
                to,
                place,
            } => {
                let (phrase, into) = match (to, place) {
                    (LogZone::Library, Some(LogPlace::Shuffled)) => {
                        (Phrase::LogMovedShuffled, self.library_place(*owner, *place))
                    }
                    (LogZone::Library, _) => (Phrase::LogMoved, self.library_place(*owner, *place)),
                    _ => (Phrase::LogMoved, self.zone(*owner, *to, true)),
                };
                Piece {
                    subject: Some(*owner),
                    ..self.about_nobody(
                        phrase,
                        vec![self.object(object), self.zone(*owner, *from, false), into],
                    )
                }
            }
            LogEvent::Created { object, controller } => self.about(
                *controller,
                Phrase::LogCreatedYou,
                Phrase::LogCreated,
                vec![self.object(object)],
            ),
            LogEvent::Damage {
                source,
                target,
                amount,
                combat,
            } => {
                let (you, victim, subject) = match target {
                    LogTarget::Player(player) => (
                        *player == self.wording.seat,
                        self.seat(*player),
                        Some(*player),
                    ),
                    LogTarget::Object(object) => (false, self.object(object), None),
                };
                let (phrase, rest) = match (source, you, combat) {
                    (Some(source), true, true) => {
                        (Phrase::LogCombatDamageYou, Some(self.object(source)))
                    }
                    (Some(source), true, false) => {
                        (Phrase::LogDamageYou, Some(self.object(source)))
                    }
                    (Some(source), false, true) => {
                        (Phrase::LogCombatDamage, Some(self.object(source)))
                    }
                    (Some(source), false, false) => (Phrase::LogDamage, Some(self.object(source))),
                    (None, true, _) => (Phrase::LogDamageUnsourcedYou, None),
                    (None, false, _) => (Phrase::LogDamageUnsourced, None),
                };
                let mut args = vec![victim, Self::count(amount)];
                args.extend(rest);
                Piece {
                    subject,
                    ..self.phrase(phrase, &args)
                }
            }
            LogEvent::Life { player, old, new } => {
                let n = usize::try_from(*new).unwrap_or(0);
                self.about_counted(
                    *player,
                    Phrase::LogLifePointsYou,
                    Phrase::LogLifePoints,
                    n,
                    vec![Self::count(old), Self::count(new)],
                )
            }
            LogEvent::PlayerCounters {
                player,
                kind,
                old,
                new,
            } => self.about(
                *player,
                Phrase::LogPlayerCountersYou,
                Phrase::LogPlayerCounters,
                vec![
                    Self::count(new),
                    self.counters(*kind, *new),
                    Self::count(old),
                ],
            ),
            LogEvent::Counters {
                object,
                kind,
                old,
                new,
            } => self.about_nobody(
                // A permanent that had none, such as a planeswalker entering
                // with its loyalty, got them: "went to 4 (from 0)" says the
                // same thing as a sum.
                if *old == 0 {
                    Phrase::LogCountersGot
                } else {
                    Phrase::LogCounters
                },
                vec![
                    self.object(object),
                    Self::count(new),
                    self.counters(*kind, *new),
                    Self::count(old),
                ],
            ),
            LogEvent::Attacked {
                attacker,
                defending,
            } => {
                let (you, defender) = self.defender(*defending);
                let phrase = if you {
                    Phrase::LogAttackedYou
                } else {
                    Phrase::LogAttacked
                };
                Piece {
                    subject: match defending {
                        Defender::Player(player) => Some(*player),
                        Defender::Planeswalker(_) | Defender::Battle(_) => None,
                    },
                    ..self.phrase(phrase, &[defender, self.object(attacker)])
                }
            }
            LogEvent::Blocked { blocker, attacker } => self.about_nobody(
                Phrase::LogBlocked,
                vec![self.object(blocker), self.object(attacker)],
            ),
            LogEvent::Banded { attacker, with } => self.about_nobody(
                Phrase::LogBanded,
                vec![self.object(attacker), self.object(with)],
            ),
            LogEvent::ControlChanged { object, new, .. } => self.about(
                *new,
                Phrase::LogControlYou,
                Phrase::LogControl,
                vec![self.object(object)],
            ),
            LogEvent::TurnedFaceUp { object } => {
                self.about_nobody(Phrase::LogTurnedFaceUp, vec![self.object(object)])
            }
            LogEvent::Transformed { object } => {
                self.about_nobody(Phrase::LogTransformed, vec![self.object(object)])
            }
            LogEvent::CardsKept { player, cards } => self.about(
                *player,
                Phrase::LogCardsKeptYou,
                Phrase::LogCardsKept,
                vec![self.objects(cards)],
            ),
            LogEvent::Revealed { player, cards } => self.about(
                *player,
                Phrase::LogRevealedYou,
                Phrase::LogRevealed,
                vec![self.objects(cards)],
            ),
            LogEvent::Shuffled { player } => self.about(
                *player,
                Phrase::LogShuffledYou,
                Phrase::LogShuffled,
                Vec::new(),
            ),
            LogEvent::DiceRolled {
                player,
                sides,
                result,
            } => self.about(
                *player,
                Phrase::LogRolledYou,
                Phrase::LogRolled,
                vec![Self::count(sides), Self::count(result)],
            ),
            LogEvent::CoinFlipped { player, won } => self.about(
                *player,
                if *won {
                    Phrase::LogFlipWonYou
                } else {
                    Phrase::LogFlipLostYou
                },
                if *won {
                    Phrase::LogFlipWon
                } else {
                    Phrase::LogFlipLost
                },
                Vec::new(),
            ),
            LogEvent::Lost { player, cause } => {
                let (you, other) = loss_phrases(*cause);
                self.about(*player, log_loss(*cause).unwrap_or(you), other, Vec::new())
            }
            LogEvent::GameOver { winners } => {
                let seat = self.wording.seat;
                let mut names: Vec<Piece> = winners
                    .iter()
                    .filter(|player| *player != seat)
                    .map(|player| self.seat(player))
                    .collect();
                if winners.contains(seat) {
                    if names.is_empty() {
                        return self.phrase(Phrase::LogWonYou, &[]);
                    }
                    names.push(self.you(Phrase::LogYouInList));
                }
                match names.len() {
                    0 => self.neutral(Phrase::LogDrawn),
                    1 => self.phrase(Phrase::LogWonOne, &names),
                    _ => self.phrase(Phrase::LogWonMany, &[self.list(names)]),
                }
            }
            LogEvent::LoopDetected { broken } => self.neutral(if *broken {
                Phrase::LogLoopBroken
            } else {
                Phrase::LogLoop
            }),
            LogEvent::DayNight { now } => self.neutral(match now {
                DayNight::Day => Phrase::LogDay,
                DayNight::Night => Phrase::LogNight,
            }),
            LogEvent::BecameMonarch { player } => self.about(
                *player,
                Phrase::LogMonarchYou,
                Phrase::LogMonarch,
                Vec::new(),
            ),
            LogEvent::Aimed {
                controller,
                source,
                ability,
                ..
            } => {
                let phrase = if *ability {
                    Phrase::LogAimedAbility
                } else {
                    Phrase::LogAimedSpell
                };
                Piece {
                    subject: Some(*controller),
                    ..self.phrase(
                        phrase,
                        &[Piece::default(), self.object(source), self.aim_parts(entry)],
                    )
                }
            }
        }
    }
}
