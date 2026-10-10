// Localisation: Czech (default) and English, chosen from the browser's
// preferred languages; a manual choice is stored per device.

export const LANGS = ['cs', 'en'];

/** First supported language in the browser's preference list; Czech otherwise. */
export function detectLang(prefs) {
  for (const l of prefs || []) {
    const p = String(l).toLowerCase().slice(0, 2);
    if (p === 'cs' || p === 'sk') return 'cs'; // Slovak readers are better served by Czech
    if (p === 'en') return 'en';
  }
  return 'cs';
}

/** Czech plural form: 1 / 2–4 / 5+ (and 0). */
function csPlural(n, one, few, many) {
  if (n === 1) return one;
  if (n >= 2 && n <= 4) return few;
  return many;
}

const label = (r, n) => (n ? `${r} (${n})` : r);

const M = {
  cs: {
    appName: 'Diagnostika hkfree.org',
    tabCheck: 'Kontrola',
    tabPath: 'Cesta',
    tabHistory: 'Historie',
    online: 'online',
    offline: 'offline',
    ready: 'Připraveno',
    readyText: 'Spusťte kontrolu a zjistěte, kde se vaše připojení přerušuje.',
    run: 'Spustit kontrolu',
    runAgain: 'Zkontrolovat znovu',
    checking: 'Kontroluji…',
    checkingText: 'Testuji připojení krok za krokem.',
    chkDevice: 'Zařízení je připojeno k síti',
    chkServer: 'Server hkfree.org',
    chkInetIp: 'Internet (podle IP adresy)',
    chkInetDns: 'Internet (podle jména, DNS)',
    ms: (p) => `${p.ms} ms`,
    pillOk: 'OK',
    pillFailed: 'selhalo',
    pillChecking: 'kontroluji…',
    pillReachable: 'dostupný',
    pillLost: 'neodpovídá',
    pillUntestable: 'nelze otestovat',
    pathProbes: 'Vaše cesta (podle odezvy)',
    mapDevice: 'Vaše zařízení',
    roleBorder: 'hraniční router skryté části sítě',
    hiddenPart: 'Routery za hraničním routerem v mapě sítě nejsou; váš AP je někde za ním.',
    apInfo: (p) => `Připojujete se přes AP ${p.name} v oblasti ${p.area}${p.address ? `, ${p.address}` : ''}.`,
    apUnknown: (p) => `Váš AP router ${p.ip} evidence členů nepojmenuje.`,
    mapInternet: 'Internet',
    pathUnknown: 'Vaši cestu pro tuto adresu nelze určit: její router v mapě sítě není.',
    roleAp: 'váš AP router',
    roleHop: 'páteř',
    roleExit: 'výstup do internetu',
    myAddress: (p) => `Vaše adresa v hkfree.org: ${p.ip}`,
    myAddressManual: (p) => `Vaše adresa v hkfree.org: ${p.ip} (zadaná ručně)`,
    myAddressUnknown: 'Vaši adresu v hkfree.org se nepodařilo zjistit. Zadejte ji:',
    addressPlaceholder: 'např. 10.107.77.5',
    use: 'Použít',
    automatic: 'Zjistit automaticky',
    install: 'Nainstalovat aplikaci',
    language: 'Jazyk',
    footer: 'Všechny kontroly běží na tomto zařízení. Kontroly routerů vycházejí z odezvy a porovnávají se s poslední kontrolou, kdy vše fungovalo.',
    // Path tab
    pathFor: (p) => `Cesta adresy ${p.ip} do internetu`,
    diagnoseOther: 'Diagnostikovat adresu',
    show: 'Zobrazit',
    snapshotFrom: (p) => `Snímek sítě ${p.ago}`,
    snapshotSaved: ' · uložená kopie (nepodařilo se obnovit)',
    noSnapshot: 'Zatím nemám snímek sítě. Otevřete aplikaci jednou, když připojení funguje.',
    badAddress: 'Neplatná IPv4 adresa.',
    verdictTitle: 'Výsledek podle snímku sítě',
    noProblem: 'Ve snímku sítě není u této adresy žádný problém.',
    findings: 'Zjištění',
    noFindings: 'Žádná zjištění.',
    noPathFound: 'Do internetu nevede žádná cesta.',
    hopSub: (p) => `${p.ip} · cena OSPF ${p.cost}${p.sojka ? ` · Sojka: ${p.sojka}` : ''}`,
    exitInfo: (p) => `Výstup: ${p.router} (${p.e1 ? 'E1' : 'E2'}, metrika ${p.metric}) · celková cena ${p.cost}`,
    sevCritical: 'kritické',
    sevWarning: 'varování',
    sevInfo: 'info',
    sojka: { Ok: 'OK', Slow: 'pomalý', Lossy: 'ztráty paketů', Dead: 'nedostupný', Stale: 'bez dat' },
    // History
    noHistory: 'Zatím žádné kontroly.',
    clearHistory: 'Smazat historii',
    // time
    ago: (s) => {
      if (s < 90) return `před ${s} s`;
      if (s < 5400) return `před ${Math.round(s / 60)} min`;
      if (s < 172800) return `před ${Math.round(s / 3600)} h`;
      const d = Math.round(s / 86400);
      return `před ${d} ${csPlural(d, 'dnem', 'dny', 'dny')}`;
    },
    // Device verdicts
    v: {
      offline: ['Zařízení není připojeno', 'Není připojeno k žádné síti. Zkontrolujte Wi-Fi nebo kabel a restartujte svůj router.'],
      localFailure: ['Ze zařízení nic neodchází', 'Zařízení hlásí připojení, ale každé spojení okamžitě selže. Zkontrolujte Wi-Fi nebo kabel, obnovte připojení (DHCP) a restartujte svůj router.'],
      ok: ['Připojení funguje', 'hkfree.org i internet jsou z tohoto zařízení dostupné.'],
      dns: ['Problém s DNS', 'Internet funguje podle IP adresy, ale jména se nepřekládají. Zkontrolujte DNS servery v routeru nebo zařízení.'],
      inetDown: ['hkfree.org funguje, internet ne', 'Vaše cesta v hkfree.org je v pořádku, problém je na výstupu do internetu nebo dál. Podrobnosti najdete v záložce Cesta.'],
      serverOnly: ['Nedostupný je jen server hkfree.org', 'Internet funguje, jen server s diagnostikou neodpovídá. Vaše připojení je v pořádku.'],
      noPath: ['Žádné spojení s hkfree.org', 'Zařízení se nedostane do hkfree.org a zatím nemám uloženou vaši cestu. Otevřete aplikaci jednou, když připojení funguje.'],
      apDown: (p) => ['Váš AP router neodpovídá', `${label(p.ip, p.name)} normálně odpovídá, ale teď ne. Problém je mezi vámi a AP: Wi-Fi nebo kabel, váš router či anténa, rádiový spoj, nebo samotný AP.`],
      backbone: (p) => ['Výpadek v páteřní síti hkfree.org', p.lastOk
        ? `Dostupné až po ${label(p.lastOk.ip, p.lastOk.name)}; ${label(p.bad.ip, p.bad.name)} neodpovídá. Jde o výpadek sítě, správci o něm obvykle už vědí.`
        : `${label(p.bad.ip, p.bad.name)} normálně odpovídá, ale teď ne; routery před ním nelze z prohlížeče otestovat. Jde o výpadek sítě.`],
      known: (p) => ['Známý problém v síti', `Podle snímku sítě ${p.ago}: ${p.title}`],
      generic: ['Žádné spojení s hkfree.org', 'hkfree.org není dostupné, ale routery na vaší cestě odsud vypadají normálně. Restartujte svůj router; pokud problém trvá, kontaktujte správce svého AP.'],
      failed: (p) => ['Kontrolu se nepodařilo dokončit', p.msg],
    },
    // What to do, per verdict
    stepsTitle: 'Co dělat',
    advanced: 'Pokročilá diagnostika',
    steps: {
      offline: [
        'Zapněte Wi-Fi nebo zkontrolujte, že je zapojený síťový kabel.',
        'Ověřte, že jste připojeni ke své domácí síti, ne k jiné Wi-Fi.',
        'Restartujte svůj router: vytáhněte ho na 30 sekund ze zásuvky.',
        'Zkuste jiné zařízení. Pokud na něm síť funguje, je problém v tomto zařízení.',
      ],
      localFailure: [
        'Vypněte a znovu zapněte Wi-Fi na tomto zařízení.',
        'Odpojte se od sítě a připojte se znovu (případně ji „zapomeňte“ a připojte znovu).',
        'Restartujte svůj router (30 sekund bez napájení).',
        'Zkontrolujte, že router svítí a že má napájení i anténa na střeše (PoE zdroj).',
      ],
      apDown: [
        'Restartujte svůj router (30 sekund bez napájení).',
        'Zkontrolujte napájení antény na střeše: PoE zdroj musí svítit a kabel k anténě musí být v pořádku.',
        'Zeptejte se sousedů připojených ke stejnému AP, jestli jim síť funguje.',
        'Pokud problém trvá déle než 15 minut, kontaktujte podporu.',
      ],
      backbone: [
        'Problém je v páteřní síti hkfree.org, ne u vás. Nic nerestartujte.',
        'Výpadek se uloží a po obnovení spojení se automaticky nahlásí.',
        'Pokud trvá déle než hodinu, kontaktujte podporu.',
      ],
      known: [
        'Problém je v síti hkfree.org, ne u vás. Nic nerestartujte.',
        'Výpadek se uloží a po obnovení spojení se automaticky nahlásí.',
        'Pokud trvá déle než hodinu, kontaktujte podporu.',
      ],
      generic: [
        'Restartujte svůj router (30 sekund bez napájení).',
        'Zkontrolujte napájení antény na střeše (PoE zdroj).',
        'Pokud problém trvá, kontaktujte podporu.',
      ],
      noPath: [
        'Vypněte a zapněte Wi-Fi a restartujte svůj router.',
        'Až připojení poběží, otevřete aplikaci znovu. Uloží si vaši cestu pro příští výpadek.',
      ],
      dns: [
        'V routeru nebo v zařízení nastavte jiné DNS servery (například 1.1.1.1 a 9.9.9.9).',
        'Restartujte svůj router.',
      ],
      inetDown: [
        'Vaše připojení k hkfree.org funguje; problém je na výstupu do internetu nebo dál.',
        'Počkejte několik minut. Pokud problém trvá, kontaktujte podporu.',
      ],
      serverOnly: ['Vaše připojení funguje, není potřeba nic dělat.'],
      ok: ['Pokud nefunguje jen určitá stránka nebo služba, je problém nejspíš na její straně.'],
    },
    support: 'Podpora',
    supportFallback: 'Kontaktujte správce svého AP nebo podporu hkfree.org.',
    adminSo: 'Správce vašeho AP',
    monitorOk: (p) => `Dohled sítě na vašem AP žádný problém nevidí (zařízení: ${p.total}).`,
    monitorProblems: (p) => `Dohled sítě hlásí problém na ${p.n} z ${p.total} zařízení vašeho AP:`,
    monitorHealth: { Slow: 'odpovídá pomalu', Lossy: 'ztráty paketů', Dead: 'neodpovídá' },
    apDevice: 'zařízení',
    adminZso: 'Zástupce správce',
    phoneHours: (p) => `(pracovní dny ${p.hours} h)`,
    slack: (p) => `Slack hkfree.org ${p.channel}`,
    // Network indicator
    netHkfree: 'hkfree.org',
    netOther: 'jiná síť',
    netUnknown: 'síť ?',
    netHkfreeTitle: 'Toto zařízení je připojeno přes síť hkfree.org.',
    netOtherTitle: 'Toto zařízení není v síti hkfree.org (např. mobilní data nebo jiný poskytovatel).',
    netUnknownTitle: 'Nepodařilo se zjistit, přes jakou síť je zařízení připojeno.',
    otherNetworkNote: 'Nejste v síti hkfree.org, proto se routery na cestě nekontrolují.',
    pillOutside: 'mimo hkfree.org',
    myAddressLast: (p) => `Vaše adresa v hkfree.org: ${p.ip} (naposledy zjištěná)`,
    // Outage reports
    outageSaved: 'Výpadek je uložený a po obnovení spojení se nahlásí. Kontroluji znovu každou minutu.',
    outagesSent: (p) => `${csPlural(p.n, 'Nahlášen', 'Nahlášeny', 'Nahlášeno')} ${p.n} ${csPlural(p.n, 'výpadek', 'výpadky', 'výpadků')}. Děkujeme.`,
    outagesPending: (p) => `Čeká na odeslání: ${p.n}`,
    // Outages tab
    tabOutages: 'Výpadky',
    outagesTitle: 'Nahlášené výpadky za posledních 7 dní',
    outagesNone: 'Za posledních 7 dní nikdo nenahlásil výpadek.',
    outagesDisabled: 'Tento server hlášení výpadků neukládá.',
    outagesUnavailable: 'Seznam výpadků teď nelze načíst.',
    outagesCached: (p) => `Uložená kopie ${p.ago}`,
    outagesTop: 'Nejčastěji hlášené routery',
    outageKind: { apDown: 'Nedostupný AP router', backbone: 'Výpadek páteřní sítě', known: 'Známý problém v síti', generic: 'Přerušené spojení s hkfree.org' },
    outageSub: (p) => [p.ap && `AP ${p.ap}`, p.bad && `neodpovídal ${p.bad}`, `trvání ${p.dur}`].filter(Boolean).join(' · '),
    duration: (s) => (s < 3600 ? `${Math.max(1, Math.round(s / 60))} min` : `${Math.floor(s / 3600)} h ${Math.round((s % 3600) / 60)} min`),
    reportsCount: (p) => `${p.n}×`,
    // Assistant (Help tab)
    tabHelp: 'Pomoc',
    helpIntro: 'Zeptejte se, jak problém zjistit a vyřešit. Odpovědi vycházejí z průvodce v aplikaci a z vaší poslední kontroly a fungují i offline.',
    askPlaceholder: 'Napište otázku…',
    ask: 'Zeptat se',
    engineGuide: 'Odpovídá: průvodce v aplikaci (funguje offline).',
    engineBuiltin: 'Odpovídá: vestavěný model prohlížeče (běží v zařízení).',
    engineWebllm: (p) => `Odpovídá: model ${p.name} (běží v zařízení, funguje offline).`,
    useBuiltin: 'Použít vestavěný model prohlížeče',
    downloadModel: (p) => `Stáhnout ${p.label} (${p.mb} MB)`,
    modelSmall: 'menší model, slabší čeština',
    modelLarge: 'větší model, lepší odpovědi',
    modelReady: 'staženo',
    useModel: (p) => `Použít ${p.name}`,
    useGuide: 'Odpovídat jen z průvodce',
    removeModel: 'Odstranit stažený model',
    loadingModel: (p) => `Připravuji model… ${p.pct} %`,
    modelFailed: (p) => `Model se nepodařilo spustit (${p.msg}). Odpovídá průvodce.`,
    noWebgpu: 'Toto zařízení neumí spustit jazykový model (chybí WebGPU). Odpovídá průvodce v aplikaci.',
    modelNote: 'Model se stáhne jednou a zůstane v zařízení. Stahujte raději přes Wi-Fi.',
    aiDisclaimer: 'Model vychází z průvodce a z poslední kontroly, ale může se mýlit. Při pochybnostech kontaktujte podporu.',
    sourcesLabel: 'Zdroje',
    fromGuide: 'Z průvodce:',
    related: 'Související:',
    lastCheck: (p) => `Vaše poslední kontrola: ${p.title}`,
    noMatch: 'V průvodci jsem k tomu nic nenašel. Zkuste otázku položit jinak, nebo kontaktujte podporu.',
    you: 'Vy',
    assistantName: 'Pomocník',
    chips: [
      'Nejde mi internet. Co mám dělat?',
      'Jak správně restartovat router a anténu?',
      'Proč je internet pomalý?',
      'Vyměnil jsem router a nejde to',
      'Co znamená výsledek kontroly?',
    ],
    // Engine findings: [title, detail lines]
    f: {
      notRouted: (p) => [`Adresa ${p.ip} není v síti hkfree.org směrována`, [
        'Žádný router v OSPF neinzeruje podsíť s touto adresou.',
        ...p.previous.map((x) => `${x.prefix} dříve inzeroval router ${label(x.router, x.name)} (naposledy ${x.agoText}); router ${x.present ? 'v OSPF stále je, jen ji už neinzeruje' : 'z OSPF zmizel (výpadek?)'}.`),
      ]],
      noLsa: (p) => [`Router ${label(p.router, p.name)} chybí v OSPF`, ['Jeho podsíť ještě uvádí soused, ale samotný router v síti není: je mimo provoz.']],
      cutOff: (p) => [`AP router ${label(p.router, p.name)} je odpojený od sítě`, [
        `Stále inzeruje ${p.advertised} ${csPlural(p.advertised, 'spoj', 'spoje', 'spojů')}, ale žádný z nich soused nepotvrzuje.`,
        `Jeho záznam je ${p.age} s starý: router nebo všechna jeho spojení jsou mimo provoz.`,
      ]],
      noExit: (p) => [`Z ${label(p.router, p.name)} nevede cesta do internetu`, [`Část sítě s tímto routerem (${p.count} ${csPlural(p.count, 'router', 'routery', 'routerů')}) nemá žádný výstup do internetu.`]],
      hopDead: (p) => [`Router ${label(p.router, p.name)} na cestě neodpovídá na ping`, ['Sojka ho nedávno přestala vidět, ale jeho spojení v OSPF jsou aktivní: provoz přes něj nejspíš prochází. Pokud potíže trvají, je to první podezřelý.']],
      hopDegraded: (p) => [`Router ${label(p.router, p.name)} na cestě: ${p.healthText}`, []],
      spof: (p) => ['Místa bez záložní cesty', [
        ...p.links.map((l) => `spoj ${label(l.a, l.an)} → ${label(l.b, l.bn)}`),
        ...p.nodes.map((n) => `router ${label(n.router, n.name)}`),
      ]],
      singleExit: (p) => ['Dostupný je jen jeden výstup do internetu', [`${p.reachable} z ${p.total} výstupů je dostupný.`]],
    },
  },
  en: {
    appName: 'hkfree.org Diagnostics',
    tabCheck: 'Check',
    tabPath: 'Path',
    tabHistory: 'History',
    online: 'online',
    offline: 'offline',
    ready: 'Ready',
    readyText: 'Run a check to find out where your connection breaks.',
    run: 'Run check',
    runAgain: 'Run check again',
    checking: 'Checking…',
    checkingText: 'Testing your connection step by step.',
    chkDevice: 'This device is online',
    chkServer: 'hkfree.org server',
    chkInetIp: 'Internet (by IP address)',
    chkInetDns: 'Internet (by name, DNS)',
    ms: (p) => `${p.ms} ms`,
    pillOk: 'OK',
    pillFailed: 'failed',
    pillChecking: 'checking…',
    pillReachable: 'reachable',
    pillLost: 'no answer',
    pillUntestable: 'not testable',
    pathProbes: 'Your path (timing-based)',
    mapDevice: 'Your device',
    roleBorder: 'border router of a hidden part of the network',
    hiddenPart: 'The routers behind the border router aren\'t in the network map; your AP is somewhere behind it.',
    apInfo: (p) => `You connect through the AP ${p.name} in the area ${p.area}${p.address ? `, ${p.address}` : ''}.`,
    apUnknown: (p) => `Your AP router ${p.ip} isn't named in the member records.`,
    mapInternet: 'Internet',
    pathUnknown: 'Your path can\'t be determined for this address: its router isn\'t in the network map.',
    roleAp: 'your AP router',
    roleHop: 'backbone',
    roleExit: 'internet exit',
    myAddress: (p) => `Your address in hkfree.org: ${p.ip}`,
    myAddressManual: (p) => `Your address in hkfree.org: ${p.ip} (entered manually)`,
    myAddressUnknown: 'Could not determine your hkfree.org address. Enter it:',
    addressPlaceholder: 'e.g. 10.107.77.5',
    use: 'Use',
    automatic: 'Detect automatically',
    install: 'Install app',
    language: 'Language',
    footer: 'All checks run on this device. Router checks are timing-based and compared with your last healthy check.',
    pathFor: (p) => `Path of ${p.ip} to the internet`,
    diagnoseOther: 'Diagnose address',
    show: 'Show',
    snapshotFrom: (p) => `Network snapshot ${p.ago}`,
    snapshotSaved: ' · saved copy (could not refresh)',
    noSnapshot: 'No network snapshot yet. Open the app once while your connection works.',
    badAddress: 'Not a valid IPv4 address.',
    verdictTitle: 'Result from the network snapshot',
    noProblem: 'The network snapshot shows no problem for this address.',
    findings: 'Findings',
    noFindings: 'No findings.',
    noPathFound: 'There is no path to the internet.',
    hopSub: (p) => `${p.ip} · OSPF cost ${p.cost}${p.sojka ? ` · Sojka: ${p.sojka}` : ''}`,
    exitInfo: (p) => `Exit: ${p.router} (${p.e1 ? 'E1' : 'E2'}, metric ${p.metric}) · total cost ${p.cost}`,
    sevCritical: 'critical',
    sevWarning: 'warning',
    sevInfo: 'info',
    sojka: { Ok: 'OK', Slow: 'slow', Lossy: 'packet loss', Dead: 'down', Stale: 'no data' },
    noHistory: 'No checks yet.',
    clearHistory: 'Clear history',
    ago: (s) => {
      if (s < 90) return `${s} s ago`;
      if (s < 5400) return `${Math.round(s / 60)} min ago`;
      if (s < 172800) return `${Math.round(s / 3600)} h ago`;
      const d = Math.round(s / 86400);
      return `${d} ${d === 1 ? 'day' : 'days'} ago`;
    },
    v: {
      offline: ['This device is offline', 'It is not connected to any network. Check Wi-Fi or the cable, then restart your router.'],
      localFailure: ['Nothing leaves this device', 'Your device says it is connected, but every connection fails immediately. Check Wi-Fi or the cable, renew the connection (DHCP), then restart your router.'],
      ok: ['Your connection works', 'hkfree.org and the internet are both reachable from this device.'],
      dns: ['DNS problem', 'The internet works by IP address but names do not resolve. Check the DNS servers set on your router or device.'],
      inetDown: ['hkfree.org works, the internet does not', 'Your path inside hkfree.org is fine; the problem is at the internet exit or beyond. See the Path tab.'],
      serverOnly: ['Only the hkfree.org server is down', 'The internet works; only the diagnostics server does not answer. Your connection is fine.'],
      noPath: ['No connection to hkfree.org', 'This device cannot reach hkfree.org and has no saved path yet. Open the app once while your connection works.'],
      apDown: (p) => ['Your AP router does not answer', `${label(p.ip, p.name)} normally answers but does not now. The problem is between you and the AP: Wi-Fi or cable, your router or antenna, the radio link, or the AP itself.`],
      backbone: (p) => ['Outage in the hkfree.org backbone', p.lastOk
        ? `Reachable up to ${label(p.lastOk.ip, p.lastOk.name)}; ${label(p.bad.ip, p.bad.name)} does not answer. This is a network outage — the admins usually know already.`
        : `${label(p.bad.ip, p.bad.name)} normally answers but does not now; the routers before it cannot be tested from a browser. This is a network outage.`],
      known: (p) => ['Known network problem', `From the network snapshot ${p.ago}: ${p.title}`],
      generic: ['No connection to hkfree.org', 'hkfree.org cannot be reached, but the routers on your path still look normal from here. Restart your router; if it persists, contact your AP admin.'],
      failed: (p) => ['Check could not finish', p.msg],
    },
    stepsTitle: 'What to do',
    advanced: 'Advanced diagnostics',
    steps: {
      offline: [
        'Turn Wi-Fi on or check that the network cable is plugged in.',
        'Make sure you are connected to your home network, not another Wi-Fi.',
        'Restart your router: unplug it for 30 seconds.',
        'Try another device. If it works there, the problem is in this device.',
      ],
      localFailure: [
        'Turn Wi-Fi on this device off and on again.',
        'Disconnect from the network and reconnect (or "forget" it and join again).',
        'Restart your router (30 seconds without power).',
        'Check that the router lights are on and that the rooftop antenna has power (PoE adapter).',
      ],
      apDown: [
        'Restart your router (30 seconds without power).',
        'Check the rooftop antenna power: the PoE adapter must be lit and the antenna cable intact.',
        'Ask neighbours on the same AP whether their connection works.',
        'If it lasts more than 15 minutes, contact support.',
      ],
      backbone: [
        'The problem is in the hkfree.org backbone, not at your place. Do not restart anything.',
        'The outage is saved and reported automatically once the connection is back.',
        'If it lasts more than an hour, contact support.',
      ],
      known: [
        'The problem is in the hkfree.org network, not at your place. Do not restart anything.',
        'The outage is saved and reported automatically once the connection is back.',
        'If it lasts more than an hour, contact support.',
      ],
      generic: [
        'Restart your router (30 seconds without power).',
        'Check the rooftop antenna power (PoE adapter).',
        'If it persists, contact support.',
      ],
      noPath: [
        'Turn Wi-Fi off and on and restart your router.',
        'Once the connection works, open this app again. It saves your path for the next outage.',
      ],
      dns: [
        'Set different DNS servers on your router or device (for example 1.1.1.1 and 9.9.9.9).',
        'Restart your router.',
      ],
      inetDown: [
        'Your connection to hkfree.org works; the problem is at the internet exit or beyond.',
        'Wait a few minutes. If it persists, contact support.',
      ],
      serverOnly: ['Your connection works; nothing to do.'],
      ok: ['If only a particular site or service fails, the problem is most likely on its side.'],
    },
    support: 'Support',
    supportFallback: 'Contact your AP admin or hkfree.org support.',
    adminSo: 'Your AP admin',
    monitorOk: (p) => `Network monitoring sees no problem at your AP (${p.total} devices).`,
    monitorProblems: (p) => `Network monitoring reports a problem at ${p.n} of ${p.total} devices of your AP:`,
    monitorHealth: { Slow: 'slow responses', Lossy: 'packet loss', Dead: 'doesn\'t answer' },
    apDevice: 'device',
    adminZso: 'Deputy admin',
    phoneHours: (p) => `(working days ${p.hours})`,
    slack: (p) => `hkfree.org Slack ${p.channel}`,
    netHkfree: 'hkfree.org',
    netOther: 'other network',
    netUnknown: 'network ?',
    netHkfreeTitle: 'This device is connected through the hkfree.org network.',
    netOtherTitle: 'This device is not on the hkfree.org network (e.g. mobile data or another provider).',
    netUnknownTitle: 'Could not determine which network this device uses.',
    otherNetworkNote: 'You are not on the hkfree.org network, so the routers on your path are not checked.',
    pillOutside: 'outside hkfree.org',
    myAddressLast: (p) => `Your address in hkfree.org: ${p.ip} (last known)`,
    outageSaved: 'The outage is saved and will be reported once the connection is back. Checking again every minute.',
    outagesSent: (p) => `Reported ${p.n} outage${p.n === 1 ? '' : 's'}. Thank you.`,
    outagesPending: (p) => `Waiting to be sent: ${p.n}`,
    tabOutages: 'Outages',
    outagesTitle: 'Outages reported in the last 7 days',
    outagesNone: 'Nobody reported an outage in the last 7 days.',
    outagesDisabled: 'This server does not store outage reports.',
    outagesUnavailable: 'The outage list cannot be loaded right now.',
    outagesCached: (p) => `Saved copy ${p.ago}`,
    outagesTop: 'Most often reported routers',
    outageKind: { apDown: 'AP router unreachable', backbone: 'Backbone outage', known: 'Known network problem', generic: 'Connection to hkfree.org lost' },
    outageSub: (p) => [p.ap && `AP ${p.ap}`, p.bad && `${p.bad} did not answer`, `lasted ${p.dur}`].filter(Boolean).join(' · '),
    duration: (s) => (s < 3600 ? `${Math.max(1, Math.round(s / 60))} min` : `${Math.floor(s / 3600)} h ${Math.round((s % 3600) / 60)} min`),
    reportsCount: (p) => `${p.n}×`,
    tabHelp: 'Help',
    helpIntro: 'Ask how to find and fix the problem. Answers come from the app\'s guide and your last check, and work offline.',
    askPlaceholder: 'Type a question…',
    ask: 'Ask',
    engineGuide: 'Answering: the app\'s guide (works offline).',
    engineBuiltin: 'Answering: the browser\'s built-in model (runs on this device).',
    engineWebllm: (p) => `Answering: ${p.name} model (runs on this device, works offline).`,
    useBuiltin: 'Use the browser\'s built-in model',
    downloadModel: (p) => `Download the ${p.label} (${p.mb} MB)`,
    modelSmall: 'smaller model, weaker Czech',
    modelLarge: 'larger model, better answers',
    modelReady: 'downloaded',
    useModel: (p) => `Use ${p.name}`,
    useGuide: 'Answer from the guide only',
    removeModel: 'Remove the downloaded model',
    loadingModel: (p) => `Preparing the model… ${p.pct} %`,
    modelFailed: (p) => `The model could not start (${p.msg}). The guide answers instead.`,
    noWebgpu: 'This device cannot run a language model (no WebGPU). The app\'s guide answers.',
    modelNote: 'The model is downloaded once and stays on the device. Better use Wi-Fi for the download.',
    aiDisclaimer: 'The model works from the guide and your last check but can be wrong. If in doubt, contact support.',
    sourcesLabel: 'Sources',
    fromGuide: 'From the guide:',
    related: 'Related:',
    lastCheck: (p) => `Your last check: ${p.title}`,
    noMatch: 'I found nothing about that in the guide. Try asking differently, or contact support.',
    you: 'You',
    assistantName: 'Assistant',
    chips: [
      'My internet does not work. What should I do?',
      'How do I restart the router and antenna properly?',
      'Why is the internet slow?',
      'I replaced my router and it does not work',
      'What does the check result mean?',
    ],
    f: {
      notRouted: (p) => [`${p.ip} is not routed in the hkfree.org network`, [
        'No router in OSPF advertises a subnet containing this address.',
        ...p.previous.map((x) => `${x.prefix} was advertised by router ${label(x.router, x.name)} (last seen ${x.agoText}); the router ${x.present ? 'is still in OSPF but no longer advertises it' : 'is missing from OSPF (down?)'}.`),
      ]],
      noLsa: (p) => [`Router ${label(p.router, p.name)} is missing from OSPF`, ['A neighbour still lists its subnet, but the router itself is gone: it is down.']],
      cutOff: (p) => [`AP router ${label(p.router, p.name)} is cut off from the network`, [
        `It still advertises ${p.advertised} link(s) but no neighbour confirms any of them.`,
        `Its record is ${p.age} s old: the router or all of its links are down.`,
      ]],
      noExit: (p) => [`No internet exit is reachable from ${label(p.router, p.name)}`, [`The part of the network containing this router (${p.count} router(s)) has no internet exit.`]],
      hopDead: (p) => [`Router ${label(p.router, p.name)} on the path does not answer ping`, ['Sojka recently lost it, but its OSPF links are up: traffic most likely still passes through it. If problems persist, it is the first suspect.']],
      hopDegraded: (p) => [`Router ${label(p.router, p.name)} on the path: ${p.healthText}`, []],
      spof: (p) => ['No backup route at', [
        ...p.links.map((l) => `link ${label(l.a, l.an)} → ${label(l.b, l.bn)}`),
        ...p.nodes.map((n) => `router ${label(n.router, n.name)}`),
      ]],
      singleExit: (p) => ['Only one internet exit is reachable', [`${p.reachable} of ${p.total} exits reachable.`]],
    },
  },
};

export function messages(lang) {
  return M[LANGS.includes(lang) ? lang : 'cs'];
}

/** Every key must exist in both languages (checked by the unit tests). */
export function missingKeys() {
  const out = [];
  const walk = (a, b, path) => {
    for (const k of Object.keys(a)) {
      if (!(k in b)) out.push(path + k);
      else if (a[k] && typeof a[k] === 'object' && !Array.isArray(a[k])) walk(a[k], b[k], `${path}${k}.`);
    }
  };
  walk(M.cs, M.en, 'en:');
  walk(M.en, M.cs, 'cs:');
  return out;
}
