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
phone_hours = "9-17"           # working days; "8:30-16:30" also works
slack_channel = "#nejdeto"     # on the hkfree.org Slack
```

The server refuses to start with an invalid value. The contact is part of
`topology.json`, so installed apps also show it offline. Without `[support]`
the app says *Kontaktujte správce svého AP nebo podporu hkfree.org*.


## Speed test

Advanced diagnostics can measure the connection's speed against a LibreSpeed
backend: response time, jitter, download and upload. It runs only when the
member taps it, only over hkfree.org, for up to 8 s each way and at most
250 MB down and 100 MB up.

```toml
[speedtest]
url = "https://speedtest.hkfree.org/backend/"   # https, ending with /
```

The app calls the backend's `empty.php` and `garbage.php` with `?cors=true`,
which makes LibreSpeed send the CORS headers it needs; nothing else on the
speed test server has to change. It doesn't use `getIP.php` (with `isp=true`
that asks an outside service).

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
- The URL must be `https://`; credentials are sent with every request. Plain
  `http://` is accepted only on loopback (`127.0.0.1`, `localhost`, `[::1]`),
  for the tests' stand-in UserDB.
- The advanced diagnostics show the member's AP: name, area and street
  address. With a key bound to one AP, only that AP's routers are named.
- **AP admins.** The area admin (SO) and deputy (ZSO) are shown above the
  support contact. Their nicknames are in `topology.json`, but their e-mails
  never are. A device gets the e-mails from `/api/contact`, and only for the
  AP its own address belongs to (the address is the connection's, or
  `X-Real-IP` from a trusted proxy). The app asks during each check on
  hkfree.org and keeps the answer, so the e-mail is still there when the
  network later fails.
- **Antenna signal.** On a member's request, advanced diagnostics show how
  strongly their antenna receives the AP (good from -70 dBm, usable to -80,
  weak below). The server reads it through UserDB's Wewimo, which logs in to
  every Wewimo-enabled router of the AP and leaves a line in its log. So one
  AP is read at most once per `[web] signal_cache_secs` (60 s), one AP at a
  time. A device only gets its own antenna: the station whose last IP is its
  address, or an address the member records give to the same member.
- For `hkdiag diagnose --member`, also set `uid` and `password` (or
  `HKDIAG_USERDB_PASSWORD`). UserDB locks an address out after 5 failed logins
  in 15 minutes, so hkdiag never retries a login.
