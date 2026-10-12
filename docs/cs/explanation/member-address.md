# Jak aplikace zjistí adresu člena, a kdy to nejde

Všechno, co aplikace říká o „vašem připojení“ (váš AP router, vaše cesta,
odznak hkfree.org / jiná síť, hlášení výpadků), závisí na jednom údaji:
adrese člena v hkfree.org. Prohlížeč ji sám nevidí. Telefon má doma obvykle
soukromou adresu od routeru člena (třeba `192.168.1.23`), kdežto hkfree.org
zná člena podle adresy toho routeru. Aplikace se proto ptá serveru:
`GET /api/whoami` vrátí adresu, **ze které dotaz přišel**.

Aplikace pak rozhodne:

| whoami vrátí | Mapa sítě adresu zná | Aplikace ukáže | „Vaše adresa“ |
|---|---|---|---|
| adresu IPv4 | ano | **hkfree.org** | tu adresu (zapamatuje si ji) |
| adresu IPv4 | ne | **jiná síť** | naposledy zapamatovanou adresu v hkfree.org |
| adresu IPv6 | pokrývá ji prefix z OSPFv3 | **hkfree.org** | naposledy zapamatovanou adresu IPv4 (cesty jsou IPv4) |
| adresu IPv6 | ne | **jiná síť** | naposledy zapamatovanou adresu v hkfree.org |
| nic | – | **síť ?** (neznámá) | naposledy zapamatovanou, nebo se zeptá |
| server nedostupný | – | **síť ?** | naposledy zapamatovanou (kontrola bez připojení) |

Adresa zadaná ručně (v pokročilé diagnostice) má vždy přednost před zjištěnou.

## Kdy se to pozná těžko

### Server to nepozná vůbec

- **Testování na vlastním počítači.** Na `127.0.0.1` vidí server localhost.
  Záměrně odpoví „neznámá“, ne `127.0.0.1`, a aplikace se na adresu zeptá.
- **IPv6.** Server čte prefixy IPv6 hkfree.org z OSPFv3 (ospf6d na routeru
  „hive“, 10.107.31.33:2606, nastavené jako `frr.ospf6_host`), takže zařízení
  s IPv6 aplikace rozpozná jako v hkfree.org (nebo ne) a jeho cestu najde přes
  router, který jeho prefix inzeruje. Když ospf6d přestane odpovídat, server
  si nechá naposledy přečtené prefixy a zkusí to znovu za 5 minut. Mapa úplně
  bez prefixů IPv6 ukáže zařízení s IPv6 jako neznámá, ne jako jinou síť.
- **Proxy, která neřekne, kdo je klient.** Za reverzní proxy server věří jen
  hlavičce určené `client_ip_header` (poslední položce `X-Forwarded-For`, nebo
  `X-Real-IP`) a jen od `trusted_proxies`. Když proxy použitelnou adresu
  nepošle, odpověď je „neznámá“, ne adresa proxy.
- **Statické soubory bez whoami.** Bez řádku `/api/whoami` v nginx adresu nic
  neřekne; člen ji jednou zadá.

### Server odpoví, ale odpověď klame

- **Proxy, která není v `trusted_proxies`.** Server hlásí u všech členů adresu
  proxy. Všichni pak vidí stejnou cestu.
- **Budoucí doména s HTTPS na veřejné adrese.** Když se členové zevnitř
  hkfree.org dostanou k aplikaci přes veřejnou adresu a jejich provoz projde
  NATem hkfree.org nebo výstupem do internetu, server vidí adresu NATu nebo
  výstupu, ne člena. Pak buď nikdo není „v hkfree.org“, nebo mají všichni
  cestu výstupu. Řešení: doména musí členům uvnitř ukazovat na adresu lide v
  hkfree.org (rozdělené DNS), nebo musí proxy před službou vidět skutečné
  adresy členů v hkfree.org.
- **VPN hkfree.org.** Člen připojený přes VPN hkfree.org dostane adresu z
  `10.107.254.0/24` nebo `2a01:168:0:dead::/64`. Server tyto rozsahy zná
  (`[network] vpn`), takže aplikace ukáže **VPN hkfree.org**: kontrola testuje
  spojení přes VPN, první router je server VPN (bez názvu AP, správce a signálu
  antény) a výpadky VPN se nehlásí jako výpadky domácí přípojky. Domácí adresa
  zůstává zapamatovaná, takže Analýza cesty i kontrola bez připojení dál
  ukazují domácí přípojku, kterou odsud otestovat nejde.
- **Víc členů za jednou adresou.** Domovní síť nebo sdílený router za jednou
  adresou v hkfree.org: všichni dostanou cestu té adresy, což platí jen pro
  sdílenou část.
- **Na návštěvě u jiného člena.** Na Wi-Fi kamaráda vidí server jeho adresu.
  Aplikace si ji zapamatuje jako „vaši“, dokud nespustíte kontrolu znovu doma.

### Volby samotného člena

- **Ručně zadaná adresa zůstává.** Přebije zjištěnou adresu všude, i po
  přestěhování nebo na mobilních datech, dokud v pokročilé diagnostice
  nezvolíte **Zjistit automaticky**.
- **Mobilní data nebo jiný poskytovatel.** Server vidí adresu poskytovatele,
  kterou mapa nezná, takže aplikace ukáže **jiná síť** a ponechá si poslední
  známou adresu v hkfree.org. To je správně, ale ukázaná cesta je pak cesta
  domů, ne ta, kterou právě používáte.

### Bez připojení

- **Bez připojení se aplikace zeptat nemůže.** Použije naposledy zapamatovanou
  adresu. Pokud se zařízení naposledy připojilo jinde (u kamaráda, přes VPN),
  kontrola bez připojení zkoumá cestu toho místa, ne místa, kde člen právě je.

## Co to znamená pro nasazení

- `hkdiag serve` dejte za proxy uvedenou v `trusted_proxies` a čtěte adresu,
  kterou přidá sama proxy: poslední položku `X-Forwarded-For` (Apache, jako na
  lide), nebo `X-Real-IP`, které proxy **přepíše**.
- Zajistěte, aby se členové uvnitř hkfree.org k aplikaci dostali na adrese v
  hkfree.org, ne přes NAT. `https://diag.hkfree.org` se uvnitř sítě překládá
  na 10.107.252.72, takže to platí; `http://10.107.252.72:9000` funguje dál.
- Adresy z VPN při čtení hlášení berte jako „ne domácí přípojka“.
