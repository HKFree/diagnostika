# How to verify a deployment

After deploying, check from the outside that everything members depend on
works, outage reporting included:

```
hkdiag verify https://diag.hkfree.org
```

It needs no config. From a checkout, `cargo build --release` first, then
`npm run verify:live -- https://diag.hkfree.org` does the same.

It checks:
- that the app and every file it caches offline are served;
- that the server sees the client's own address (the proxy passes it on);
- that the network map is fresh;
- that outage reports are enabled, pass the proxy and the origin check, and
  that reports from another site are refused;
- that a **trial report** would be stored;
- that the server is reading UserDB's status (what network monitoring finds)
  and the power monitor: their last read worked, within the last 15 minutes;
- that a router's measurement history comes through;
- that the public API is described.

Each check prints `PASS`, `FAIL`, or `SKIP` (doesn't apply, e.g. no proxy in
front of a local server), with a detail. It exits non-zero if any check fails.

## Every hour

On lide a systemd timer runs the same checks every hour
(`deploy/systemd/hkdiag-verify.{service,timer}`):

```
install -m 644 deploy/systemd/hkdiag-verify.* ~/.local/share/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now hkdiag-verify.timer
```

On lide `~/.config` is a file, so `enable` can't create its link there; link it
next to the unit instead:

```
d=~/.local/share/systemd/user
mkdir -p "$d/timers.target.wants"
ln -sf "$d/hkdiag-verify.timer" "$d/timers.target.wants/"
systemctl --user daemon-reload && systemctl --user start hkdiag-verify.timer
```

A failed check fails the unit: `systemctl --user --failed` lists it, and
`journalctl --user -u hkdiag-verify` shows which check failed and why.

## The trial report

A trial report (header `X-Hkdiag-Trial: 1`) goes through every check a
member's report goes through, and is written to the report database inside a
transaction that is then rolled back. Nothing is kept. It first sends a
trial that no server would store. Only when the server answers that it knows
trial reports does it send the real trial, so an older server never stores
it.

## The server log

On lide, the server's log is in the journal:

```
journalctl --user -u hkdiag -f | grep -E 'access:|reports'
```

- `access: GET /topology.json 200 12ms hkfree`: one line per request, with the
  client's network (`hkfree`, `vpn`, `other`, `local`, or `unknown` before the
  first network map is built), never its address.
- `reports: 1 received from hkfree, 1 stored, 0 already stored, 0 refused`:
  what happened to each batch of outage reports a member's app sent, with the
  reason for anything refused. Trial reports say `(trial, nothing kept)`.

Turn the access lines off with `access_log = false` under `[web]`.
