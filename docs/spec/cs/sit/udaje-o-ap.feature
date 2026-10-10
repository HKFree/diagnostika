# language: cs
@id:udaje-o-ap @backlog
Požadavek: Údaje o AP
  Člen chce vědět, přes který AP se připojuje, kde je a kdo se o něj stará, jak
  dobrý signál má jeho anténa a jestli dohled sítě vidí na jeho AP problém, aby
  odlišil problém u sebe od problému na AP. Údaje pocházejí z evidence členů
  hkfree.org; údaje o zařízení člena vidí jen on sám a správci.

  Scénář: Člen vidí jméno, oblast a adresu svého AP
    Pokud adresa "10.107.77.5" patří k AP "AP2" v oblasti "Kukleny" na adrese "Pospíšilova 12, Hradec Králové"
    Když člen spustí kontrolu
    Pak vidí, že se připojuje přes AP "AP2" v oblasti "Kukleny" na adrese "Pospíšilova 12, Hradec Králové"

  Scénář: Člen na požádání vidí signál své antény
    Pokud anténa člena "10.107.77.5" přijímá AP se signálem -67 dBm
    Když si člen vyžádá signál své antény
    Pak vidí signál -67 dBm

  Scénář: Člen vidí, že dohled sítě hlásí problém na jeho AP
    Pokud dohled sítě hlásí ztráty paketů na AP routeru "10.107.77.1"
    Když člen spustí kontrolu
    Pak vidí, že dohled sítě hlásí ztráty paketů na jeho AP routeru "10.107.77.1"
