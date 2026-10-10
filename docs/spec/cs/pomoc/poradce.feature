# language: cs
@id:poradce @backlog
Požadavek: Rady, jak si pomoci sám
  Člen se chce zeptat vlastními slovy, jak problém zjistit a vyřešit, a dostat
  srozumitelný postup i bez připojení. Odpověď vychází z průvodce hkfree.org
  a z jeho poslední kontroly a otázka neodchází mimo zařízení člena.

  Scénář: Jak restartovat router a anténu
    Pokud zařízení člena není připojené k internetu
    Když se člen zeptá "Jak správně restartovat router a anténu?"
    Pak dostane postup, ve kterém se nejdřív zapojí anténa a až potom router
    A vidí, ze kterého článku průvodce rada pochází

  Scénář: Rada navazuje na poslední kontrolu
    Pokud poslední kontrola člena zjistila výpadek v páteři
    Když se člen zeptá "Co mám dělat?"
    Pak vidí výsledek své poslední kontroly
    A radu nic nerestartovat

  Scénář: Průvodce odpověď nezná
    Když se člen zeptá "Kdo vyhrál včera fotbal?"
    Pak se dozví, že průvodce k tomu nic nemá
    A dostane radu zeptat se jinak nebo kontaktovat podporu

  Scénář: Odpověď vlastními slovy od jazykového modelu
    Pokud si člen do zařízení stáhl jazykový model z hkfree.org
    Když se zeptá "Proč je internet pomalý?"
    Pak dostane odpověď vlastními slovy, která vychází z průvodce
    A vidí, ze kterých článků průvodce vychází
