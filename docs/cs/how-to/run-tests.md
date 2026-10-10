# Návod: spuštění testů a obnova testovacích dat

```
cargo test                                  # Rust: model OSPF, konfigurace, server, hlášení, …
node --test web/test/engine.test.mjs       # JS: jádro v zařízení, překlady, hledání v průvodci
```

(Testovací soubor zadejte výslovně; Node 22 zde adresář nepřijme.)

## Společná testovací data

Obě jádra se testují na stejných datech: `tests/fixtures/snapshot.json` se
generuje ze syntetické databáze OSPF v `tests/fixtures/*.json` rustovým testem
`export::tests::golden_snapshot_is_current`. Když změníte formát mapy sítě
nebo testovací data, obnovte je:

```
HKDIAG_BLESS=1 cargo test golden
```

Pak spusťte obě sady testů. Testy JS musí souhlasit s rustovými (stejná
cesta, stejná místa bez zálohy, stejná zjištění).

## Porovnání Rustu a JS na živé síti

S přístupem k FRR vyexportujte živou mapu sítě a porovnejte cesty pro seznam
adres:

```
hkdiag export --out /tmp/topology.json
hkdiag diagnose --no-probe --json 10.107.77.5 | jq -r '[.router] + [.path[].router] | join(">")'
```

a tutéž adresu přes `analyze()` z `web/engine.js` v Node.

## Specifikace a architektura

```
npx gherkin-lint-plus -c .gherkin-lintrc features   # lint specifikace
(cd docs/architecture && npx likec4 validate .)      # platnost modelu
```

Totéž (a soulad české verze specifikace) kontrolují při pushi hooky v `.githooks/`.

## Závislosti

Na sestavovacím počítači není `cargo audit`. Zamčené balíčky ověřte proti
databázi OSV (obsahuje i RustSec): pošlete dvojice název–verze z `Cargo.lock`
na `https://api.osv.dev/v1/querybatch`.
