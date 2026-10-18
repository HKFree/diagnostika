# How to use the public API

hkdiag publishes what it measures about the network for other programs:
without login, readable from other sites (CORS), and without anything that
identifies a member.

- **Find it:** `GET /.well-known/api-catalog` ([RFC 9727](https://www.rfc-editor.org/rfc/rfc9727))
  points to the description, `GET /api/openapi.json` (OpenAPI 3).
- **Endpoints:**
  - `/api/v1/network`: the network map (routers, links, prefixes, exits,
    lost links, per-AP devices with their monitoring health).
  - `/api/v1/outages`: member reports and measured outages of the last week,
    what monitoring sees now, and the power situation.
  - `/api/v1/measured?since=<unix time>`: outages network monitoring
    measured, with start and end (90 days kept).
  - `/api/v1/reports?since=<unix time>`: outages members reported, without
    their addresses.
- `since` defaults to a week ago and reaches back at most 90 days.

```
curl -s https://diag.hkfree.org/.well-known/api-catalog
curl -s "https://diag.hkfree.org/api/v1/measured?since=$(date -d yesterday +%s)"
```

Not part of the API, and not readable from other sites: answers about the
requesting device itself (`/api/whoami`, `/api/contact`, `/api/signal`).
