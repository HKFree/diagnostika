# The whole network map goes to every member's device

- Status: accepted
- Date: 2026-10-10
- Shaped views: `index`, `diagnostika`

## Context and decision

The check must work when the member has no connection at all, so the device
needs the network map before an outage, not during it. We send every device
the **whole** hkfree.org network map (routers, links, which addresses belong
where, internet exits), and the device works out paths itself.

## Considered options

- **Whole map (chosen):** the best offline coverage. Any path can be worked
  out on the device, also for an address the member didn't foresee.
- **Only the member's own path:** keeps other members' APs private, but
  offline the device only knows the one path it fetched last time.
- **Map without address ownership:** hides which address is where, but
  neighbours' APs stay guessable, and every offline lookup of an address fails.

## Consequences

- Any member can see which AP any address connects through. The
  specification accepts this: "anyone sees the path of any address".
- Members' devices, descriptions and contacts never go into the map. Only
  infrastructure does.
- Reports with members' addresses stay on the server, behind an admin login.
