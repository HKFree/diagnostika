# Diagnostika hkfree.org

Aplikace, se kterou člen sítě hkfree.org zjistí, proč mu nefunguje připojení,
a co s tím. / An app with which an hkfree.org member finds out why their
connection doesn't work, and what to do about it.

## Lidé / People

**Člen** / **Member**:
Člověk připojený k síti hkfree.org přes svůj AP. / A person connected to the hkfree.org network through their AP.
_Avoid_: uživatel / user, zákazník / customer

**Správce** / **Admin**:
Člen, kterého evidence členů hkfree.org vede v roli SO (správce oblasti), ZSO (zástupce správce oblasti), TECH (technik) nebo VV (výkonný výbor). / A member whom the hkfree.org member records hold in the role SO (area admin), ZSO (deputy area admin), TECH (technician) or VV (board).
_Avoid_: operátor / operator

**Správce AP** / **AP admin**:
Správce oblasti, do které patří AP člena; první, na koho se člen obrací. / The admin of the area the member's AP belongs to; the first person a member turns to.

**Účet hkfree.org** / **hkfree.org account**:
Přihlášení člena do evidence členů hkfree.org. / A member's login to the hkfree.org member records.
_Avoid_: UserDB účet / UserDB account

**Jednotné přihlášení hkfree.org** / **hkfree.org single sign-on**:
Společné přihlášení ke službám hkfree.org účtem hkfree.org. / The shared login to hkfree.org's services with an hkfree.org account.
_Avoid_: SSO (in member-facing text)

**Podpora** / **Support**:
Ústřední kontakt hkfree.org pro členy, jejichž správce AP není známý: e-mail, telefon v pracovní dny 9–17 h a kanál #nejdeto na Slacku hkfree.org. / hkfree.org's central contact for members whose AP admin isn't known: e-mail, phone on working days 9–17, and the #nejdeto channel on the hkfree.org Slack.

## Síť / Network

**hkfree.org**:
Komunitní síť; vždy psáno malými písmeny s „.org“. / The community network; always written lowercase with ".org".
_Avoid_: HKFree, HKfree.org

**AP**:
Místo, ke kterému se připojují antény členů. / The point members' antennas connect to.
_Avoid_: vysílač / transmitter

**Anténa člena** / **Member's antenna**:
Rádiové zařízení u člena, které ho spojuje s jeho AP. / The radio device at the member's place that links them to their AP.
_Avoid_: klientská jednotka / CPE

**Signál antény** / **Antenna's signal**:
Jak silně anténa člena přijímá svůj AP, v dBm. / How strongly the member's antenna receives its AP, in dBm.

**Měření rychlosti** / **Speed test**:
Změření, jak rychle přenáší data připojení člena k serveru pro měření rychlosti v síti hkfree.org: stahování, odesílání a odezva. / Measuring how fast a member's connection carries data to the speed test server in the hkfree.org network: download, upload and response time.

**Páteř** / **Backbone**:
Spoje a routery mezi AP a výstupem do internetu. / The links and routers between the APs and the internet exit.

**Mapa sítě** / **Network map**:
Popis celé sítě hkfree.org: routery, spoje mezi nimi, které adresy kam patří a výstupy do internetu. Neobsahuje nic o zařízeních členů. / A description of the whole hkfree.org network: routers, the links between them, which addresses belong where, and the internet exits. Contains nothing about members' devices.
_Avoid_: topologie / topology, snímek / snapshot

**Skrytá část sítě** / **Hidden part of the network**:
Část sítě hkfree.org, jejíž routery mapa sítě neukazuje; ví se jen, které adresy do ní patří a přes který hraniční router se do ní vstupuje. / A part of the hkfree.org network whose routers the network map doesn't show; only which addresses belong to it and through which border router it is entered are known.
_Avoid_: oblast / area (that is the member records' area of APs)

**Hraniční router** / **Border router**:
Router v mapě sítě, za kterým leží skrytá část sítě. / A router in the network map behind which a hidden part of the network lies.

**Cesta** / **Path**:
Routery, přes které vede spojení adresy do internetu, od jejího AP po výstup. / The routers an address's connection to the internet goes through, from its AP to the exit.

**Dohled sítě** / **Network monitoring**:
Služba hkfree.org, která průběžně ověřuje, že routery odpovídají. / The hkfree.org service that continuously checks that routers answer.
_Avoid_: Sojka (the current tool's name)

## Diagnostika / Diagnosis

**Kontrola** / **Check**:
Jedno zjištění stavu připojení člena a jeho výsledek. / One assessment of a member's connection, and its result.
_Avoid_: test, diagnóza / diagnosis

**Výsledek kontroly** / **Check result**:
To, co člen vidí jako první: jestli připojení funguje, co je rozbité a co má udělat. / What the member sees first: whether the connection works, what is broken and what to do.

**Pokročilá diagnostika** / **Advanced diagnostics**:
Podrobnosti kontroly (jednotlivé testy, cesta), skryté, dokud si je člen neotevře. / The check's details (individual tests, the path), hidden until the member opens them.

**VPN hkfree.org** / **hkfree.org VPN**:
Připojení do sítě hkfree.org odjinud, přes server VPN hkfree.org. Člen je pak v síti hkfree.org, ale ne přes svou domácí přípojku. / A connection into the hkfree.org network from elsewhere, through the hkfree.org VPN server. The member is then on hkfree.org, but not through their home connection.

**Jiný poskytovatel** / **Other provider**:
Jakékoli připojení zařízení, které nevede přes hkfree.org, například mobilní data. / Any connection of the device that doesn't go through hkfree.org, e.g. mobile data.

**Výpadek** / **Outage**:
Doba, kdy člen kvůli problému v síti hkfree.org neměl spojení. / A period when a member had no connection because of a problem in the hkfree.org network.
_Avoid_: porucha / fault, chyba / error

**Hlášení výpadku** / **Outage report**:
Záznam o výpadku, který aplikace předá správcům po obnovení spojení. / The record of an outage the app hands to the admins once the connection is back.

**Průvodce** / **Guide**:
Články hkfree.org o tom, jak si s připojením pomoci sám. / hkfree.org's articles on how to help oneself with the connection.
