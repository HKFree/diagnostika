# Návod: aplikace přes HTTPS za Apachem

Telefon si aplikaci nainstaluje a zapne provoz bez připojení jen na adrese
HTTPS. Na lide.hkfree.org poběží aplikace za Apachem serveru, který ukončuje
TLS. **Apache hkdiag nenastavuje**: virtuální host zřídí jeho správce. Tady je,
co od něj hkdiag potřebuje, s konfigurací vyzkoušenou proti hkdiag.

## Co hkdiag od proxy potřebuje

| Potřeba | Proč | Apache |
|---|---|---|
| Vlastní jméno hostitele, přeposílané od `/` | Aplikace používá absolutní cesty (`/app.js`, `/topology.json`), takže nemůže běžet v podadresáři | `ProxyPass / …` |
| Adresa klienta v hlavičce, kterou klient nezfalšuje | Podle adresy člena aplikace najde jeho cestu, AP a e-mail správce. Hodnota od klienta by komukoli dovolila vydávat se za kteréhokoli člena | nic: hkdiag čte adresu, kterou Apache připojí do `X-Forwarded-For` (`client_ip_header = "x-forwarded-for"`); nebo `RequestHeader set X-Real-IP "expr=%{REMOTE_ADDR}"` s výchozím `x-real-ip` |
| Původní hlavička `Host`, nebo uvedený veřejný původ | Hlášení výpadků se přijímají jen ze stránek tohoto serveru | `ProxyPreserveHost On`; nebo bez změny Apache `origins = ["https://diag.hkfree.org"]` v `[web]` (jako na lide) |
| Žádná mezipaměť | Každá odpověď si `Cache-Control` nastavuje sama; ukládání řeší service worker | pro tento host nezapínejte `mod_cache` |

hkdiag věří hlavičce s adresou klienta jen od adres v `trusted_proxies`, a jen
té, kterou určuje `client_ip_header`:

- `"x-forwarded-for"`: **poslední** položka `X-Forwarded-For`, kterou sám
  připojí proxy modul Apache. Dřívější položky (i jakékoli `X-Real-IP`) může
  zfalšovat klient, takže se ignorují. Tak to běží na lide: Apache nebylo
  třeba měnit. Předpokládá právě jednu proxy před hkdiag.
- `"x-real-ip"` (výchozí): `X-Real-IP`, které proxy musí **přepsat**. Apache
  vlastní `X-Real-IP` klienta bez nastavení pošle dál beze změny, takže s tímto
  režimem a nenastaveným Apachem by se kdokoli mohl vydávat za kteréhokoli
  člena.

Důvěryhodná proxy, která nepošle použitelnou adresu, způsobí, že adresa není
známá, místo aby všichni členové vypadali jako proxy.

> `RequestHeader set X-Real-IP "%{REMOTE_ADDR}s"`, které se často objevuje na
> internetu, je špatně: v `mod_headers` čte `%{…}s` proměnnou SSL, takže
> hlavička zůstane prázdná a adresa žádného člena není známá.

## Příklad virtuálního hostu

`diag.hkfree.org` zastupuje jméno, které aplikace dostane.

Moduly: `proxy`, `proxy_http`, `headers` (na lide jsou dnes zapnuté jen
`headers` a `ssl`). Certifikát se nastaví jako u ostatních hostů
`*-le-ssl.conf`.

```apache
<VirtualHost *:443>
    ServerName diag.hkfree.org
    # SSLEngine / SSLCertificateFile / SSLCertificateKeyFile: jako ostatní -le-ssl hosty

    ProxyPreserveHost On
    ProxyPass        / http://127.0.0.1:9000/
    ProxyPassReverse / http://127.0.0.1:9000/
    RequestHeader set X-Real-IP "expr=%{REMOTE_ADDR}"

    # Volitelně: jen síť hkfree.org (viz poznámku níže).
    # <Location />
    #     Require ip 10.107.0.0/16
    # </Location>
</VirtualHost>

<VirtualHost *:80>
    ServerName diag.hkfree.org
    Redirect permanent / https://diag.hkfree.org/
</VirtualHost>
```

## Strana hkdiag

hkdiag naslouchá na localhostu pro proxy a může si nechat i adresu v
hkfree.org pro členy, kteří se k němu dostanou přímo. `X-Real-IP` smí
nastavit jen proxy; hlavička poslaná na adresu v hkfree.org se ignoruje:

```toml
[web]
listen = ["127.0.0.1:9000", "10.107.252.72:9000"]
trusted_proxies = ["127.0.0.1", "::1"]
```

Pak z členského zařízení otevřete `https://diag.hkfree.org/api/whoami`: musí
ukázat vlastní adresu zařízení, ne `null` ani `127.0.0.1`. Falešné `X-Real-IP`
poslané přes curl odpověď změnit nesmí.

Poznámky:

- Když má jméno hostitele i záznam AAAA, členové s IPv6 se k němu dostanou
  přes IPv6. Aplikace je pak najde podle jejich prefixu OSPFv3.
- Omezení hostu na `10.107.0.0/16` drží mapu sítě v soukromí, ale členové na
  mobilních datech si pak aplikaci neobnoví. Nainstalovaná kopie jim dál
  funguje bez připojení.
