# Když se síť rozdělí

Když selže spoj a obě strany nespojuje žádná jiná cesta, síť hkfree.org se
rozpadne na části. Část, která zůstane bez výstupu do internetu, dosáhne na
všechno uvnitř sebe (často i na server hkfree.org), ale ne do internetu. OSPF
na té straně přestane instalovat výchozí trasu. Členové tam vidí „hkfree.org
funguje, internet ne“, což by mylně ukazovalo na výstup do internetu.

## Jak to aplikace pozná

- **Vlastní část sítě každého routeru.** Engine najde část sítě, do které
  patří router člena. Když ta část nemá výstup do internetu, síť se rozdělila.
  Už nezáleží na tom, ze kterého routeru se mapa četla: když se sám FRR
  (10.107.3.1) ocitne v části bez výstupu, členové na zdravé straně se jako
  odříznutí neoznačí.
- **Kde se přerušila.** Oba konce selhaného spoje ho z OSPF vyřadí, takže spoj
  prostě zmizí. Server si týden pamatuje každý spoj, který viděl (ve svém
  souboru se stavem, vedle historie prefixů), a mapa uvádí spoje, které
  zmizely (`lost_links`). Spoje ztracené na okraji části člena jsou místo, kde
  se síť přerušila.
- **Co člen uvidí.** Když server odpovídá, internet ne a mapa ukazuje
  rozdělení: *Vaše část sítě je odříznutá od internetu. Síť se rozdělila na
  spoji mezi A a B …*, s radou nic nerestartovat. Výpadek se nahlásí jako
  výpadek páteře. Přes mobilní data totéž ukáže Analýza cesty domácí adresy.
- **Samotný router nebo žádný výstup.** Router bez jediného funkčního spoje je
  *odpojený*. Když výchozí trasu nikde neohlašuje žádný router, zjištění zní
  *žádný výstup do internetu*.

## Hranice

- Když je server hkfree.org na druhé straně, aplikace pracuje s mapou uloženou
  dřív, která rozdělení ještě neukazuje. Kde se provoz zastavil, pak řeknou
  testy ze zařízení (*dostupné až po A, B neodpovídá*).
- Výstup, který výchozí trasu ohlašuje vždy (`default-information originate
  always`), ji ohlašuje i tehdy, když nefunguje jeho vlastní připojení. To OSPF
  ukázat neumí; pozná to jen test internetu ze zařízení.
- Pohled z více routerů to zpřesní: viz [routery MikroTik](../how-to/add-mikrotik-routers.md).
  Každý z nich hlásí, jestli má výchozí trasu.
