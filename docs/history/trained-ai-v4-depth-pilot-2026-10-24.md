# Playernet v4: mehr Tiefe, danach Lernen aus eigenen Partien

Der Besitzer wünscht zuerst mehr Layer, anschließend eine erneute Übernahme
bewährter Ideen aus v3: Bewertung, Spiele gegen die Haus-KIs und später
Selbstspiel. Zugleich sollen Trainingsspiele die Haus-KI messbar verbessern.

## Erste Änderung: acht auf zwölf Transformer-Schichten

`deepen4.py` vergrößert nur den räumlichen Trunk. Breite 640, zehn Heads,
FFN 2560, Kartenencoder, Köpfe, Gedächtnis H mit neun Slots und Denkblock
bleiben kompatibel. Neue Schichten liegen an den Indizes 2, 5, 8 und 11;
die alten acht Schichten werden in gleicher Reihenfolge übernommen.
Nullinitialisierte Ausgangsprojektionen der neuen Attention-/FFN-Zweige
machen deren Residualblöcke zunächst zur Identität. Die neuen Blöcke sind
trainierbar; weder das bestehende Wissen noch die Funktionsweise der
persistenten Rückkopplung werden absichtlich verworfen.

Die Idee folgt funktionserhaltendem Netz-Wachstum
([Net2Net, Chen/Goodfellow/Shlens](https://arxiv.org/abs/1511.05641)); die
konkrete Initialisierung nutzt die vorhandenen Pre-Norm-Residualblöcke.
Das ist kein Beleg für eine Stärkeverbesserung. Ein eigener Quellstand,
neuer Optimierer und frische Gedächtnis-/Samplerbahnen kennzeichnen den
Architekturversuch. Alte Checkpoints und gefrorene Experimente bleiben erhalten.
CPU-Tests prüfen den unveränderten Gedächtnisverlauf und alle Aktions-/Value-
Ausgaben bei K=0/1/4/8, unabhängige Gewichte, trainierbare neue Zweige und
Abweisung inkompatibler Checkpoints. CUDA-Speicher und ONNX werden separat
am echten Modell geprüft, bevor längeres Lernen freigegeben wird.

Als Start dient das Referenzmodell vor dem zuletzt schwächeren BC-Kandidaten.
Ein kurzer kontrollierter Versuch vergleicht zwölf und acht Schichten auf
demselben vollständig gemischten 240-Spiele-Bestand, mit identischem
Sampler, B/U, Anzahl Updates und Validierung. Die gleichbleibende BC-Aufgabe
dient hier der Isolation des Tiefeneffekts; sie ist noch keine Umsetzung der
anschließenden Liga. Ganze Gedächtnisverläufe bleiben chronologisch erhalten.
Die Auswertung prüft zusätzliche Denkschritte und echte gepaarte Partien;
kein Modell wird allein wegen niedrigerem Loss oder mehr Parametern übernommen.

## Was sich aus dem alten Modell tatsächlich übertragen lässt

Der geprüfte `model3.py`-/`netplay3.rs`-Pfad besitzt keine neuronale
`h_out -> h_in`-Rückkopplung. Diese ist im aktuellen v4 bereits vorhanden:
Beobachtungen aktualisieren H, spätere Entscheidungen erhalten diesen Zustand.
Die historische Erinnerung des Besitzers kann sich auf einen anderen Entwurf
beziehen; sie ist kein Beleg dafür, dass v4 seinen Speicher erst bekommen muss.
Zusätzliche räumliche Layer vergrößern nicht die TBPTT-Reichweite U.

Das separate frühere `ValueNet` bewertete Stellungen unter Haus-KI-
Fortsetzungen, nicht direkt jede mögliche Alternativaktion. v3 `train_rl.py`
verwendet den eigenen trainierten Value-Head für GAE und PPO auf den
Entscheidungen des Lerners aus Ligapartien gegen Hausprofile, eingefrorene
Netze und sich selbst. Der letzte v4-Lauf lernte dagegen nur aufgezeichnete
Hausaktionen. Diese Trainingsregime dürfen beim Interpretieren der besseren
v3-Spielergebnisse nicht gleichgesetzt werden. Auch GAE ordnet einem Zug
keine beweisbar kausale Güte zu; der Critic kann sich irren.

Nach dem Tiefenpilot: Fehlerzustände aus eigenen v4-Partien sammeln,
Lehrer-/Schülerherkunft trennen und echte terminale Outcomes bewahren.
Den Bewertungskopf auf diesen Zuständen und nach Format/Spielphase prüfen;
ein separater Rater wird erst übernommen, wenn er die Aktionswahl oder das
Training messbar verbessert. Danach kleine on-policy Liga-Iterationen mit
verifizierten Verhaltenswahrscheinlichkeiten, wiederkehrenden Hausgegnern
und historischen Modellen; Selbstspiel ergänzt diese Gegner, es ersetzt
sie nicht vollständig. Ein bloßes weiteres unverändertes BC-Paket erfüllt
diese Stufe nicht.

## Trainingsspiele verbessern auch die Haus-KI

Jeder belastbare Fehlerfund erhält Engine-/KI-Revision, Decks, Seed,
betroffenen Sitz, Profil, Aufzeichnungs-/Replaypfad und Entscheidungscursor.
Zuerst unterscheiden wir Regel-/Enginefehler, unzulässige Antworten,
Kodierungs-/Gedächtnisfehler und schwache legale Spielentscheidungen.
Die gespeicherte Spielaufzeichnung ist für Entwickler bestimmt; Policy und
Hausheuristik dürfen weiterhin nur ihre zulässige `PlayerView` sehen.

Aus dem Fund entsteht ein kleiner deterministischer Regressionstest. Eine
Haus-KI-Änderung wird zusätzlich auf anderen Decks, Seeds und Sitzrotationen
gegen ihren eingefrorenen Vorgänger geprüft; ein einzelner gewonnener
Trainingsfall genügt nicht. Detonate (X-Zielwahl) ist ein bereits behobenes
Beispiel dieses Kreislaufs. Auch verlorene Partien liefern brauchbare Fälle;
die Niederlage allein belegt noch keinen Fehler.

Eine laufende Vergleichssuite behält ihre ursprünglichen Gegner und Engine.
Verbesserte Haus-KIs werden als neue Version in die nächste Liga aufgenommen;
neue Lehrerpartien tragen ihre tatsächliche Herkunft. Geänderte Engine-Regeln
erfordern erneut hashgeprüfte Replays. Damit profitieren Netz und Haus-KI
voneinander, ohne dass ein wechselnder Gegner den Fortschritt verfälscht.
