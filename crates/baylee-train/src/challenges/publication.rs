use anyhow::{Result, ensure};
use central_api::challenges::{Certificate, Description, Period, Publication, Witness, Words};

use super::{Certified, Kind};

impl Certified {
    /// Bind proved execution material to a canonical Central calendar slot.
    ///
    /// # Errors
    /// Invalid slot, kind mismatch or publication admission failure.
    pub fn publication(&self, period: Period) -> Result<Publication> {
        period.validate()?;
        ensure!(
            period.kind == self.kind,
            "candidate cadence differs from its slot"
        );
        let words = |en: &str, de: &str| Words {
            en: en.into(),
            de: de.into(),
        };
        let (title, instructions, minutes, hints) = match self.kind {
            Kind::Daily => (
                words("Crossroads", "Am Scheideweg"),
                words(
                    "Win from the supplied position. Your opening hand and both starting battlefields are fixed. Several resource choices lead to victory. Every retry has the same draw order and opponent policy. Pausing is allowed; interrupted games do not enter the sprint ranking.",
                    "Gewinne aus der vorgegebenen Stellung. Deine Starthand und beide Spielfelder sind festgelegt. Mehrere Kombinationen deiner Ressourcen führen zum Sieg. Jeder neue Versuch hat dieselbe Ziehreihenfolge und Gegnerstrategie. Pausen sind erlaubt; unterbrochene Partien zählen nicht für die Sprint-Rangliste.",
                ),
                10,
                [
                    words(
                        "Compare the opponent's life with the damage available in your hand and on the battlefield. Your own low life makes a wasted turn dangerous.",
                        "Vergleiche die gegnerischen Lebenspunkte mit dem Schaden aus deiner Hand und vom Spielfeld. Deine wenigen Lebenspunkte machen einen verlorenen Zug gefährlich.",
                    ),
                    words(
                        "Direct damage can be combined. Increasing a creature's power also increases the damage from sacrificing it to Fling. Alternatively, remove the blocker and attack.",
                        "Direkten Schaden kannst du kombinieren. Mehr Stärke erhöht auch den Schaden, wenn du die Kreatur für Fling opferst. Alternativ kannst du den Blocker entfernen und angreifen.",
                    ),
                ],
            ),
            Kind::Weekly => (
                words("The long game", "Das lange Spiel"),
                words(
                    "Win a full duel with the supplied generated deck. Manage development, combat and removal across several turns. The shuffle and opponent are identical for everyone. You may pause and return; interrupted games keep completion and turn scores but have no sprint time.",
                    "Gewinne ein vollständiges Duell mit dem vorgegebenen generierten Deck. Plane deinen Aufbau, Kämpfe und Entfernungszauber über mehrere Züge. Mischung und Gegner sind für alle gleich. Du kannst pausieren und zurückkehren; unterbrochene Partien behalten Abschluss und Zugwertung, aber keine Sprint-Zeit.",
                ),
                45,
                [
                    words(
                        "Read your supplied deck before committing your removal. Consider which threats you can race and which must be answered.",
                        "Lies dein vorgegebenes Deck, bevor du Entfernungszauber verbrauchst. Überlege, welche Bedrohungen du im Schadensrennen schlagen kannst und welche du beantworten musst.",
                    ),
                    words(
                        "The certified routes include both combat and direct-damage finishes. Preserve reach when the opponent's remaining life is within range, and trade creatures when it buys the turn you need.",
                        "Die geprüften Lösungswege enden sowohl durch Kampf als auch durch direkten Schaden. Hebe direkten Schaden auf, wenn die gegnerischen Lebenspunkte in Reichweite kommen, und tausche Kreaturen, wenn dir das den nötigen Zug verschafft.",
                    ),
                ],
            ),
        };
        let mut publication = Publication {
            description: Description {
                period,
                title,
                instructions,
                deck: self.deck.clone(),
                executor: self.package.executor.clone(),
                target_turn: self.package.target_turn,
                expected_minutes: minutes,
            },
            digest: [0; 32],
            package: self.package.encode()?,
            certificate: Certificate {
                validator: "baylee-contributors-v1".into(),
                witnesses: self
                    .proofs
                    .iter()
                    .map(|proof| Witness {
                        record_digest: proof.record_digest,
                        strategy_digest: proof.strategy_digest,
                        family: proof.family,
                    })
                    .collect(),
            },
            hints,
        };
        publication.digest = publication.calculated_digest()?;
        publication.validate()?;
        Ok(publication)
    }
}
