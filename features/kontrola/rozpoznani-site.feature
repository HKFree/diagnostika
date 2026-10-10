@id:rozpoznani-site @backlog
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

  Scenario: The member sees the other provider's name
    Given the member's device is connected through the mobile data of the provider "T-Mobile"
    When the member runs a check
    Then the warning tells them they are connected through "T-Mobile"

  Scenario: The network can't be determined
    Given the member's device has no contact with the hkfree.org server
    When the member runs a check
    Then they see that it can't be determined which network they are on
