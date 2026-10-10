@id:rozpoznani-site
Feature: Knowing whether I'm on the hkfree.org network
  A member wants to know whether their device is currently connected through
  hkfree.org or some other way, for example mobile data, so they read the
  check result correctly. Being connected through another provider must be
  obvious at first sight.

  @backlog
  Scenario: A member at home on hkfree.org
    Given the member's device is connected through hkfree.org with the address "10.107.77.5"
    When the member runs a check
    Then they see that they are on the hkfree.org network

  @backlog
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

  @backlog
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
