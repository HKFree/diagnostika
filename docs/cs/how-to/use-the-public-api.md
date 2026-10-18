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
- `since` je výchozí před týdnem a sahá nejvýš 90 dní zpět.

```
curl -s https://diag.hkfree.org/.well-known/api-catalog
curl -s "https://diag.hkfree.org/api/v1/measured?since=$(date -d yesterday +%s)"
```

Součástí API nejsou a z jiných webů se číst nedají odpovědi o samotném
zařízení, které se ptá (`/api/whoami`, `/api/contact`, `/api/signal`).
