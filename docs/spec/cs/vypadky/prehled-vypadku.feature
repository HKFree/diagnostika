# language: cs
@id:prehled-vypadku @backlog
Požadavek: Přehled výpadků pro všechny
  Kdokoli, kdo otevře aplikaci, chce vidět, jaké výpadky členové nahlásili,
  aby poznal, jestli jeho problém nemá i někdo další. Adresy členů v přehledu
  nejsou.

  Scénář: Nahlášený výpadek bez adresy člena
    Pokud člen "10.107.10.146" nahlásil výpadek routeru "10.107.241.36", který trval 1 minutu
    Když si kdokoli zobrazí přehled výpadků
    Pak vidí výpadek routeru "10.107.241.36", který trval 1 minutu
    Ale nevidí adresu "10.107.10.146"

  Scénář: Přehled ukazuje poslední týden
    Pokud byl jeden výpadek nahlášen před 2 dny a druhý před 8 dny
    Když si kdokoli zobrazí přehled výpadků
    Pak vidí jen výpadek z doby před 2 dny

  Scénář: Nejčastěji hlášené routery
    Pokud za poslední týden nahlásili 3 členové výpadek routeru "10.107.241.36"
    Když si kdokoli zobrazí přehled výpadků
    Pak vidí router "10.107.241.36" mezi nejčastěji hlášenými se 3 hlášeními
