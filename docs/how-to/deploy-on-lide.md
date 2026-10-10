# How to deploy or update hkdiag on lide.hkfree.org

hkdiag runs on lide as a systemd **user** service of `jrk` and listens on the
hkfree.org address only: `10.107.252.72:9000`. In the end it moves behind the
server's Apache; see [serve over HTTPS behind Apache](serve-over-https.md).

| What | Where |
|---|---|
| Binary | `~/hkdiag/bin/hkdiag` |
| Config (mode 600) | `~/hkdiag/config.toml` |
| Prefix history, `reports.sqlite` | `~/hkdiag/data/` |
| Source copy | `~/hkdiag/src/` |
| Unit | `~/.local/share/systemd/user/hkdiag.service` |

> On lide, `~/.config` is a regular file, not a directory. Never write under
> it; that's why everything lives in `~/hkdiag/` and the unit lives in
> `~/.local/share/systemd/user/`.

## Update to a new build

Build locally (lide has no Rust toolchain). The binary needs glibc ≥ 2.34;
lide has 2.39.

```
cargo build --release
rsync -a --delete --exclude target/ --exclude .git/ ./ jrk@lide.hkfree.org:hkdiag/src/
rsync -a target/release/hkdiag jrk@lide.hkfree.org:hkdiag/bin/hkdiag.new
ssh jrk@lide.hkfree.org 'mv ~/hkdiag/bin/hkdiag.new ~/hkdiag/bin/hkdiag && systemctl --user restart hkdiag'
```

Check it:

```
ssh jrk@lide.hkfree.org 'systemctl --user status hkdiag --no-pager | head -5'
curl -s http://10.107.252.72:9000/api/whoami
```

Installed apps pick up the new version by themselves. The service worker's
cache name contains a hash of all app files, and a new version takes over once
no page uses the old one.

## Install from scratch

1. Copy the binary and source as above, and write `~/hkdiag/config.toml`
   (mode 600) from `config.example.toml`. Set:

   ```toml
   [web]
   listen = "10.107.252.72:9000"
   trusted_proxies = []
   reports_db = "/home/jrk/hkdiag/data/reports.sqlite"
   ```

2. Create `~/.local/share/systemd/user/hkdiag.service`:

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

3. `systemctl --user enable` can't write under `~/.config`, so link the unit
   by hand, keep the user manager running after logout, and start it:

   ```
   mkdir -p ~/.local/share/systemd/user/default.target.wants
   ln -sf ../hkdiag.service ~/.local/share/systemd/user/default.target.wants/hkdiag.service
   loginctl enable-linger jrk
   systemctl --user daemon-reload
   systemctl --user start hkdiag
   ```

   `systemctl --user is-enabled hkdiag` will say `disabled`; that's cosmetic.
   `systemctl --user list-dependencies default.target` shows it is pulled in
   at boot.

## Stop or remove

```
systemctl --user stop hkdiag
rm ~/.local/share/systemd/user/default.target.wants/hkdiag.service   # no start at boot
```

To stop a stray instance, kill it by PID (`pgrep -x hkdiag`). Don't use
`pkill -f` with a pattern that also appears in your own command line: it kills
your shell too.
