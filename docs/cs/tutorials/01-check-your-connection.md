# Tutoriál: zkontrolujte své připojení aplikací

V tomto tutoriálu si do telefonu nainstalujete diagnostiku hkfree.org,
spustíte první kontrolu a uvidíte, co si aplikace pamatuje, aby vám mohla
pomoct i později, až vám spojení nepůjde. Zabere to asi pět minut.

Potřebujete telefon (nebo notebook) připojený k domácí síti hkfree.org a
adresu aplikace od svých správců.

## 1. Otevřete aplikaci doma

Připojte telefon k domácí Wi-Fi a otevřete adresu aplikace v prohlížeči.
Aplikace sama spustí kontrolu.

Vpravo nahoře uvidíte odznak. Měl by ukazovat **hkfree.org**: aplikace
poznala, že jste v síti hkfree.org. Pokud ukazuje **jiná síť**, jste na
mobilních datech; přepněte se na domácí Wi-Fi.

## 2. Přečtěte si první výsledek

Po několika sekundách velký rámeček ukáže výsledek, například **Připojení
funguje**. Pod ním je **Co dělat** s dalšími kroky, pokud nějaké jsou.

Sjeďte níž. Uvidíte čtyři testy (zařízení, server hkfree.org, internet podle
IP adresy, internet podle jména) a pod nimi **Vaše cesta**: routery, přes které
vede vaše spojení, od vašeho AP routeru po výstup do internetu. Většina ukazuje
**dostupný**, některé **nelze otestovat**; to je v pořádku.

![Kontrola](../../screenshots/11-cs-healthy-hkfree-phone.png)

## 3. Spusťte ji ještě jednou

Stiskněte **Zkontrolovat znovu**. Aplikace si teď zapamatuje, které routery
na vaší cestě obvykle odpovídají. Díky tomu později pozná, *kde* je výpadek.

## 4. Podívejte se na svou cestu

Otevřete záložku **Cesta**. Je v ní vaše cesta do internetu se stavem
každého routeru podle dohledu sítě a pod ní **Zjištění**, například routery
bez záložní cesty.

## 5. Nainstalujte aplikaci

> Na to musí aplikace běžet na adrese HTTPS. Dokud běží na
> `http://10.107.252.72:9000`, nabídka instalace se neobjeví a aplikace
> nefunguje bez připojení; tento krok přeskočte, dokud hkfree.org nebude mít
> doménu s HTTPS.

Stiskněte **Nainstalovat aplikaci**, nebo v prohlížeči zvolte „Přidat na
plochu“. Aplikace se pak otevírá jako každá jiná, i když nemáte připojení.

## 6. Zeptejte se poradce

Otevřete **Pomoc** a zvolte navrženou otázku, například *Jak správně
restartovat router a anténu?* Odpověď pochází z průvodce přímo v aplikaci a
funguje i bez připojení.

## Co jste zvládli

Aplikace teď zná vaši adresu, vaši cestu a to, jak vaše routery obvykle
odpovídají. Až vám příště spojení nepůjde:

1. Otevřete aplikaci (z plochy; funguje i bez připojení).
2. Přečtěte si výsledek a postupujte podle **Co dělat**.
3. Pokud byl problém v síti, aplikace si ho uloží a po obnovení spojení ho
   nahlásí správcům. Nahlášené výpadky ukazuje záložka **Výpadky**.
