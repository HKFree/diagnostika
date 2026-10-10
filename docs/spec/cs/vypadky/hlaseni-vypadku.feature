# language: cs
@id:hlaseni-vypadku
Požadavek: Hlášení výpadků
  Když má člen výpadek v síti hkfree.org, správci se o něm často nedozví.
  Aplikace si výpadek zapamatuje a nahlásí ho sama, jakmile se spojení
  obnoví, aby správci věděli, kde a kdy síť selhává. Hlášení odchází bez
  ptaní; adresu člena v něm vidí jen správci.

  Pravidlo: Hlásí se jen výpadky v síti hkfree.org

    Scénář: Výpadek páteře se nahlásí po obnovení spojení
      Pokud člen "10.107.10.146" zjistil, že router "10.107.241.36" na jeho cestě neodpovídá
      Když se jeho připojení obnoví
      Pak člen vidí, že výpadek byl nahlášen
      A správci vidí výpadek routeru "10.107.241.36" od člena "10.107.10.146"

    Scénář: Problém v zařízení člena se nehlásí
      Pokud kontrola zjistila, že zařízení člena "10.107.77.5" není připojené k žádné síti
      Když se jeho připojení obnoví
      Pak správci žádný výpadek od člena "10.107.77.5" nevidí

  Pravidlo: Výpadek se nahlásí, až je člen zpět v síti hkfree.org

    Scénář: Člen se mezitím připojil přes mobilní data
      Pokud člen "10.107.10.146" zjistil výpadek routeru "10.107.241.36"
      A pak se připojil přes mobilní data
      Když spustí kontrolu
      Pak výpadek zatím nahlášený není

    Scénář: Kontrola se během výpadku opakuje sama
      Pokud člen zjistil výpadek routeru "10.107.241.36"
      A aplikace zůstala otevřená
      Když se za 2 minuty spojení obnoví
      Pak člen do minuty vidí, že výpadek byl nahlášen

    Scénář: Výpadek starší než měsíc se nehlásí
      Pokud člen zjistil výpadek před 31 dny a od té doby aplikaci neotevřel
      Když se jeho připojení obnoví
      Pak správci tento výpadek nevidí
