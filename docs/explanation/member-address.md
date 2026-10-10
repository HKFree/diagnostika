# How the app learns the member's address, and when it can't

Everything the app says about "your connection" (your AP router, your path,
the hkfree.org / other network badge, outage reports) depends on one fact:
the member's address in hkfree.org. A browser can't see that address itself.
A phone at home usually has a private address from the member's own router
(for example `192.168.1.23`), while hkfree.org knows the member by the address
of that router. So the app asks the server: `GET /api/whoami` returns the
address the request **arrived from**.

The app then decides:

| whoami says | Network map knows the address | App shows | "Your address" |
|---|---|---|---|
| an IPv4 address | yes | **hkfree.org** | that address (remembered) |
| an IPv4 address | no | **jiná síť** (other network) | the last remembered hkfree.org address |
| nothing, or IPv6 | – | **síť ?** (unknown) | the last remembered one, or it asks |
| unreachable | – | **síť ?** | the last remembered one (offline check) |

An address entered by hand (under advanced diagnostics) always takes
precedence over the detected one.

## Cases where it is hard to tell

### The server can't tell at all

- **Local testing.** Opened at `127.0.0.1`, the server sees loopback. It
  answers "unknown" on purpose rather than `127.0.0.1`, and the app asks for
  the address.
- **IPv6.** The device reaches the server over IPv6. The network map holds
  IPv4 only, so the app treats it as unknown. Only an IPv4 path to the server
  (or entering the address) helps.
- **A proxy that doesn't say who the client is.** Behind a reverse proxy,
  the server believes only the proxy's `X-Real-IP`, and only from
  `trusted_proxies`. If the proxy sends none, or two of them, the answer is
  "unknown" rather than the proxy's own address.
- **Static hosting without whoami.** Served as plain files without the nginx
  `/api/whoami` line, nothing tells the address; the member enters it once.

### The server tells, but the answer misleads

- **A proxy not listed in `trusted_proxies`.** The server reports the
  proxy's address for every member. Every member then shows the same path.
- **The future HTTPS domain resolving to a public address.** If members inside
  hkfree.org reach the app through a public address and their traffic passes
  through an hkfree.org NAT or the internet exit, the server sees the NAT's
  or exit's address, not the member's. Then either nobody is "on hkfree.org",
  or everyone has the exit's path. Fix: the domain must resolve to lide's
  hkfree.org address for members inside (split DNS), or the proxy in front
  must see the members' real hkfree.org addresses.
- **hkfree.org VPN.** A member connected through the hkfree.org VPN gets an
  address from `10.107.254.0/24`. The map knows that pool, so the app says
  "hkfree.org", but the path is the VPN server's, not the member's home
  connection. Diagnosing the home connection over the VPN isn't possible.
- **Several members behind one address.** A building network or a shared
  router behind one hkfree.org address: everyone gets that address's path,
  which is correct for the shared part only.
- **Visiting another member.** On a friend's Wi-Fi the server sees the
  friend's address. The app then remembers that address as "yours" until you
  check from home again.

### The member's own choices

- **An address entered by hand stays.** It overrides detection everywhere,
  also after moving or on mobile data, until **Detect automatically** under
  advanced diagnostics is chosen.
- **Mobile data or another provider.** The server sees the provider's address,
  which isn't in the map, so the app shows **jiná síť** and keeps the last
  known hkfree.org address. That's correct, but the path shown is then your
  home path, not the one you're using.

### Without a connection

- **Offline, the app can't ask at all.** It uses the last remembered address.
  If the device last connected somewhere else (a friend, the VPN), the
  offline check examines that place's path, not the one the member is at now.

## What this means for deployment

- Put `hkdiag serve` behind a proxy listed in `trusted_proxies` that
  **overwrites** `X-Real-IP` with the real client address.
- Make sure members inside hkfree.org reach the app on an hkfree.org address,
  not through NAT. Today that's `http://10.107.252.72:9000`; keep it that way
  for the HTTPS domain.
- Treat VPN addresses as "not your home connection" when reading reports.
