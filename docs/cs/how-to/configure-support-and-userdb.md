# Návod: nastavení kontaktu na podporu a UserDB

## Kontakt na podporu

Aplikace ukáže kontakt pod výsledky, kde je problém v síti (AP router,
páteř, známý problém, žádné spojení, nefunkční internet). Přidejte ho do
konfigurace serveru a službu restartujte:

```toml
[support]
name = "Podpora hkfree.org"
url = "https://hkfree.org"     # musí začínat https://
email = "podpora@hkfree.org"   # prostá adresa, bez ?subject= apod.
phone = "608 10 10 70"         # číslice, mezery, + - ( )
```

S neplatnou hodnotou server nenastartuje. Kontakt je součástí
`topology.json`, takže ho nainstalované aplikace ukazují i bez připojení.
Bez `[support]` aplikace napíše *Kontaktujte správce svého AP nebo podporu
hkfree.org*.

> Specifikace počítá ještě s provozní dobou telefonu (pracovní dny 9–17 h) a
> kanálem `#nejdeto` na Slacku hkfree.org; konfigurace je zatím neumí
> (`features/kontrola/kontrola-pripojeni.feature`, `@backlog`).

## UserDB (jména routerů a AP)

S API klíčem ukazuje mapa sítě jména AP („oblast / AP“) místo holých adres
routerů.

```toml
[userdb]
api_key_id = 123          # uživatel HTTP je apikey123
api_key = "…"             # nebo proměnná prostředí HKDIAG_USERDB_API_KEY
```

- Pro jména se používají jen adresy infrastruktury (patřící AP). Zařízení
  členů se do mapy sítě nikdy nedostanou.
- Adresa musí být `https://`; přihlašovací údaje jdou s každým dotazem.
- Pro `hkdiag diagnose --member` nastavte i `uid` a `password` (nebo
  `HKDIAG_USERDB_PASSWORD`). UserDB po 5 neúspěšných přihlášeních během 15
  minut adresu zablokuje, proto hkdiag přihlášení nikdy neopakuje.
