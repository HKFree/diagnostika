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

  Rule: The member sees their antenna's radio signal on request
    Reading the signal means logging in to the AP's routers, so it happens
    only when the member asks for it, at most once a minute per AP, and a
    member gets only the signal of their own antenna.

    Scenario: The member sees their antenna's signal
      Given the antenna of the member "10.107.10.146" receives the AP with a signal of -67 dBm
      When the member asks for the radio signal of their antenna
      Then they see the signal -67 dBm, rated as good

    Scenario: A weak signal
      Given the antenna of the member "10.107.10.146" receives the AP with a signal of -84 dBm
      When the member asks for the radio signal of their antenna
      Then they see the signal -84 dBm, rated as weak
      And they are advised to check that the antenna points at the AP and nothing blocks the view

    Scenario: The AP's routers don't see the member's antenna
      Given the AP's routers don't see the antenna of the member "10.107.10.140"
      When the member asks for the radio signal of their antenna
      Then they see that their antenna wasn't found at the AP

    Scenario: Only the member's own antenna
      Given the antenna of the member "10.107.10.146" receives the AP with a signal of -67 dBm
      When the device with the address "10.107.10.140" asks the hkfree.org server for its antenna's signal
      Then it doesn't get the signal of the antenna of "10.107.10.146"

  Rule: The member sees what network monitoring reports at their AP
    Network monitoring watches the AP's own devices (routers, links, sectors),
    never members' devices. Devices it hasn't heard from for a week are taken
    as decommissioned and left out.

    Scenario: Monitoring reports packet loss at the AP router
      Given network monitoring reports packet loss at the AP router "10.107.10.149" of the member "10.107.10.146"
      When the member runs a check and opens advanced diagnostics
      Then they see that network monitoring reports packet loss at "10.107.10.149"

    Scenario: A device of the AP doesn't answer monitoring
      Given network monitoring reports that the device "sektor jih" with the address "10.107.10.150" of the AP of the member "10.107.10.146" doesn't answer
      When the member runs a check and opens advanced diagnostics
      Then they see that network monitoring reports that "sektor jih" "10.107.10.150" doesn't answer

    Scenario: Monitoring sees no problem at the AP
      Given network monitoring sees all devices of the AP of the member "10.107.10.146" working
      When the member runs a check and opens advanced diagnostics
      Then they see that network monitoring sees no problem at their AP
