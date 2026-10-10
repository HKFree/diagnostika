# How to serve the app over HTTPS behind Apache

A phone only installs the app and enables offline mode on an HTTPS origin. On
lide.hkfree.org the app will run behind the server's Apache, which terminates
TLS. **Apache isn't configured by hkdiag**: its admin sets up the virtual
host. This page lists what hkdiag needs from it, with a configuration tested
against hkdiag.

## What hkdiag needs from the proxy

| Need | Why | Apache |
|---|---|---|
| Its own host name, proxied from `/` | The app uses absolute paths (`/app.js`, `/topology.json`), so it can't live under a sub-path | `ProxyPass / …` |
| `X-Real-IP` with the client's address, **overwriting** any value the client sent | The member's address is how the app finds their path, AP and admin's e-mail. A client-supplied value would let anyone pose as any member | `RequestHeader set X-Real-IP "expr=%{REMOTE_ADDR}"` |
| The original `Host` header | Outage reports are accepted only when the browser's `Origin` matches `Host` | `ProxyPreserveHost On` |
| No caching | Each response sets `Cache-Control` itself; the service worker does the caching | don't add `mod_cache` for this host |

hkdiag ignores `X-Forwarded-For`, which Apache adds by itself. It believes
`X-Real-IP` only from the addresses in `trusted_proxies`. A trusted proxy that
sends no `X-Real-IP` makes the address unknown, rather than every member
looking like the proxy.

> `RequestHeader set X-Real-IP "%{REMOTE_ADDR}s"`, often seen online, is
> wrong: in `mod_headers`, `%{…}s` reads an SSL variable, so the header ends
> up empty and every member's address is unknown.

## Example virtual host

`diag.hkfree.org` stands for whatever name the app gets.

Modules: `proxy`, `proxy_http`, `headers` (on lide only `headers` and `ssl`
are enabled today). The certificate is set up as for the other `*-le-ssl.conf`
hosts.

```apache
<VirtualHost *:443>
    ServerName diag.hkfree.org
    # SSLEngine / SSLCertificateFile / SSLCertificateKeyFile: as the other -le-ssl hosts

    ProxyPreserveHost On
    ProxyPass        / http://127.0.0.1:9000/
    ProxyPassReverse / http://127.0.0.1:9000/
    RequestHeader set X-Real-IP "expr=%{REMOTE_ADDR}"

    # Optional: only the hkfree.org network (see the note below).
    # <Location />
    #     Require ip 10.107.0.0/16
    # </Location>
</VirtualHost>

<VirtualHost *:80>
    ServerName diag.hkfree.org
    Redirect permanent / https://diag.hkfree.org/
</VirtualHost>
```

## hkdiag's side

Once the virtual host is in place, move hkdiag off the public address and
restart it:

```toml
[web]
listen = "127.0.0.1:9000"
trusted_proxies = ["127.0.0.1"]
```

Then open `https://diag.hkfree.org/api/whoami` from a member device: it must
show the device's own address, not `null` and not `127.0.0.1`. Sending a fake
`X-Real-IP` with curl must not change the answer.

Notes:

- If the host name also has an AAAA record, members on IPv6 reach it over
  IPv6. The app then locates them by their OSPFv3 prefix.
- Restricting the host to `10.107.0.0/16` keeps the network map private, but
  members on mobile data then can't refresh the app. Their installed copy
  still works offline.
