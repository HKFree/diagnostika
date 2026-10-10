@id:udaje-o-ap @backlog
Feature: Information about the AP
  A member wants to know which AP they connect through, where it is and who
  looks after it, how good their antenna's radio signal is, and whether
  network monitoring sees a problem at their AP, so they can tell a local
  problem from one at the AP. The information comes from the hkfree.org member
  records; information about a member's own device reaches only that member
  and admins.

  Scenario: The member sees their AP's name, area and address
    Given the address "10.107.77.5" belongs to the AP "AP2" in the area "Kukleny" at "Pospíšilova 12, Hradec Králové"
    When the member runs a check
    Then they see that they connect through the AP "AP2" in the area "Kukleny" at "Pospíšilova 12, Hradec Králové"

  Scenario: The member sees their antenna's radio signal on request
    Given the antenna of the member "10.107.77.5" receives the AP with a signal of -67 dBm
    When the member asks for the radio signal of their antenna
    Then they see the signal -67 dBm

  Scenario: The member sees that monitoring reports a problem at their AP
    Given network monitoring reports packet loss at the AP router "10.107.77.1"
    When the member runs a check
    Then they see that network monitoring reports packet loss at their AP router "10.107.77.1"
