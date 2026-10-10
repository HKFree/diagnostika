// Built-in troubleshooting guide (cs/en). The assistant answers from these
// articles (offline search), and a model, if used, may only rephrase them.
// Keep both languages in sync: the unit tests check that every article exists
// in both.

export const KB = {
  cs: [
    {
      id: 'restart',
      title: 'Restart routeru a antény',
      keywords: 'restart restartovat vypnout zapnout router antena napajeni zasuvka nefunguje internet spojeni',
      body: [
        'Vytáhněte ze zásuvky svůj router i PoE zdroj antény (malý adaptér, ze kterého vede kabel na střechu).',
        'Počkejte 30 sekund.',
        'Zapojte nejdřív PoE zdroj antény a počkejte asi 2 minuty, než se anténa spojí s AP.',
        'Potom zapojte router a počkejte další 1–2 minuty.',
        'Nedělejte tovární reset routeru: smazali byste nastavení a připojení by nemuselo fungovat vůbec.',
      ],
    },
    {
      id: 'poe',
      title: 'Kontrola napájení antény (PoE)',
      keywords: 'poe zdroj adapter antena strecha kabel napajeni led kontrolka svetlo konektor vlhkost blesk',
      body: [
        'Anténa na střeše je napájená po síťovém kabelu z PoE zdroje (adaptéru) v bytě.',
        'Zkontrolujte, že PoE zdroj svítí. Nesvítí-li, zkuste jinou zásuvku.',
        'Zkontrolujte, že je kabel do PoE zdroje (port POE/DATA) i z něj do routeru (LAN) pevně zacvaknutý.',
        'Po bouřce může být PoE zdroj nebo anténa poškozená bleskem: pak kontaktujte podporu.',
      ],
    },
    {
      id: 'wifi',
      title: 'Wi-Fi nebo kabel',
      keywords: 'wifi wi-fi bezdrat kabel signal dosah 5ghz 2.4ghz slaby pripojeni zarizeni telefon notebook',
      body: [
        'Pokud to jde, připojte počítač k routeru kabelem. Funguje-li po kabelu a po Wi-Fi ne, je problém ve Wi-Fi.',
        'Wi-Fi 5 GHz je rychlejší, ale má menší dosah přes zdi; 2,4 GHz dosáhne dál.',
        'Zkuste jít blíž k routeru, vypnout a zapnout Wi-Fi na zařízení, případně restartovat router.',
        'Funguje-li jiné zařízení, je problém v tom jednom zařízení.',
      ],
    },
    {
      id: 'dhcp',
      title: 'Obnovení IP adresy (DHCP)',
      keywords: 'ip adresa dhcp obnovit zapujcka ipconfig renew windows mac android iphone ios zapomenout sit',
      body: [
        'Windows: spusťte příkazový řádek a zadejte ipconfig /release a potom ipconfig /renew.',
        'macOS: Nastavení systému → Síť → vaše připojení → Podrobnosti → TCP/IP → Obnovit zápůjčku DHCP.',
        'iPhone: Nastavení → Wi-Fi → (i) u vaší sítě → Obnovit zápůjčku.',
        'Android: dlouze podržte síť → Zapomenout a připojte se znovu.',
      ],
    },
    {
      id: 'dns',
      title: 'Nefungují jména stránek (DNS)',
      keywords: 'dns jmena stranky nenacita adresa preklad 1.1.1.1 9.9.9.9 nameserver nelze najit server',
      body: [
        'Když internet funguje podle IP adresy, ale stránky se nenačítají podle jména, je problém v DNS.',
        'V routeru (nebo v nastavení zařízení) nastavte DNS servery ručně, například 1.1.1.1 a 9.9.9.9.',
        'Potom restartujte router nebo obnovte připojení zařízení.',
      ],
    },
    {
      id: 'slow',
      title: 'Pomalé nebo přerušované připojení',
      keywords: 'pomaly pomale rychlost vypadky vypadava prerusovane ztraty paketu lag ping zpozdeni seka',
      body: [
        'Nejdřív vyzkoušejte kabel místo Wi-Fi: pomalá Wi-Fi je nejčastější příčina.',
        'Zkontrolujte, jestli síť nevytěžuje jiné zařízení v domácnosti (stahování, zálohy, kamery).',
        'V záložce Cesta se podívejte, jestli některý router nemá „ztráty paketů“ nebo „pomalý“ (údaje ze Sojky).',
        'Slabý signál antény (stromy, sníh, námraza, pootočená anténa) zpomaluje připojení: viz Síla signálu antény.',
        'Trvá-li problém, kontaktujte podporu a uveďte, kdy se to děje.',
      ],
    },
    {
      id: 'signal',
      title: 'Síla signálu antény',
      keywords: 'signal sila antena natoceni smer stromy snih namraza pocasi rusani ccq wewimo',
      body: [
        'Rádiový spoj potřebuje přímou viditelnost mezi vaší anténou a AP.',
        'Zhoršit ho mohou listí, sníh nebo námraza na anténě, nová stavba nebo pootočení antény větrem.',
        'Správci sítě vidí sílu signálu vaší antény (nástroj Wewimo). Pokud máte podezření na signál, kontaktujte podporu.',
      ],
    },
    {
      id: 'newrouter',
      title: 'Vyměnil jsem router nebo zařízení',
      keywords: 'novy router vymena zmena mac adresa filtr registrace userdb nefunguje po vymene zarizeni',
      body: [
        'V síti hkfree.org je zařízení často registrované podle MAC adresy (MAC filtr v UserDB).',
        'Po výměně routeru proto připojení nemusí fungovat, dokud se nová MAC adresa nezaregistruje.',
        'Upravte ji v UserDB (userdb.hkfree.org) nebo požádejte správce svého AP.',
      ],
    },
    {
      id: 'partial',
      title: 'Nefunguje jen něco',
      keywords: 'nefunguje jen jedna stranka sluzba aplikace youtube netflix email nekde nekdy vpn',
      body: [
        'Pokud ostatní stránky fungují, je problém nejspíš na straně dané služby.',
        'Zkuste to v jiném prohlížeči nebo na jiném zařízení a vypněte případnou VPN.',
        'Pokud nefungují jen jména stránek, viz Nefungují jména stránek (DNS).',
      ],
    },
    {
      id: 'verdicts',
      title: 'Co znamenají výsledky kontroly',
      keywords: 'vysledek kontrola znamena verdikt zarizeni neni pripojeno ap router neodpovida vypadek pater dns',
      body: [
        '„Zařízení není připojeno“ nebo „Ze zařízení nic neodchází“: problém je u vás (Wi-Fi, kabel, router).',
        '„Váš AP router neodpovídá“: problém je mezi vámi a AP (váš router, anténa, rádiový spoj) nebo na AP.',
        '„Výpadek v páteřní síti hkfree.org“: problém je dál v síti, ne u vás; nic nerestartujte.',
        '„Problém s DNS“: internet funguje, jen se nepřekládají jména.',
        '„hkfree.org funguje, internet ne“: problém je na výstupu do internetu nebo dál.',
      ],
    },
    {
      id: 'pathtab',
      title: 'Jak číst záložku Cesta',
      keywords: 'cesta zalozka router hop cena ospf sojka zalozni cesta vystup internet jak cist',
      body: [
        'Cesta ukazuje routery, přes které vede vaše připojení do internetu, od vašeho AP po výstup.',
        'U každého routeru je stav ze Sojky (monitorovací server): OK, pomalý, ztráty paketů, nedostupný, bez dat.',
        '„Místa bez záložní cesty“ jsou spoje a routery, jejichž výpadek vás odpojí úplně.',
      ],
    },
    {
      id: 'reports',
      title: 'Hlášení výpadků',
      keywords: 'hlaseni vypadek nahlasit odeslat soukromi data co se odesila vypadky zalozka',
      body: [
        'Když kontrola zjistí výpadek v síti hkfree.org, aplikace si ho uloží a po obnovení spojení ho sama nahlásí.',
        'Odesílá se čas výpadku, typ výsledku, routery na vaší cestě a vaše adresa v hkfree.org.',
        'V záložce Výpadky jsou nahlášené výpadky všech členů, ale bez jejich adres.',
      ],
    },
    {
      id: 'support',
      title: 'Kdy a jak kontaktovat podporu',
      keywords: 'podpora kontakt spravce ap pomoc nahlasit telefon email kdy kontaktovat',
      body: [
        'Kontaktujte podporu, když je problém v síti (AP nebo páteř) a trvá déle než asi hodinu, nebo když nepomohl restart.',
        'Uveďte svou adresu v hkfree.org, od kdy problém trvá a výsledek kontroly (nejlépe snímek záložky Cesta).',
        'Kontakt na podporu ukazuje aplikace pod výsledkem kontroly.',
      ],
    },
    {
      id: 'othernet',
      title: 'Nejsem v síti hkfree.org',
      keywords: 'jina sit mobilni data mimo hkfree odjinud prace',
      body: [
        'Ukazatel „jiná síť“ znamená, že zařízení je připojené odjinud, třeba přes mobilní data.',
        'Routery hkfree.org jsou pak zvenku nedostupné, proto je aplikace nekontroluje.',
        'Diagnostika vaší domácí přípojky podle snímku sítě (záložka Cesta) funguje i tak.',
      ],
    },
  ],
  en: [
    {
      id: 'restart',
      title: 'Restarting the router and antenna',
      keywords: 'restart reboot power off on router antenna socket not working internet connection',
      body: [
        'Unplug your router and the antenna\'s PoE adapter (the small adapter with the cable going to the roof).',
        'Wait 30 seconds.',
        'Plug in the antenna\'s PoE adapter first and wait about 2 minutes for the antenna to connect to the AP.',
        'Then plug in the router and wait another 1–2 minutes.',
        'Do not factory-reset the router: its settings would be lost and the connection might stop working entirely.',
      ],
    },
    {
      id: 'poe',
      title: 'Checking the antenna power (PoE)',
      keywords: 'poe adapter injector antenna roof cable power led light connector moisture lightning',
      body: [
        'The rooftop antenna is powered over the network cable from a PoE adapter in your flat.',
        'Check that the PoE adapter is lit. If not, try another socket.',
        'Check that the cables into the PoE adapter (POE/DATA port) and from it to the router (LAN) are firmly clicked in.',
        'After a thunderstorm the PoE adapter or the antenna may be damaged by lightning: contact support.',
      ],
    },
    {
      id: 'wifi',
      title: 'Wi-Fi or cable',
      keywords: 'wifi wi-fi wireless cable signal range 5ghz 2.4ghz weak connection device phone laptop',
      body: [
        'If you can, connect a computer to the router with a cable. If it works by cable but not over Wi-Fi, the problem is the Wi-Fi.',
        '5 GHz Wi-Fi is faster but reaches less far through walls; 2.4 GHz reaches further.',
        'Move closer to the router, turn Wi-Fi on the device off and on, or restart the router.',
        'If another device works, the problem is in that one device.',
      ],
    },
    {
      id: 'dhcp',
      title: 'Renewing the IP address (DHCP)',
      keywords: 'ip address dhcp renew lease ipconfig windows mac android iphone ios forget network',
      body: [
        'Windows: open a command prompt and run ipconfig /release, then ipconfig /renew.',
        'macOS: System Settings → Network → your connection → Details → TCP/IP → Renew DHCP Lease.',
        'iPhone: Settings → Wi-Fi → (i) next to your network → Renew Lease.',
        'Android: long-press the network → Forget, then connect again.',
      ],
    },
    {
      id: 'dns',
      title: 'Site names do not work (DNS)',
      keywords: 'dns names sites not loading address resolve 1.1.1.1 9.9.9.9 nameserver server not found',
      body: [
        'When the internet works by IP address but sites do not load by name, the problem is DNS.',
        'Set the DNS servers manually on the router (or the device), for example 1.1.1.1 and 9.9.9.9.',
        'Then restart the router or reconnect the device.',
      ],
    },
    {
      id: 'slow',
      title: 'Slow or intermittent connection',
      keywords: 'slow speed drops dropping intermittent packet loss lag ping latency stutter',
      body: [
        'First try a cable instead of Wi-Fi: slow Wi-Fi is the most common cause.',
        'Check whether another device at home is using the connection heavily (downloads, backups, cameras).',
        'In the Path tab, check whether any router shows "packet loss" or "slow" (data from Sojka).',
        'A weak antenna signal (trees, snow, ice, a turned antenna) slows the connection: see Antenna signal strength.',
        'If it persists, contact support and say when it happens.',
      ],
    },
    {
      id: 'signal',
      title: 'Antenna signal strength',
      keywords: 'signal strength antenna alignment direction trees snow ice weather interference ccq wewimo',
      body: [
        'The radio link needs a clear line of sight between your antenna and the AP.',
        'Leaves, snow or ice on the antenna, a new building, or the antenna turned by wind can weaken it.',
        'The network admins can see your antenna\'s signal strength (the Wewimo tool). If you suspect the signal, contact support.',
      ],
    },
    {
      id: 'newrouter',
      title: 'I replaced my router or device',
      keywords: 'new router replace change mac address filter registration userdb not working after replacement device',
      body: [
        'In hkfree.org, devices are often registered by MAC address (MAC filter in UserDB).',
        'After replacing the router the connection may not work until the new MAC address is registered.',
        'Update it in UserDB (userdb.hkfree.org) or ask your AP admin.',
      ],
    },
    {
      id: 'partial',
      title: 'Only something does not work',
      keywords: 'only one site service app youtube netflix email sometimes somewhere vpn',
      body: [
        'If other sites work, the problem is most likely on that service\'s side.',
        'Try another browser or device and turn off any VPN.',
        'If only site names fail, see Site names do not work (DNS).',
      ],
    },
    {
      id: 'verdicts',
      title: 'What the check results mean',
      keywords: 'result check meaning verdict device offline ap router not answering outage backbone dns',
      body: [
        '"This device is offline" or "Nothing leaves this device": the problem is at your place (Wi-Fi, cable, router).',
        '"Your AP router does not answer": the problem is between you and the AP (your router, antenna, radio link) or at the AP.',
        '"Outage in the hkfree.org backbone": the problem is further in the network, not at your place; do not restart anything.',
        '"DNS problem": the internet works, only names do not resolve.',
        '"hkfree.org works, the internet does not": the problem is at the internet exit or beyond.',
      ],
    },
    {
      id: 'pathtab',
      title: 'How to read the Path tab',
      keywords: 'path tab router hop cost ospf sojka backup route exit internet how to read',
      body: [
        'Path shows the routers your connection uses to reach the internet, from your AP to the exit.',
        'Each router shows its status from Sojka (the monitoring server): OK, slow, packet loss, down, no data.',
        '"No backup route at" lists links and routers whose failure would disconnect you completely.',
      ],
    },
    {
      id: 'reports',
      title: 'Outage reports',
      keywords: 'report outage send privacy data what is sent outages tab',
      body: [
        'When a check finds an outage in the hkfree.org network, the app saves it and reports it itself once the connection is back.',
        'It sends the time of the outage, the result type, the routers on your path and your hkfree.org address.',
        'The Outages tab lists everyone\'s reported outages, without their addresses.',
      ],
    },
    {
      id: 'support',
      title: 'When and how to contact support',
      keywords: 'support contact admin ap help report phone email when',
      body: [
        'Contact support when the problem is in the network (AP or backbone) and lasts more than about an hour, or when a restart did not help.',
        'Give your hkfree.org address, since when it has been happening and the check result (ideally a screenshot of the Path tab).',
        'The app shows the support contact under the check result.',
      ],
    },
    {
      id: 'othernet',
      title: 'I am not on the hkfree.org network',
      keywords: 'other network mobile data outside hkfree elsewhere work',
      body: [
        'The "other network" indicator means the device is connected some other way, e.g. mobile data.',
        'hkfree.org routers are unreachable from outside, so the app does not check them.',
        'The diagnosis of your home connection from the network snapshot (Path tab) still works.',
      ],
    },
  ],
};

/** Lowercase, strip diacritics, split into words. */
export function tokens(text) {
  return String(text)
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .split(/[^a-z0-9.]+/)
    .filter((t) => t.length > 1);
}

// Words that carry no topic (they occur in every article).
const STOP = new Set((
  'a i o u v k s z na je se to ten ta to jak co mam mi me muj moje jsem jsi ale nebo kdyz pro po od do '
  + 'uz jeste jen tak take tez byt bylo by ktery ktera ktere kde kdy proc nejde nefunguje '
  + 'the a an is are was be to of in on at for with and or my me i it this that what how why when where '
  + 'do does did not no can could should would your you have has'
).split(' '));

/** Two words match if one is a prefix of the other on ≥ 4 letters (crude Czech stemming). */
function similar(a, b) {
  if (a === b) return true;
  const n = Math.min(a.length, b.length, 5);
  return n >= 4 && a.slice(0, n) === b.slice(0, n);
}

/** The best matching articles for a question, best first. */
export function search(lang, question, limit = 3) {
  const articles = KB[lang] || KB.cs;
  const q = [...new Set(tokens(question))].filter((t) => !STOP.has(t));
  if (!q.length) return [];
  const scored = articles.map((a) => {
    const title = tokens(a.title);
    const keys = tokens(a.keywords);
    const body = tokens(a.body.join(' '));
    let score = 0;
    for (const t of q) {
      if (title.some((w) => similar(w, t))) score += 3;
      if (keys.some((w) => similar(w, t))) score += 3;
      if (body.some((w) => similar(w, t))) score += 1;
    }
    return { a, score };
  });
  // At least one match in a title or the keywords: body-only hits are noise.
  return scored.filter((x) => x.score >= 3).sort((x, y) => y.score - x.score).slice(0, limit).map((x) => x.a);
}
