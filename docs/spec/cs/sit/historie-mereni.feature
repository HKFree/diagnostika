# language: cs
@id:historie-mereni
Požadavek: Historie měření
  Člen nebo správce chce vidět, jak se router na cestě choval za poslední den
  nebo týden: jestli mu rostla odezva nebo ztrácel pakety, a odkdy. Člen chce
  také vidět, kolik jeho vlastní připojení stahovalo a odesílalo, aby odlišil
  problém v síti od linky vytížené doma. Obojí měří dohled sítě. Historie
  routeru je sdílená jako mapa sítě; provoz připojení patří jeho členovi a
  dostane ho jen on.

  Pravidlo: Kdokoli vidí historii měření routerů na cestě
    Router, který na svém ID routeru neodpovídá na ping, se měří na své
    adrese na spoji, kterým do něj cesta vede.

    Scénář: Odezva a ztráta paketů routeru za poslední den
      Pokud dohled sítě naměřil, že router "10.107.76.1" za poslední den odpovídal za 2 ms, s 30 % ztrátou paketů v poslední hodině
      Když si člen zobrazí cestu své adresy "10.107.77.5" a otevře historii routeru "10.107.76.1"
      Pak vidí graf odezvy a ztráty paketů "10.107.76.1" za poslední den
      A graf ukazuje ztrátu paketů až 30 %

    Scénář: Historie routeru za poslední týden
      Pokud dohled sítě naměřil, že router "10.107.76.1" za poslední týden odpovídal za 2 ms
      Když si člen zobrazí cestu své adresy "10.107.77.5" a otevře historii routeru "10.107.76.1" za poslední týden
      Pak vidí graf odezvy a ztráty paketů "10.107.76.1" za poslední týden

    Scénář: Program čte historii routeru
      Pokud dohled sítě naměřil, že router "10.107.76.1" za poslední den odpovídal za 2 ms, s 30 % ztrátou paketů v poslední hodině
      Když se program zeptá API na historii měření routeru "10.107.76.1"
      Pak dostane jeho odezvu a ztrátu paketů za poslední den

    Scénář: Žádná historie měření zařízení člena
      Když se program zeptá API na historii měření adresy "10.107.77.5"
      Pak nedostane žádná měření té adresy

    Scénář: Router, který na svém ID routeru neodpovídá na ping
      Pokud dohled sítě nemůže pingnout router "10.107.99.1" na jeho ID routeru, ale naměřil, že jeho adresa "10.107.0.6" na spoji od routeru "10.107.76.1" za poslední den odpovídala za 3 ms
      Když si člen zobrazí cestu své adresy "10.107.77.5" a otevře historii routeru "10.107.99.1"
      Pak vidí odezvu "10.107.99.1" nejvýše 3 ms, naměřenou na jeho adrese "10.107.0.6"

    Scénář: Program čte historii routeru na jeho adrese na cestě
      Pokud dohled sítě nemůže pingnout router "10.107.99.1" na jeho ID routeru, ale naměřil, že jeho adresa "10.107.0.6" na spoji od routeru "10.107.76.1" za poslední den odpovídala za 3 ms
      Když se program zeptá API na historii měření routeru "10.107.99.1" dosaženého od routeru "10.107.76.1"
      Pak dostane odezvu a ztrátu paketů naměřené na "10.107.0.6"

  Pravidlo: Člen vidí provoz svého připojení, a ničí jiný
    Dohled sítě zaznamenává provoz podle adresy IPv4, takže o něj může
    požádat jen zařízení, které se do hkfree.org dostane přes IPv4.

    Scénář: Člen vidí stahování a odesílání svého připojení
      Pokud dohled sítě naměřil, že připojení člena "10.107.10.146" za poslední den stahovalo až 40 Mbit/s a odesílalo až 5 Mbit/s
      Když člen požádá o provoz svého připojení
      Pak vidí graf svého stahování a odesílání za poslední den, až 40 Mbit/s dolů a 5 Mbit/s nahoru

    Scénář: Jen vlastní provoz člena
      Pokud dohled sítě naměřil, že připojení člena "10.107.10.146" za poslední den stahovalo až 40 Mbit/s a odesílalo až 5 Mbit/s
      Když se zařízení s adresou "10.107.10.140" zeptá serveru hkfree.org na provoz svého připojení
      Pak nedostane provoz "10.107.10.146"

    Scénář: Provoz se ukazuje jen v síti hkfree.org
      Pokud je zařízení člena připojené přes mobilní data
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak nemůže požádat o provoz svého připojení

    Scénář: Provoz se ukazuje jen pro adresu IPv4
      Pokud se zařízení člena dostane do hkfree.org přes IPv6 s adresou "2a01:168:331::5"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak nemůže požádat o provoz svého připojení
