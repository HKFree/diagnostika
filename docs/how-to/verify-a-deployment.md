# How to verify a deployment

After deploying, check from the outside that everything members depend on
works, outage reporting included:

```
npm run verify:live -- https://diag.hkfree.org
```

It checks:
- that the app and every file it caches offline are served;
- that the server sees the client's own address (the proxy passes it on);
- that the network map is fresh;
- that outage reports are enabled, pass the proxy and the origin check, and
  that reports from another site are refused;
- that a **trial report** would be stored;
- that network monitoring's outages are being recorded, and that a router's
  measurement history comes through;
- that the public API is described.

Each check prints `PASS` or `FAIL` with a detail. The script exits non-zero if
any check fails.

## The trial report

A trial report (header `X-Hkdiag-Trial: 1`) goes through every check a
member's report goes through, and is written to the report database inside a
transaction that is then rolled back. Nothing is kept. The script first sends a
trial that no server would store. Only when the server answers that it knows
trial reports does the script send the real trial, so an older server never
stores it.

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
