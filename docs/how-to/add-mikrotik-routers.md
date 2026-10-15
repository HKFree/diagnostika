# How to add MikroTik routers as extra views of the OSPF database

hkdiag builds the network map from FRR's OSPF database (10.107.3.1). It can
also read the OSPF database of MikroTik routers (RouterOS 7) with a read-only
account, and merge them into the map. Each router shows the network from where
it stands:

- **Inside its area.** A border router sees the routers of its own area, which
  FRR only knows as summary routes. Members there then see their whole path,
  not just the border router. See [hidden parts](../explanation/hidden-parts.md).
- **Its side of a split.** Records are merged newest first by sequence number,
  so during a split each side contributes its own fresh view.
- **Its default route.** The snapshot's `sources` list says, per router,
  whether it has an active `0.0.0.0/0`.

If FRR can't be read, the map is built from the MikroTik routers alone.

## Add a router

1. On the router, an account in the `read` group (read-only) and the `www`
   service (REST API) enabled. `www-ssl` with a certificate is better: the
   login then isn't sent in the clear.
2. Check from the server:

   ```
   printf 'user = "read:PASSWORD"\n' | curl -s -K - http://10.107.10.129/rest/routing/ospf/instance
   ```

   It must list an instance with `"version":"2"`.
3. Add it to the config (mode 600) and restart hkdiag:

   ```toml
   [[mikrotik]]
   url = "https://10.107.10.129"
   user = "read"
   password = "…"
   ```

   A router without `www-ssl` only answers on `http://`, which sends the login
   unencrypted. hkdiag refuses that unless the entry also says
   `insecure_http = true`, and then warns about it at start.

4. Check that `topology.json` lists it under `sources` with `"ok": true`.

Notes:

- Only the IPv4 (OSPFv2) instances are read. The OSPFv3 data still comes from
  `frr.ospf6_host`.
- A router that doesn't answer is skipped for `retry_after_secs` (5 minutes)
  and the map is built without it.
- Each read is a few hundred kB; the map is rebuilt at most every
  `topology_cache_secs`.
- The accounts could later come from UserDB, which already stores logins per
  router address.
