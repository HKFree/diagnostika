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
| The client's address in a header the client can't forge | The member's address is how the app finds their path, AP and admin's e-mail. A client-supplied value would let anyone pose as any member | nothing: hkdiag reads the address Apache appends to `X-Forwarded-For` (`client_ip_header = "x-forwarded-for"`); or `RequestHeader set X-Real-IP "expr=%{REMOTE_ADDR}"` with the default `x-real-ip` |
| The original `Host` header, or the public origin listed | Outage reports are accepted only from this server's own pages | `ProxyPreserveHost On`; or, without changing Apache, `origins = ["https://diag.hkfree.org"]` in `[web]` (as on lide) |
| No caching | Each response sets `Cache-Control` itself; the service worker does the caching | don't add `mod_cache` for this host |

hkdiag believes a client-address header only from the addresses in
`trusted_proxies`, and only the one named by `client_ip_header`:

- `"x-forwarded-for"`: the **last** entry of `X-Forwarded-For`, which Apache's
  proxy module appends by itself. Earlier entries (and any `X-Real-IP`) may be
  forged by the client and are ignored. This is what lide uses: its Apache
  needed no change. It assumes exactly one proxy in front of hkdiag.
- `"x-real-ip"` (default): `X-Real-IP`, which the proxy must **overwrite**.
  Apache passes a client's own `X-Real-IP` on unchanged unless told to set
  it, so with this mode and an unconfigured Apache, anyone could pose as any
  member.

A trusted proxy that sends no usable address makes the address unknown, rather
than every member looking like the proxy.

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

hkdiag listens on localhost for the proxy, and can keep its hkfree.org
address for members who reach it directly. Only the proxy is trusted to set
`X-Real-IP`; a header sent to the hkfree.org address is ignored:

```toml
[web]
listen = ["127.0.0.1:9000", "10.107.252.72:9000"]
trusted_proxies = ["127.0.0.1", "::1"]
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
