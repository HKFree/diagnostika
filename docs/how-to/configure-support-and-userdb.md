# How to configure the support contact and UserDB

## Support contact

The app shows the contact under results where the problem is on the network's
side (AP router, backbone, known problem, no connection, internet down). Add
it to the server config and restart:

```toml
[support]
name = "Podpora hkfree.org"
url = "https://hkfree.org"     # must start with https://
email = "podpora@hkfree.org"   # a plain address, no ?subject= etc.
phone = "608 10 10 70"         # digits, spaces, + - ( )
```

The server refuses to start with an invalid value. The contact is part of
`topology.json`, so installed apps also show it offline. Without `[support]`
the app says *Kontaktujte správce svého AP nebo podporu hkfree.org*.

> The specification also wants the phone's opening hours (working days 9–17)
> and the `#nejdeto` channel on the hkfree.org Slack; the config can't express
> them yet (`features/kontrola/kontrola-pripojeni.feature`, `@backlog`).

## UserDB (router and AP names)

With an API key, the snapshot shows AP names ("area / AP") instead of bare
router addresses.

```toml
[userdb]
api_key_id = 123          # the HTTP user is apikey123
api_key = "…"             # or the HKDIAG_USERDB_API_KEY environment variable
```

- Only infrastructure (AP-owned) addresses are used for names. Member devices
  never go into the snapshot.
- The URL must be `https://`; credentials are sent with every request.
- For `hkdiag diagnose --member`, also set `uid` and `password` (or
  `HKDIAG_USERDB_PASSWORD`). UserDB locks an address out after 5 failed logins
  in 15 minutes, so hkdiag never retries a login.
