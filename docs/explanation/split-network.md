# When the network splits

When a link fails and no other route connects the two sides, the hkfree.org
network splits into parts. The part left without an internet exit still
reaches everything inside it (often including the hkfree.org server), but not
the internet. OSPF on that side stops installing the default route. Members
there see "hkfree.org works, the internet doesn't", which would wrongly point
at the internet exit.

## How the app recognises it

- **Each router's own part.** The engine finds the part of the network a
  member's router belongs to. If that part has no internet exit, the network
  split. It no longer matters which router the map was read from: if FRR
  (10.107.3.1) itself ends up in the part without an exit, members on the
  healthy side are not marked as cut off.
- **Where it broke.** Both ends of a failed link drop it from OSPF, so the
  link just disappears. The server remembers every link it has seen for a
  week (in its state file, beside the prefix history), and the map lists the
  links that disappeared (`lost_links`). The links lost at the edge of the
  member's part are where the network broke.
- **What the member sees.** If the server answers but the internet doesn't
  and the map shows the split: *Your part of the network is cut off from the
  internet. The network split at the link between A and B …*, with the advice
  not to restart anything. The outage is reported like a backbone outage.
  Over mobile data, Analýza cesty for the home address shows the same.
- **Alone or with no exit at all.** A router with no working link at all is
  *cut off*. If no router announces a default route anywhere, the finding is
  *no internet exit*.

## Limits

- If the hkfree.org server is on the other side, the app works from the map
  it saved earlier, which doesn't show the split yet. The device's own probes
  then say where traffic stops (*reachable up to A, B doesn't answer*).
- An exit that always announces a default route (`default-information
  originate always`) keeps announcing it when its own upstream is down. OSPF
  can't show that; only the device's internet probe can.
- A view from more routers makes this sharper: see
  [MikroTik routers](../how-to/add-mikrotik-routers.md). Each one reports
  whether it has a default route.
