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
| nic, nebo IPv6 | – | **síť ?** (neznámá) | naposledy zapamatovanou, nebo se zeptá |
| server nedostupný | – | **síť ?** | naposledy zapamatovanou (kontrola bez připojení) |

Adresa zadaná ručně (v pokročilé diagnostice) má vždy přednost před zjištěnou.

## Kdy se to pozná těžko

### Server to nepozná vůbec

- **Testování na vlastním počítači.** Na `127.0.0.1` vidí server localhost.
  Záměrně odpoví „neznámá“, ne `127.0.0.1`, a aplikace se na adresu zeptá.
- **IPv6.** Zařízení se k serveru dostane přes IPv6. Mapa sítě zná jen IPv4,
  takže to aplikace bere jako neznámé. Pomůže jen cesta k serveru přes IPv4
  (nebo zadání adresy).
- **Proxy, která neřekne, kdo je klient.** Za reverzní proxy server věří jen
  hlavičce `X-Real-IP` a jen od `trusted_proxies`. Když ji proxy nepošle,
  nebo pošle dvě, odpověď je „neznámá“, ne adresa proxy.
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
  `10.107.254.0/24`. Mapa ten rozsah zná, takže aplikace ukáže „hkfree.org“,
  ale cesta je cesta serveru VPN, ne domácí přípojky člena. Domácí připojení
  přes VPN diagnostikovat nejde.
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

- `hkdiag serve` dejte za proxy uvedenou v `trusted_proxies`, která
  `X-Real-IP` **přepíše** skutečnou adresou klienta.
- Zajistěte, aby se členové uvnitř hkfree.org k aplikaci dostali na adrese v
  hkfree.org, ne přes NAT. Dnes je to `http://10.107.252.72:9000`; tak to
  zachovejte i pro doménu s HTTPS.
- Adresy z VPN při čtení hlášení berte jako „ne domácí přípojka“.
