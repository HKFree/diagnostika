# language: cs
@id:jazyk-aplikace @backlog
Požadavek: Jazyk aplikace
  Aplikace mluví česky. Členové, kteří česky nemluví, ji dostanou v angličtině
  podle nastavení svého zařízení, a jazyk si mohou i přepnout.

  Scénář: Telefon nastavený na angličtinu
    Pokud má člen telefon nastavený na angličtinu
    Když otevře aplikaci
    Pak je aplikace v angličtině

  Scénář: Telefon nastavený na jiný jazyk
    Pokud má člen telefon nastavený na němčinu
    Když otevře aplikaci
    Pak je aplikace v češtině

  Scénář: Člen si přepne jazyk
    Pokud má člen telefon nastavený na češtinu
    A přepne si aplikaci do angličtiny
    Když aplikaci znovu otevře
    Pak je aplikace v angličtině
