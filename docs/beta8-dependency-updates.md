# Paket 1 – Dependencies: Übergabe am 14.10.2026

Branch: `codex/b8-deps`; geprüfter Code-Stand: `6ffd2005db6368d1c701c0ae6866538e570a5360`. Dieser Bericht ergänzt die unveränderte, bereits geprüfte Codebasis. Alle nachstehenden Gates sind lokal; ein CI-Lauf nach dem Push wird nicht behauptet.

MSRV bleibt **Rust 1.97.1** (Manifest vor/nach unverändert); der dokumentierte Workspace-Check mit rustc 1.97.1 ist erfolgreich. Bevy bleibt 0.20. Node bleibt **22.22.1**, Prüfwerkzeug npm **10.9.4**; keine Serveränderung.

## Rust: alle 20 semver-kompatiblen Lockfile-Updates

Commit `eb6582b8d`, verglichen mit seinem Parent `7f6316b25`. Ziel: die vom Resolver innerhalb der bestehenden Ranges für Rust 1.97.1 gewählten Updates; keine pauschalen Manifest-Major-Bumps.

| Paket | Alt → neu |
| --- | --- |
| cc | 1.6.0 → 1.8.0 |
| deranged | 0.5.8 → 0.5.9 |
| equivalent | 1.0.2 → 1.0.3 |
| indoc | 2.0.7 → 2.0.8 |
| keyring-core | 1.0.0 → 1.0.1 |
| litrs | 1.0.0 → 1.0.1 |
| mac_address | 1.2.0 → 1.2.1 |
| parlance | 0.1.0 → 0.1.1 |
| rustls-native-certs | 0.8.4 → 0.8.5 |
| serde_json | 1.0.151 → 1.0.154 |
| sha2 | 0.11.0 → 0.11.1 |
| smallvec | 1.16.2 → 1.16.3 |
| syn | 3.0.6 → 3.0.9 |
| tokio | 1.53.2 → 1.53.3 |
| toml_edit | 0.25.16+spec-1.1.0 → 0.25.17+spec-1.1.0 |
| unicode-ident | 1.0.26 → 1.0.27 |
| uuid | 1.27.0 → 1.28.0 |
| winresource | 0.1.31 → 0.1.34 |
| zerocopy | 0.8.62 → 0.8.63 |
| zerocopy-derive | 0.8.62 → 0.8.63 |

Zusätzlich entfernt: `nix 0.30.1`; die bereits vorhandene `0.31.3` bleibt. Das sind **20 Versionsupdates plus eine Entfernung**, nicht 21 Upgrades. `sha2 0.10.9` und `syn 2.0.119` bleiben für andere Abnehmer im Graphen.

## Sechs direkte inkompatible Versionssprünge

| Paket / Commit | Alt → neu, aufgelöst | Begründung / Kompatibilität |
| --- | --- | --- |
| base64 / `d351d2ec5` | 0.22.1 → 0.23.1 | Bestehende Engines und strikte Signaturdekodierung erhalten; Updater-Suite 84 bestanden, 1 ignoriert. |
| syn / `54531b579` | 2.0.119 → 3.0.9 | Syntaxanalyse auf aktuelle Linie; sechs Mechanics-Coverage-Tests prüfen Imports, Match-Arme und Quellspannen. |
| zip / `74d56044c` | 8.6.0 → 9.0.3 | Fehlerhafte Archivnamen explizit behandeln; Pfad-Traversal-/Symlink-Schutz bleibt; 84 Updater-Tests bestanden. |
| serde-saphyr / `a5d83c4ed` | 1.3.0 → 2.0.0 | Nur Serialize/Deserialize-Features; 47 Deck-I/O-Tests inklusive Aliaslimits und Format-Roundtrips bestanden. |
| windows-sys / `f98fa6b4d` | 0.52.0 → 0.61.2 | Pointer-typisierte HANDLEs mit Null-/Invalid-Prüfung; Konsolenmodul samt Tests für Windows-GNU typgeprüft und nativer Client-Check grün. Windows-Laufzeitprüfung bleibt offen. |
| tungstenite, nativer Client / `6ffd2005d` | 0.24.0 → 0.30.0 | API-Anpassung mit explizitem 4-KiB-Lesepuffer und Frame-Limits; fünf Loopback- und 13 NetworkHost-Tests sowie Client-Clippy grün. |

`serde-saphyr` zieht `granit-parser 1.3.0 → 2.0.0` mit. Die neuen base64-, syn-, windows-sys- und tungstenite-Versionen waren bereits transitiv vorhanden: hier wechseln die **direkten Abnehmer**, alte transitive Linien verschwinden deshalb nicht. Insbesondere behält `ewebsock 0.8.0` sein tungstenite 0.24.0.

## npm: vollständige Versionsänderungen

Commit `1bbbf18fb`. Die Tabelle umfasst alle **74 veränderten Lockfile-Paketpositionen**, mit identischen Plattformänderungen gebündelt: sieben direkte Änderungen und 67 transitive Änderungen/Zu- und Abgänge. Lockfile: 165 → 167 Paketpositionen ohne Root. Exakte direkte Pins, keine Overrides, keine Frontend-Kompatibilitätsänderung.

| Direktes Paket | Alt → neu | Grund |
| --- | --- | --- |
| @playwright/test | 1.63.0 → 1.64.0 | Minor; Browserabweichung unten. |
| @testing-library/user-event | 14.6.7 → 14.6.8 | Patch, bestehende Interaktionstests grün. |
| @types/node | 24.19.0 → 24.19.2 | Patch innerhalb der bestehenden 24er-Typlinie. |
| jsdom | 30.1.1 → 29.1.1 | Gezielter Downgrade für Node 22.22.1. |
| oxlint | 1.85.0 → 1.88.0 | Minor; keine neuen Lintfehler. |
| vite | 8.3.1 → 8.3.4 | Patch; Produktionsbuild grün. |
| vitest | 5.0.2 → 5.0.4 | Patch; 274 bestehende Tests grün. |

| Transitives Paket / Familie | Alt → neu |
| --- | --- |
| @asamuzakjp/css-color | 7.1.2 → 5.1.11 |
| @asamuzakjp/dom-selector | 9.2.2 → 7.1.1 |
| @asamuzakjp/generational-cache | nicht enthalten → 1.0.1 |
| @asamuzakjp/nwsapi | nicht enthalten → 2.3.9 |
| @babel/runtime | 7.29.7 → 7.29.10 |
| @bramus/specificity | 2.4.2 → 2.4.3 |
| @csstools/css-calc | 3.4.1 → 3.4.3 |
| @csstools/css-color-parser | 4.2.4 → 4.2.6 |
| @csstools/css-parser-algorithms | 4.0.1 → 4.0.2 |
| @csstools/css-syntax-patches-for-csstree | 1.1.14 → 1.1.15 |
| @oxc-project/types | 0.151.0 → 0.153.0 |
| @oxlint/binding-* (19 Plattformpakete) | 1.85.0 → 1.88.0 |
| @rolldown/binding-* (15 Plattformpakete) | 1.2.11 → 1.2.13 |
| @vitest/mocker | 5.0.2 → 5.0.4 |
| @vitest/spy | 5.0.2 → 5.0.4 |
| chai | 6.2.2 → 6.3.0 |
| data-urls/node_modules/whatwg-url | 16.0.1 → entfernt |
| html-encoding-sniffer | 7.0.0 → 6.0.0 |
| magic-string | 1.4.2 → 1.4.3 |
| nanoid | 3.3.19 → 3.3.20 |
| obug | 2.2.1 → 3.0.0 |
| playwright | 1.63.0 → 1.64.0 |
| playwright-core | 1.63.0 → 1.64.0 |
| postcss | 8.5.28 → 8.5.30 |
| rolldown | 1.2.11 → 1.2.13 |
| source-map-js | 1.2.1 → 1.2.2 |
| std-env | 4.2.0 → 4.3.0 |
| symbol-tree | nicht enthalten → 3.2.4 |
| tinybench | 6.2.0 → 6.2.2 |
| tldts | 7.4.15 → 7.4.18 |
| tldts-core | 7.4.15 → 7.4.18 |
| undici | 8.11.2 → 7.30.0 |
| w3c-xmlserializer | 6.0.0 → 5.0.0 |
| whatwg-url | 17.1.2 → 16.0.1 |
| why-is-node-running | 3.2.2 → 3.2.1 |

Die Downgrades folgen der normalen jsdom-29-Auflösung; `obug 2 → 3` kommt über Vitest, `why-is-node-running 3.2.1` ist dessen exakter Pin. Die entfernte verschachtelte whatwg-url-Kopie wird vom gemeinsamen Paket ersetzt. React/React DOM bleiben 19.3.0; die Browser-Laufzeitabhängigkeiten wurden nicht aktualisiert. npm 10.9.4 ist das verwendete Prüfwerkzeug, kein behaupteter CLI-Versionssprung im Repository.

## Bewusst zurückgestellt / Grenzen

- `glam 0.33.12 → 0.34.1` und `accesskit 0.24.1 → 0.25.1`: bei den mit Bevy 0.20 verwendeten Typ-/Integrationslinien bleiben; kein isolierter Math-/Accessibility-Major. `ort =2.0.0-rc.13` bleibt bewusst gepinnt, rc.14 wird nicht erzwungen. Die bereits aufgelösten `winresource 0.1.34` und `objc2 0.6.5` erfordern keine kosmetische Erhöhung ihrer kompatiblen Manifest-Untergrenzen.
- `ruzstd 0.8.3 → 0.9.1`: Bevy Image erlaubt weiterhin nur 0.8.x. Die vorhandene Ausnahme **RUSTSEC-2026-0344** bleibt für die eingebetteten vertrauenswürdigen LUTs bestehen; kein behobenes Advisory behaupten. Auch die vorhandene ttf-parser-Ausnahme **RUSTSEC-2026-0192** bleibt unverändert.
- `jsdom 30.1.2`, `@asamuzakjp/css-color 7.1.5`, `@asamuzakjp/dom-selector 10.0.2` und `w3c-xmlserializer 6.0.0` verlangen `^22.22.2 || ^24.15.0 || >=26.0.0` und schließen **22.22.1** aus. Bereits der Ausgangsbaum mit jsdom 30.1.1 verletzte diese Grenze. jsdom 29.1.1 samt normaler transitiver Auflösung korrigiert das; kein engine-strict-Bypass.
- `@types/node 26.6.5` bleibt aus, um nicht zusätzlich die Typ-API zu erweitern; die bisherige 24er-Linie bleibt. Andere npm-Angebote außerhalb der Abhängigkeits-Ranges/exakten Pins werden nicht per Override erzwungen: generational-cache, Babel-Pakete, @oxc-project/types, ansi-regex/-styles, aria-query, data-urls, dom-accessibility-api, es-module-lexer, html-encoding-sniffer, js-tokens, mdn-data, nanoid, pretty-format, react-is, tr46, undici/-types, whatwg-url und why-is-node-running. Auch css-color 6 / dom-selector 8 wären nicht innerhalb der jsdom-29-Ranges. Vollständige ausgeschlossene Versionen/Engines stehen im npm-Einzelbericht.

## Lokale Prüfungen und verbleibende Einschränkungen

Die abschließenden vier Gate-Ergebnisse stammen aus `/private/tmp/baylee-b8-final-results.json` (Laufzeiten des Wrappers, nicht isolierte Testzeiten):

| Gate | Ergebnis | Dauer |
| --- | --- | --- |
| `cargo deny check` | Exit 0, bestanden | 31,5 s |
| `scripts/gate.sh` | Exit 0, bestanden | 599,1 s |
| `scripts/gate-features.sh` | Exit 0, bestanden | 593,2 s |
| `scripts/gate-wasm.sh` | Exit 0, bestanden | 57,8 s |

Das Hauptgate meldet **14.349/14.349 Tests bestanden, 21 übersprungen**, Testphase 335,575 s. cargo-deny ist mit den bestehenden Ausnahmen grün; Warnungen über Mehrfachversionen/unbenutzte Lizenzfreigaben bleiben. Der abschließende separate `cargo-audit 0.22.2`-Lauf ist ebenfalls mit **Exit 0** belegt (2,483 s): 902 Abhängigkeiten, 1.310 Advisories, 0 nicht ausgenommene Schwachstellen und keine Warnungen. Es gelten ausschließlich die beiden bestehenden Ausnahmen oben. Die Advisory-Datenbank wurde aktualisiert; Commit `6aed84668c23d7f535077a9fe918bf1fa0f6a5bc` vom 13.10.2026. [Audit-Nachweis](perf-data/beta8-dependency-audit.json).

npm: **129 Node-Engine-Deklarationen einschließlich Root, 0 inkompatibel** mit Node 22.22.1; frisches `npm ci --include=dev --engine-strict` und `npm run check` bestanden (Typecheck, Lint, 274 Tests in 15 Dateien, Build), `npm audit`: 0 bekannte Schwachstellen. Diese Ergebnisse gelten für die geprüfte macOS-arm64-Installation; optionale Plattformpakete sind im Engine-Scan enthalten.

Playwright: **14/14 bestanden, 0 fehlgeschlagen/übersprungen, 33,7 s**, ein Worker, keine Retries, Screenshot-Test aktiv; abschließender Lauf gegen frisch gebaute Gateway-/Feedback-Binaries. **Browsergrenze:** Playwright 1.64.0 erwartet Chromium **156.0.8078.4 / Revision 1248**; getestet wurde vorhandenes Chromium **153.0.8010.12 / Revision 1243**, weil der passende Download nicht erreichbar war. Der Lauf mit Chromium 156 ist **nicht bestätigt**. Loopback-Sandbox, minimale Umgebung, Kartenbilder durch lokale Fixture; keine echten Modell-/externen API-Aufrufe. Kein Test abgeschwächt. Die nicht fatale jsdom-scrollTo-Meldung bleibt.

Änderungsumfang: Cargo-Manifeste/-Lockfile; enge Kompatibilitätsanpassungen in `baylee-update/src/archive.rs`, `baylee-client/src/console.rs` und `transport.rs` samt fünf nativen Transporttests; `web/feedback/package.json`, `package-lock.json`, `README.md`. Keine Engine-Regeländerung, kein Bevy-Major, kein Server-Node-Update.

Belege: `/private/tmp/baylee-b8-final-results.json` und zugehörige final-Logs; `/private/tmp/baylee-b8-msrv.log`; npm-Details, komplette Deferred-Tabelle und Browser-/Binary-Nachweise in `/private/tmp/baylee-b8-npm-report.md`. Versionslisten wurden ausschließlich aus den Git-Blobs der alten/neuen Manifeste und Lockfiles erzeugt.
