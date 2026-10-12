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
phone_hours = "9-17"           # pracovní dny; jde i "8:30-16:30"
slack_channel = "#nejdeto"     # na Slacku hkfree.org
```

S neplatnou hodnotou server nenastartuje. Kontakt je součástí
`topology.json`, takže ho nainstalované aplikace ukazují i bez připojení.
Bez `[support]` aplikace napíše *Kontaktujte správce svého AP nebo podporu
hkfree.org*.


## Měření rychlosti

Pokročilá diagnostika umí změřit rychlost připojení proti backendu LibreSpeed:
odezvu, kolísání, stahování a odesílání. Běží jen na ťuknutí člena, jen přes
hkfree.org, nejvýš 8 s každým směrem a nanejvýš 250 MB dolů a 100 MB nahoru.

```toml
[speedtest]
url = "https://speedtest.hkfree.org/backend/"   # https, končí na /
```

Aplikace volá `empty.php` a `garbage.php` backendu s `?cors=true`, čímž
LibreSpeed pošle potřebné hlavičky CORS; na serveru pro měření rychlosti se nic
jiného měnit nemusí. `getIP.php` nepoužívá (s `isp=true` se ptá vnější služby).

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
  Prosté `http://` projde jen na loopbacku (`127.0.0.1`, `localhost`, `[::1]`),
  kvůli zástupné UserDB v testech.
- Pokročilá diagnostika ukazuje členův AP: název, oblast a adresu. S klíčem
  vázaným na jeden AP se pojmenují jen routery toho AP.
- **Správci AP.** Správce oblasti (SO) a zástupce (ZSO) se ukážou nad
  kontaktem na podporu. Jejich přezdívky jsou v `topology.json`, e-maily
  nikdy. E-maily dá zařízení `/api/contact`, a to jen pro AP, ke kterému
  patří jeho vlastní adresa (adresa spojení, nebo `X-Real-IP` od důvěryhodné
  proxy). Aplikace se ptá při každé kontrole v hkfree.org a odpověď si
  pamatuje, takže e-mail má po ruce, i když síť později selže.
- **Dohled sítě v přehledu výpadků.** Každých `[web] status_poll_secs`
  (60 s) server přečte stav z UserDB (`status/get-problemove-ip`): zařízení AP,
  která neodpovídají, ztrácejí pakety nebo odpovídají pomalu, v celé síti.
  Karta Výpadky je ukáže pod *právě teď* a server si problém každého zařízení
  zaznamená jako výpadek se začátkem a koncem (90 dní, vedle hlášení výpadků),
  takže karta uvádí i výpadky, které dohled sítě naměřil za poslední týden.
  Jde jen o infrastrukturu AP, žádná zařízení členů.
- **Signál antény.** Na žádost člena ukáže pokročilá diagnostika, jak silně
  jeho anténa přijímá AP (dobrý od -70 dBm, použitelný do -80, pod tím
  slabý). Server ho čte přes Wewimo v UserDB, které se přihlásí do všech
  routerů AP zařazených do Wewima a nechá jim řádek v logu. Proto se jeden AP
  čte nejvýš jednou za `[web] signal_cache_secs` (60 s) a vždy jen jeden AP
  naráz. Zařízení dostane jen svou anténu: stanici, jejíž poslední IP je jeho
  adresa, nebo adresa, kterou evidence členů přiřazuje témuž členovi.
- Pro `hkdiag diagnose --member` nastavte i `uid` a `password` (nebo
  `HKDIAG_USERDB_PASSWORD`). UserDB po 5 neúspěšných přihlášeních během 15
  minut adresu zablokuje, proto hkdiag přihlášení nikdy neopakuje.
