# Hidden parts of the network

hkdiag reads the OSPF database of the backbone (area 0). Some parts of
hkfree.org run as their own OSPF area. Inside such an area, routers and links
aren't announced into the backbone. Its **border router** (the ABR) announces
only *summary-LSAs*: which address ranges lie behind it. Those ranges may also
be aggregated into one larger prefix. For example, 10.107.10.97 announces
10.107.10.56/30, 10.107.10.64/28 and 10.107.10.128/28.

The app therefore knows *that* an address is behind the border router, but not
which router inside the area serves it.

- **IPv4:** if the most specific prefix covering a member's address is a
  summary, the path starts at the border router. Advanced diagnostics label
  it *border router of a hidden part of the network* and say the routers
  behind it aren't in the map.
- **IPv6:** OSPFv3 may run the same routers in area 0, so their IPv6
  prefixes are visible while the routers aren't in the IPv4 map. OSPFv3
  router IDs are the IPv4 router IDs, so such a router (e.g. 10.107.10.65)
  is placed by the summary covering its router ID, behind the same border
  router.
- If the problem is inside the hidden part, the app can only show that the
  border router answers and the member's traffic doesn't get through. It
  can't say where the traffic stops.

In the snapshot (`topology.json`), summaries are `prefixes` of kind `"a"`.
