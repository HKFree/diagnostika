# Návod: čtení hlášení výpadků

Hlášení jsou uložená v SQLite na serveru (na lide:
`~/hkdiag/data/reports.sqlite`).

## Výpis a pořadí

```
hkdiag --config ~/hkdiag/config.toml reports --hours 24
```

Každý řádek ukazuje čas zjištění (sekundy Unixu), trvání, výsledek, adresu
člena, jeho AP router, první router, který přestal odpovídat, a poslední,
který ještě odpovídal. Na konci jsou nejčastěji selhávající routery.

Cesta k databázi se bere z `[web] reports_db` (na lide
`/home/jrk/hkdiag/data/reports.sqlite`), jinak
`$XDG_STATE_HOME/hkdiag/reports.sqlite`.

## Strojově čitelně

```
hkdiag reports --hours 168 --json | jq '.[] | {detected_at, verdict, first_bad}'
```

## Dotaz přímo do databáze

```
sqlite3 ~/hkdiag/data/reports.sqlite \
  "SELECT first_bad, COUNT(*) FROM reports GROUP BY first_bad ORDER BY 2 DESC LIMIT 10"
```

Tabulku vytváří `src/reports.rs` (`Store::open`).

## Co vidí členové

Záložka **Výpadky** ukazuje poslední týden z `GET /api/outages`: čas,
trvání, výsledek, AP a routery, nikdy adresy členů.
