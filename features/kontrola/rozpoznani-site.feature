@id:rozpoznani-site
Feature: Knowing whether I'm on the hkfree.org network
  A member wants to know whether their device is currently connected through
  hkfree.org or some other way, for example mobile data, so they read the
  check result correctly. Being connected through another provider must be
  obvious at first sight.

  Scenario: A member at home on hkfree.org
    Given the member's device is connected through hkfree.org with the address "10.107.77.5"
    When the member runs a check
    Then they see that they are on the hkfree.org network

  Scenario: A member on mobile data
    Given the member's device is connected through mobile data
    When the member runs a check
    Then right at the top they see a prominent warning that they are connected not through hkfree.org but through another provider
    And they see the routers on their path as not verifiable from outside

  @backlog
  Scenario: The member sees the other provider's name
    Given the member's device is connected through the mobile data of the provider "T-Mobile"
    When the member runs a check
    Then the warning tells them they are connected through "T-Mobile"

  Scenario: The network can't be determined
    Given the member's device has no contact with the hkfree.org server
    When the member runs a check
    Then they see that it can't be determined which network they are on

  Rule: The member sees whether they are on hkfree.org also over IPv6

    Scenario: A member whose device uses IPv6
      Given the member's device reaches hkfree.org over IPv6 with the address "2a01:168:331::5"
      When the member runs a check
      Then they see that they are on the hkfree.org network

    Scenario: An IPv6 address of another provider
      Given the member's device reaches the hkfree.org server over IPv6 from the address "2001:db8::5" of another provider
      When the member runs a check
      Then they see that they are not on the hkfree.org network

  Rule: The member sees their path also over IPv6

    Scenario: An IPv6 member sees their path
      Given the router "10.107.2.1" advertises the IPv6 prefix "2a01:168:200:100::/56"
      And the member's device reaches hkfree.org over IPv6 with the address "2a01:168:200:100::5"
      When the member runs a check
      Then they see their path through the router "10.107.2.1"

    Scenario: The router of an IPv6 address isn't in the network map
      Given the router "10.107.200.5" advertises the IPv6 prefix "2a01:168:c00::/64" but isn't in the network map
      And the member's device reaches hkfree.org over IPv6 with the address "2a01:168:c00::5"
      When the member runs a check
      Then they see that they are on the hkfree.org network
      And they see that their path can't be determined

  Rule: The member sees their path also from a hidden part of the network

    Scenario: A member in a hidden part of the network
      Given the addresses "10.107.10.64/28" are in a hidden part of the network behind the border router "10.107.10.97"
      And the member's device is connected through hkfree.org with the address "10.107.10.70"
      When the member runs a check and opens advanced diagnostics
      Then they see their path through the border router "10.107.10.97"
      And they see that the routers behind the border router aren't in the network map

    Scenario: An IPv6 member whose router is in a hidden part of the network
      Given the router "10.107.10.65" advertises the IPv6 prefix "2a01:168:a00:1500::/64" from a hidden part of the network behind the border router "10.107.10.97"
      And the member's device reaches hkfree.org over IPv6 with the address "2a01:168:a00:1500::5"
      When the member runs a check and opens advanced diagnostics
      Then they see their path through the border router "10.107.10.97"

  Rule: A router that sees inside a hidden part shows the member's whole path
    Besides the main OSPF router, the network map reads routers that have a
    read-only account. A border router sees inside its own part, so the
    routers there become part of the map.

    Scenario: The border router lets the network map see inside its part
      Given the border router "10.107.10.97" lets the network map see inside its part, where the router "10.107.10.65" serves the addresses "10.107.10.64/28"
      And the member's device is connected through hkfree.org with the address "10.107.10.70"
      When the member runs a check and opens advanced diagnostics
      Then they see a map from their device through the routers "10.107.10.65" and "10.107.10.97" to the exit "10.107.99.1"

    Scenario: A router that can't be read leaves the map as it was
      Given the router that lets the network map see inside the part behind "10.107.10.97" doesn't answer
      And the member's device is connected through hkfree.org with the address "10.107.10.70"
      When the member runs a check and opens advanced diagnostics
      Then they see their path through the border router "10.107.10.97"

  Rule: The member sees that they are connected through the hkfree.org VPN
    Through the VPN the member is on hkfree.org, but not through their home
    connection: the check tests the VPN, and their home address stays the one
    the app remembers.

    Scenario: A member connected through the VPN
      Given the member's device is connected through the hkfree.org VPN with the address "10.107.254.18"
      When the member runs a check and opens advanced diagnostics
      Then they see that they are connected through the hkfree.org VPN, so the check doesn't test their home connection
      And they see the VPN server instead of an AP router on their path

    Scenario: A member connected through the VPN over IPv6
      Given the member's device is connected through the hkfree.org VPN with the address "2a01:168:0:dead::1003"
      When the member runs a check and opens advanced diagnostics
      Then they see that they are connected through the hkfree.org VPN, so the check doesn't test their home connection

    Scenario: The home address stays remembered
      Given the member checked their connection at home with the address "10.107.77.5"
      And then connected through the hkfree.org VPN with the address "10.107.254.18"
      When the member views the path of their home address
      Then they see the path of the address "10.107.77.5"
