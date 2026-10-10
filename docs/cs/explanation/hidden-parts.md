# Skryté části sítě

hkdiag čte databázi OSPF páteře (oblast 0). Některé části hkfree.org běží jako
samostatná oblast OSPF. Routery a spoje uvnitř takové oblasti se do páteře
neohlašují. Její **hraniční router** (ABR) ohlašuje jen *summary-LSA*: jaké
rozsahy adres za ním leží. Tyto rozsahy můžou být i agregované do jednoho
většího prefixu. Například 10.107.10.97 ohlašuje 10.107.10.56/30,
10.107.10.64/28 a 10.107.10.128/28.

Aplikace tedy ví, *že* je adresa za hraničním routerem, ale ne, který router
uvnitř oblasti ji obsluhuje.

- **IPv4:** když je nejpřesnějším prefixem adresy člena souhrn, cesta začíná
  u hraničního routeru. Pokročilá diagnostika ho označí jako *hraniční router
  skryté části sítě* a řekne, že routery za ním v mapě nejsou.
- **IPv6:** OSPFv3 může tytéž routery vést v oblasti 0, takže jejich prefixy
  IPv6 jsou vidět, i když routery v mapě IPv4 nejsou. ID routerů v OSPFv3 jsou
  jejich IPv4 ID, takže takový router (např. 10.107.10.65) se umístí podle
  souhrnu, který jeho ID pokrývá, tedy za týž hraniční router.
- Když je problém uvnitř skryté části, aplikace ukáže jen to, že hraniční
  router odpovídá a provoz člena přesto neprojde. Kde přesně se zastaví,
  říct nedokáže.

V mapě sítě (`topology.json`) jsou souhrny `prefixes` druhu `"a"`.
