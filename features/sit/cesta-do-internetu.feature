@id:cesta-do-internetu @backlog
Feature: An address's path to the internet
  A member or an admin wants to see which routers an address's connection to
  the internet goes through, and where on that path something doesn't work,
  so they know where to look for the cause. The hkfree.org network map is
  shared among members, so anyone sees the path of any address, also without
  a connection.

  Rule: Anyone sees the path of any address

    Scenario: A member sees their path
      Given the member's address is "10.107.77.5"
      When the member views the path of their own address
      Then they see the path through the routers "10.107.77.1", "10.107.76.1" and "10.107.209.1" to the exit "10.107.99.1"

    Scenario: A member sees another address's path
      Given the member's address is "10.107.12.65"
      When the member views the path of the address "10.107.77.5"
      Then they see the path of the address "10.107.77.5"

  Rule: The path shows where the problem is

    Scenario: The path contains a router that network monitoring cannot see
      Given the path of the member "10.107.12.65" goes through the router "10.107.142.1"
      And network monitoring cannot see the router "10.107.142.1"
      When the member views the path of their own address
      Then they see the router "10.107.142.1" marked as suspect

    Scenario: No router knows the address
      Given no router in the network knows the address "10.107.77.240"
      And the router "10.107.77.1" used to know it
      When an admin views the path of the address "10.107.77.240"
      Then they see that the address isn't routed in the network and used to belong to the router "10.107.77.1"

    Scenario: The AP router is cut off from the network
      Given the AP router "10.107.9.1" has no working connection to the network
      When a member of this AP views their path
      Then they see that their AP router "10.107.9.1" is cut off from the network

    Scenario: The path of any address drawn as a map
      Given the member's address is "10.107.12.65"
      When the member views the path of the address "10.107.77.5"
      Then they see that path drawn as a map from the AP router "10.107.77.1" to the exit "10.107.99.1"

    Scenario: A place without a backup route
      Given the only connection of the AP router "10.107.77.1" goes through the router "10.107.76.1"
      When the member views the path of their address "10.107.77.5"
      Then they see the router "10.107.76.1" as a place without a backup route
