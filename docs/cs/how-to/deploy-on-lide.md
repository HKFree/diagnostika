# Návod: nasazení nebo aktualizace hkdiag na lide.hkfree.org

hkdiag běží na lide jako **uživatelská** služba systemd uživatele `jrk`.
Naslouchá na adrese v hkfree.org `10.107.252.72:9000` pro členy a na
`127.0.0.1:9000` pro reverzní proxy Apache na serveru; viz
[HTTPS za Apachem](serve-over-https.md).

| Co | Kde |
|---|---|
| Program | `~/hkdiag/bin/hkdiag` |
| Konfigurace (práva 600) | `~/hkdiag/config.toml` |
| Historie prefixů, `reports.sqlite` | `~/hkdiag/data/` |
| Kopie zdrojových kódů | `~/hkdiag/src/` |
| Jednotka | `~/.local/share/systemd/user/hkdiag.service` |

> Na lide je `~/.config` obyčejný soubor, ne adresář. Nikdy pod něj nic
> nezapisujte; proto je všechno v `~/hkdiag/` a jednotka v
> `~/.local/share/systemd/user/`.

## Aktualizace na nové sestavení

Sestavte lokálně (na lide Rust není). Program potřebuje glibc 2.34 nebo
novější; lide má 2.39.

```
cargo build --release
rsync -a --delete --exclude target/ --exclude .git/ ./ jrk@lide.hkfree.org:hkdiag/src/
rsync -a target/release/hkdiag jrk@lide.hkfree.org:hkdiag/bin/hkdiag.new
ssh jrk@lide.hkfree.org 'mv ~/hkdiag/bin/hkdiag.new ~/hkdiag/bin/hkdiag && systemctl --user restart hkdiag'
```

Kontrola:

```
ssh jrk@lide.hkfree.org 'systemctl --user status hkdiag --no-pager | head -5'
curl -s http://10.107.252.72:9000/api/whoami
```

Nainstalované aplikace si novou verzi převezmou samy. Název mezipaměti
service workeru obsahuje otisk všech souborů aplikace a nová verze převezme
řízení, jakmile starou nepoužívá žádná stránka.

## Instalace od začátku

1. Nahrajte program a zdrojové kódy jako výše a z `config.example.toml`
   vytvořte `~/hkdiag/config.toml` (práva 600). Nastavte:

   ```toml
   [web]
   listen = ["127.0.0.1:9000", "10.107.252.72:9000"]
   trusted_proxies = ["127.0.0.1", "::1"]  # X-Real-IP smí nastavit jen místní reverzní proxy
   reports_db = "/home/jrk/hkdiag/data/reports.sqlite"
   ```

2. Vytvořte `~/.local/share/systemd/user/hkdiag.service`:

   ```ini
   [Unit]
   Description=hkdiag - hkfree.org connectivity diagnostics
   After=network-online.target

   [Service]
   Environment=XDG_STATE_HOME=%h/hkdiag/data
   ExecStart=%h/hkdiag/bin/hkdiag --config %h/hkdiag/config.toml serve
   Restart=on-failure
   RestartSec=5
   NoNewPrivileges=yes
   PrivateTmp=yes
   UMask=0077

   [Install]
   WantedBy=default.target
   ```

3. `systemctl --user enable` nemůže zapisovat pod `~/.config`, takže jednotku
   propojte ručně, nechte uživatelského správce běžet i po odhlášení a
   spusťte ji:

   ```
   mkdir -p ~/.local/share/systemd/user/default.target.wants
   ln -sf ../hkdiag.service ~/.local/share/systemd/user/default.target.wants/hkdiag.service
   loginctl enable-linger jrk
   systemctl --user daemon-reload
   systemctl --user start hkdiag
   ```

   `systemctl --user is-enabled hkdiag` hlásí `disabled`; to je jen kosmetické.
   `systemctl --user list-dependencies default.target` ukáže, že se služba při
   startu spouští.

## Zastavení nebo odstranění

```
systemctl --user stop hkdiag
rm ~/.local/share/systemd/user/default.target.wants/hkdiag.service   # nespouštět při startu
```

Zatoulanou instanci ukončete podle PID (`pgrep -x hkdiag`). Nepoužívejte
`pkill -f` se vzorem, který je i ve vašem vlastním příkazu: ukončí i váš shell.
