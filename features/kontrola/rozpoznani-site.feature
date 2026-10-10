@id:rozpoznani-site @backlog
Feature: Knowing whether I'm on the hkfree.org network
  A member wants to know whether their device is currently connected through
  hkfree.org or some other way, for example mobile data, so they read the
  check result correctly.

  Scenario: A member at home on hkfree.org
    Given the member's device is connected through hkfree.org with the address "10.107.77.5"
    When the member runs a check
    Then they see that they are on the hkfree.org network

  Scenario: A member on mobile data
    Given the member's device is connected through mobile data
    When the member runs a check
    Then they see that they are not on the hkfree.org network
    And they see the routers on their path as not verifiable from outside

  Scenario: The network can't be determined
    Given the member's device has no contact with the hkfree.org server
    When the member runs a check
    Then they see that it can't be determined which network they are on
