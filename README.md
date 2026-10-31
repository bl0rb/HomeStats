# HomeStats

Haushalts-Tool für Zählerstände, Fahrzeuge, Gefriergut und Finanzen – als einzelne HTML-Datei für den Browser und als Desktop-App für macOS und Windows.

- **Zähler**: frei anlegbar – Strom, Wasser, Gas, Wärme, Tanks und beliebig viele Solaranlagen, als fortlaufender Zählerstand, Menge seit der letzten Ablesung oder Füllstand; Tagesverbrauch, Tank-Reichweite, Kostenschätzung je Zähler, Monats- und Jahresvergleich
- **Fahrzeuge**: beliebig viele KFZ mit Tanken/Laden, Kilometerständen und Kosten (Versicherung, Steuer, Werkstatt …) – Verbrauch, Kosten pro km, Fixkosten, Fälligkeiten (HU, Inspektion, Zahlungen)
- **Kühlvorrat**: Geräte mit Fächern, Einlagern mit Haltbarkeit, „Diese Woche verbrauchen“, QR-Code-Etiketten drucken (Etikettendrucker oder A4-Bogen), Entnehmen per Scan (Kamera, QR-fähiger Handscanner oder Eintippen); wahlweise nur 5-stellige Codes statt QR
- **Finanzen**: Einnahmen und Ausgaben mit Kategorien und wiederkehrenden Buchungen, Fahrzeugkosten automatisch einbezogen, Monats-/Jahresübersicht mit Sparquote; Import von Kontoumsätzen als CSV (Sparkasse, ING, DKB, comdirect, Volksbank u. a.) mit Kategorie-Vorschlägen und Schutz vor Doppelten
- **Einstellungen**: JSON-Sicherung, CSV-Export, Import der alten `energie_verbrauch.csv`

## Browser

`homestats.html` im Browser öffnen (am besten Chrome oder Edge). Die Daten liegen im Browser; in Chrome/Edge kann zusätzlich eine Datei `homestats.json` verknüpft werden, in die jede Änderung geschrieben wird.

## Desktop-App (macOS und Windows)

Die App ([Tauri 2](https://tauri.app)) zeigt dieselbe `homestats.html` in einem eigenen Fenster. Installer gibt es auf der [Release-Seite](https://github.com/bl0rb/HomeStats/releases): `HomeStats-<version>.pkg` für macOS (Apple silicon), `HomeStats-<version>.msi` für Windows.

- **Daten**: jede Änderung wird sofort in `homestats.json` im App-Datenordner gespeichert – macOS `~/Library/Application Support/io.github.bl0rb.homestats/`, Windows `%APPDATA%\io.github.bl0rb.homestats\`. Der vorherige Stand bleibt als `homestats.json.bak` erhalten, dazu eine Tagessicherung im Unterordner `Sicherungen` (30 Tage). Daten aus dem Browser übernimmst du über *Einstellungen → Sicherung einspielen*.
- **Updates**: die App sucht beim Start nach einer neuen Version (abschaltbar, Beta-Versionen optional) und installiert sie auf Wunsch. Installiert wird nur ein Paket, dessen SHA-256 zur Angabe von GitHub passt und das mit dem Release-Schlüssel (`desktop/update-key.pub`, derselbe wie bei KeyZapper und OpenReleasePilot) signiert ist. macOS fragt nach dem Administrator-Passwort und startet die App neu; unter Windows übernimmt der Windows Installer und startet die neue Version.
- **Nicht in der App**: der Kamera-Scan (die WebViews haben keine Barcode-Erkennung) – QR-Handscanner und Eintippen funktionieren.

### Entwicklung

```bash
cd desktop
npm ci
npx tauri dev                    # App mit homestats.html starten
cargo test -p homestats-core     # Datendatei und Update-Prüfung
scripts/build-pkg.sh             # dist/HomeStats-<version>.pkg (macOS)
scripts/build-msi.sh             # dist/HomeStats-<version>.msi (Windows, Git Bash)
```

`homestats.html` bleibt die einzige Quelle: `desktop/scripts/sync-ui.mjs` kopiert sie vor jedem Start/Build nach `desktop/ui/index.html` (nicht eingecheckt). Die Seite erkennt die App an `window.__TAURI__` und nutzt dann die Befehle aus `desktop/src-tauri/src/main.rs` statt Browser-Speicher und Downloads.

### Release

Ein Tag wie `0.2.0` (oder `0.2.0-beta.1` für eine Vorabversion) baut in GitHub Actions beide Installer, signiert sie mit minisign und veröffentlicht das Release. Dafür muss im Repository das Secret `MINISIGN_SECRET_KEY` gesetzt sein (derselbe geheime Schlüssel wie bei den anderen Apps); ohne Secret bricht die Veröffentlichung ab. Die Version im Tag gilt für den Build, `tauri.conf.json` nur für lokale Builds.
