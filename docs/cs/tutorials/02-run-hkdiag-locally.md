# Tutoriál: spusťte hkdiag na svém počítači

V tomto tutoriálu sestavíte hkdiag, prohlédnete si živou páteř hkfree.org,
diagnostikujete adresu z příkazové řádky a otevřete aplikaci v prohlížeči.
Zabere to asi patnáct minut.

Potřebujete počítač s Linuxem v síti hkfree.org (musí se dostat na
`10.107.3.1`), Rust 1.75 nebo novější a heslo k rozhraní vty FRR pro čtení.

## 1. Sestavení

```
git clone git@github.com:HKFree/diagnostika.git
cd diagnostika
cargo build --release
```

Použijte `Cargo.lock` tak, jak je v repozitáři: drží verze, které se s Rustem
1.75 sestaví.

## 2. Konfigurace

```
mkdir -p ~/.config/hkdiag
install -m 600 config.example.toml ~/.config/hkdiag/config.toml
```

V `~/.config/hkdiag/config.toml` nastavte `[frr] password` a sekci `[userdb]`
zatím smažte (potřebuje API klíč). hkdiag odmítne konfigurační soubor, který
mohou číst jiní uživatelé; proto práva 600.

## 3. Prohlédněte si páteř

```
./target/release/hkdiag topology | head
```

Uvidíte asi 200 routerů s počty sousedů a na konci výstupy do internetu
(`exit: … E1/E2`). Je to živá databáze OSPF, přečtená z routeru `10.107.3.1`.

## 4. Diagnostikujte adresu

Vyberte adresu člena a spusťte:

```
./target/release/hkdiag diagnose 10.107.77.5
```

Výstup ukáže, do které podsítě a ke kterému routeru adresa patří, cestu do
internetu s cenami OSPF a výsledky pingu, zjištění a na konci výsledek.
Zkuste i adresu, která neexistuje, třeba `10.107.77.240`: uvidíte, že není
směrovaná.

## 5. Otevřete aplikaci

```
./target/release/hkdiag serve
```

Otevřete <http://127.0.0.1:8087>. Localhost se počítá jako zabezpečený
kontext, takže funguje celá aplikace včetně provozu bez připojení. Server vás
vidí jako `127.0.0.1`, proto se aplikace zeptá na vaši adresu v hkfree.org:
zadejte tu z kroku 4 a stiskněte **Použít**.

Spusťte kontrolu dvakrát a otevřete záložku **Cesta**. Je tam stejná cesta
jako v kroku 4, jen ji spočítal prohlížeč z `topology.json`.

## 6. Vyzkoušejte provoz bez připojení

V nástrojích pro vývojáře v prohlížeči přepněte síť na *Offline* a stránku
obnovte. Aplikace se přesto otevře a provede kontrolu podle uložené mapy sítě.

## Co jste zvládli

Viděli jste obě poloviny hkdiag: server, který čte OSPF a vydává mapu sítě, a
aplikaci, která ji rozebírá v zařízení. Pokračujte:

- [Nasazení nebo aktualizace na lide.hkfree.org](../how-to/deploy-on-lide.md)
- Architektura: `docs/architecture/` (`npx likec4 serve docs/architecture`) a
  rozhodnutí v `docs/adrs/`
