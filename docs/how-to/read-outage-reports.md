# How to read outage reports

Reports are stored in SQLite on the server (on lide:
`~/hkdiag/data/reports.sqlite`).

## List and rank

```
hkdiag --config ~/hkdiag/config.toml reports --hours 24
```

Each line shows the detection time (Unix seconds), duration, verdict, the
member's address, their AP router, the first router that stopped answering
and the last one that still did. At the end, the most often failing routers
are listed.

The database path comes from `[web] reports_db` (on lide
`/home/jrk/hkdiag/data/reports.sqlite`), or else
`$XDG_STATE_HOME/hkdiag/reports.sqlite`.

## Machine-readable

```
hkdiag reports --hours 168 --json | jq '.[] | {detected_at, verdict, first_bad}'
```

## Query the database directly

```
sqlite3 ~/hkdiag/data/reports.sqlite \
  "SELECT first_bad, COUNT(*) FROM reports GROUP BY first_bad ORDER BY 2 DESC LIMIT 10"
```

The table is created in `src/reports.rs` (`Store::open`).

## What members see

The **Výpadky** (Outages) tab shows the last 7 days from `GET /api/outages`:
time, duration, verdict, AP and routers, never member addresses.
