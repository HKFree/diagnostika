# language: cs
@id:udaje-o-ap
Požadavek: Údaje o AP
  Člen chce vědět, přes který AP se připojuje, kde je a kdo se o něj stará, jak
  dobrý signál má jeho anténa a jestli dohled sítě vidí na jeho AP problém, aby
  odlišil problém u sebe od problému na AP. Údaje pocházejí z evidence členů
  hkfree.org; údaje o zařízení člena vidí jen on sám a správci.

  Pravidlo: Člen vidí, přes který AP se připojuje

    Scénář: Člen vidí jméno, oblast a adresu svého AP
      Pokud adresa "10.107.10.146" patří k AP "Podůlšany" v oblasti "Podůlšany" na adrese "Podůlšany 1, 533 45 Podůlšany"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí, že se připojuje přes AP "Podůlšany" v oblasti "Podůlšany" na adrese "Podůlšany 1, 533 45 Podůlšany"

    Scénář: Evidence členů AP nepojmenuje
      Pokud AP router "10.107.77.1" člena "10.107.77.5" evidence členů nepojmenuje
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí svůj AP router "10.107.77.1" bez jména AP

  Pravidlo: Člen na požádání vidí rádiový signál své antény
    Zjistit signál znamená přihlásit se do routerů AP, takže se to děje jen
    na žádost člena, nejvýš jednou za minutu pro jeden AP, a člen dostane jen
    signál své vlastní antény.

    Scénář: Člen vidí signál své antény
      Pokud anténa člena "10.107.10.146" přijímá AP se signálem -67 dBm
      Když si člen vyžádá rádiový signál své antény
      Pak vidí signál -67 dBm hodnocený jako dobrý

    Scénář: Slabý signál
      Pokud anténa člena "10.107.10.146" přijímá AP se signálem -84 dBm
      Když si člen vyžádá rádiový signál své antény
      Pak vidí signál -84 dBm hodnocený jako slabý
      A dostane radu zkontrolovat, že anténa míří na AP a nic jí nestojí v cestě

    Scénář: Routery AP anténu člena nevidí
      Pokud routery AP nevidí anténu člena "10.107.10.140"
      Když si člen vyžádá rádiový signál své antény
      Pak vidí, že jeho anténa na AP nalezena nebyla

    Scénář: Jen vlastní anténa člena
      Pokud anténa člena "10.107.10.146" přijímá AP se signálem -67 dBm
      Když se zařízení s adresou "10.107.10.140" zeptá serveru hkfree.org na signál své antény
      Pak signál antény "10.107.10.146" nedostane

  Pravidlo: Člen vidí, co dohled sítě hlásí na jeho AP
    Dohled sítě sleduje vlastní zařízení AP (routery, spoje, sektory), nikdy
    zařízení členů. Zařízení, o kterém neslyšel týden, se bere jako vyřazené
    a vynechá se.

    Scénář: Dohled hlásí ztráty paketů na AP routeru
      Pokud dohled sítě hlásí ztráty paketů na AP routeru "10.107.10.149" člena "10.107.10.146"
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí, že dohled sítě hlásí ztráty paketů na "10.107.10.149"

    Scénář: Zařízení AP dohledu neodpovídá
      Pokud dohled sítě hlásí, že zařízení "sektor jih" s adresou "10.107.10.150" na AP člena "10.107.10.146" neodpovídá
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí, že dohled sítě hlásí, že "sektor jih" "10.107.10.150" neodpovídá

    Scénář: Dohled na AP žádný problém nevidí
      Pokud dohled sítě vidí všechna zařízení AP člena "10.107.10.146" v pořádku
      Když člen spustí kontrolu a otevře pokročilou diagnostiku
      Pak vidí, že dohled sítě na jeho AP žádný problém nevidí
