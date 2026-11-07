# Ansichten als Änderungen statt als ganzes Brett

Stand 7. November 2026, Entwurf. Noch nichts davon ist gebaut. Der Eigentümer hat entschieden, ans
Protokoll zu gehen; Kompatibilität wird nicht gebraucht, solange niemand baylee.app nutzt.

## Heute

Nach jeder Aktion bekommt jeder Sitz seine ganze Ansicht, als JSON in einem Protobuf-Rahmen. Eine
Ansicht mit 25 Objekten sind rund 10,8 KB, also gut 400 Byte je Objekt, und fast alle Objekte sind
unverändert. Das Schreiben kostet auf dem Server, das Lesen kostet auf dem Client, und beides zahlt
jeder Sitz bei jedem Schritt. `WrittenObjects` spart das Schreiben, nicht die Bytes und nicht das
Lesen.

Bereits gut: Das Gateway liest die Frames nicht, gedruckte Karten kommen einmal als eigener Rahmen
(`game_static_envelope`), und die Engine baut je Sitz eine Ansicht, in der Verdecktes nicht
darstellbar ist.

## Das Ziel

Ein Sitz bekommt einmal einen Stand und danach nur noch, was sich geändert hat, binär.

```text
ViewFrame  = Snapshot { seq, view }
           | Patch { seq, base, upserts, removed, fields, digest }
```

- **Der Diff entsteht aus der fertigen Ansicht je Sitz,** nie aus dem Spielzustand. Was im
  `PlayerView` nicht steht, kann in keinem Patch stehen. Die Regel „Verdecktes ist nicht darstellbar“
  bleibt also ohne neue Prüfung gültig.
- **`diff(alt, neu)` und `apply(alt, patch)` leben in `baylee-view`,** an einer Stelle, für Server,
  Client, LLM-Sitz und Netz-Sitz. Alles darüber arbeitet weiter auf einem ganzen `PlayerView`. Der
  Client und die Sitze ändern sich damit kaum.
- **Objekte sind die Einheit:** ein neues oder geändertes Objekt geht ganz, ein gelöschtes als ID,
  die übrigen Felder des Views einzeln. Tiefer zu diffen lohnt erst nach einer Messung.
- **`digest` über die Ansicht** nach dem Patch: Weicht er ab, verlangt der Client einen Snapshot.
  Ein Fehler wird so zu einem unsichtbaren Neuladen statt zu einem falschen Brett.
- **Snapshot bei Beitritt, Wiederverbinden und Lücke in `seq`.** Zuschauer laufen genauso.
- **Binär mit `postcard`** über die vorhandenen Serde-Typen, für Snapshot, Patch und Aktionen des
  Clients. Protobuf bleibt der Rahmen. `bitcode` wird daneben gemessen; `rkyv` scheidet aus, weil es
  fremde Bytes ohne Kopie liest.
- **Danach entfallen** `WrittenObjects` und das JSON im Spielpfad. JSON bleibt für Werkzeuge,
  Aufzeichnungen und `--verify`, wo es nicht im Spiel läuft.

## Sicherheit

Decoder lesen fremde Bytes, auch in der Engine: Das Gateway reicht Frames ungelesen durch. Darum:
harte Grenzen für Längen und Tiefe vor jeder Belegung, kein `unsafe` im Decoder, Fuzz-Tests für beide
Richtungen, und ein Patch, der auf eine unbekannte ID zeigt, führt zum Snapshot, nicht zur Panik.

## Beweis

- Eigenschaftstest: `apply(alt, diff(alt, neu)) == neu` über aufgezeichnete Partien, je Sitz.
- `engine_perf --verify` vergleicht die Ansicht, die ein Empfänger aus Patches aufbaut, mit der
  ganzen Ansicht, Bit für Bit, auf allen Messlasten.
- Vorher und nachher: Bytes je Partie und Sitz, CPU auf dem Server, CPU auf dem Client.

## Reihenfolge

1. **Messen:** Bytes und Frames je Partie heute.
2. **`diff` und `apply`** mit Eigenschaftstests, ohne Änderung am Draht.
3. **Patches im Spielpfad,** noch als JSON, mit Digest und Snapshot.
4. **Binär** für Snapshot, Patch und Aktionen, dann Vergleich `postcard` und `bitcode`.
5. **`WrittenObjects` und JSON-Pfad entfernen,** `VIEW_VERSION` und `docs/protocol.md` nachziehen.
6. **Sicherheitsprüfung** der neuen Decoder, Fuzz-Lauf, Sichtprüfung im Client.

## Offen

- Was Aufzeichnungen und Replays speichern; sie sollten weiter ohne Patches lesbar sein.
- Ob auch das Log denselben Weg nimmt; es wird heute schon inkrementell geschickt.
- Der Gewinn ist geschätzt, nicht gemessen: Bytes je Schritt sollten um eine Größenordnung fallen.
