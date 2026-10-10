@id:udaje-o-ap
Feature: Information about the AP
  A member wants to know which AP they connect through, where it is and who
  looks after it, how good their antenna's radio signal is, and whether
  network monitoring sees a problem at their AP, so they can tell a local
  problem from one at the AP. The information comes from the hkfree.org member
  records; information about a member's own device reaches only that member
  and admins.

  Rule: The member sees which AP they connect through

    Scenario: The member sees their AP's name, area and address
      Given the address "10.107.10.146" belongs to the AP "Podůlšany" in the area "Podůlšany" at "Podůlšany 1, 533 45 Podůlšany"
      When the member runs a check and opens advanced diagnostics
      Then they see that they connect through the AP "Podůlšany" in the area "Podůlšany" at "Podůlšany 1, 533 45 Podůlšany"

    Scenario: The member records don't name the AP
      Given the AP router "10.107.77.1" of the member "10.107.77.5" isn't named in the member records
      When the member runs a check and opens advanced diagnostics
      Then they see their AP router "10.107.77.1" without an AP name

  @backlog
  Scenario: The member sees their antenna's radio signal on request
    Given the antenna of the member "10.107.10.146" receives the AP with a signal of -67 dBm
    When the member asks for the radio signal of their antenna
    Then they see the signal -67 dBm

  @backlog
  Scenario: The member sees that monitoring reports a problem at their AP
    Given network monitoring reports packet loss at the AP router "10.107.10.149"
    When the member runs a check
    Then they see that network monitoring reports packet loss at their AP router "10.107.10.149"
