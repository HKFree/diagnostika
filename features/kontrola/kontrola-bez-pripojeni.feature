@id:kontrola-bez-pripojeni @backlog
Feature: Checking even when nothing works
  A member with no connection at all can't look anything up. So the check
  must work without a connection too, from what the device remembered the
  last time the connection worked.

  Scenario: The device isn't connected to any network
    Given the member ran a check while their connection worked
    And now their device isn't connected to any network
    When the member runs a check
    Then they see that the problem is in their device
    And they are advised to check the Wi-Fi or cable and to restart the router

  Scenario: An AP outage without contact to the server
    Given the member ran a check while their connection worked
    And now neither the hkfree.org server, the internet nor the AP router "10.107.77.1" answer
    When the member runs a check
    Then they see that their AP router "10.107.77.1" doesn't answer

  Scenario: No earlier check
    Given the member never ran a check while their connection worked
    And now nothing answers
    When the member runs a check
    Then they see that they can't reach hkfree.org
    And they learn to open the app next time while the connection works

  Scenario: The member sees how old the remembered network state is
    Given the device last remembered the network state 3 hours ago
    When the member runs a check without a connection
    Then they see that the result is based on the network state from 3 hours ago
