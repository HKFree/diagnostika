# Návod: použití veřejného API

hkdiag zveřejňuje, co o síti měří, pro jiné programy: bez přihlášení, čitelné i
z jiných webů (CORS) a bez čehokoli, co identifikuje člena.

- **Kde ho najít:** `GET /.well-known/api-catalog` ([RFC 9727](https://www.rfc-editor.org/rfc/rfc9727))
  odkazuje na popis `GET /api/openapi.json` (OpenAPI 3).
- **Koncové body:**
  - `/api/v1/network`: mapa sítě (routery, spoje, prefixy, výstupy, ztracené
    spoje, zařízení AP se stavem z dohledu).
  - `/api/v1/outages`: hlášení členů a naměřené výpadky za poslední týden, co
    dohled vidí teď a stav napájení.
  - `/api/v1/measured?since=<unixový čas>`: výpadky naměřené dohledem sítě se
    začátkem a koncem (drží se 90 dní).
  - `/api/v1/reports?since=<unixový čas>`: výpadky nahlášené členy, bez jejich
    adres.
  - `/api/v1/speeds?since=<unixový čas>`: rychlosti naměřené členy po AP
    routerech: kolik, obvyklé (medián) a nejnižší stahování, obvyklé
    odesílání a odezva. Bez adres členů; AP se ukáže až od 3 měření.
  - `/api/v1/history?ip=<router>&range=day|week`: odezva a ztráta paketů
    routeru nebo zařízení AP, po 10 minutách za den nebo po hodině za týden.
    Zařízení členů tu historii nemají (404).
- `since` je výchozí před týdnem a sahá nejvýš 90 dní zpět.

```
curl -s https://diag.hkfree.org/.well-known/api-catalog
curl -s "https://diag.hkfree.org/api/v1/measured?since=$(date -d yesterday +%s)"
curl -s "https://diag.hkfree.org/api/v1/history?ip=10.107.10.129&range=week"
```

Součástí API nejsou a z jiných webů se číst nedají odpovědi o samotném
zařízení, které se ptá (`/api/whoami`, `/api/contact`, `/api/signal`,
`/api/traffic`: provoz vlastního připojení zařízení).
