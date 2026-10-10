@id:kontrola-pripojeni
Feature: Checking one's own connection
  An hkfree.org member whose internet doesn't work wants to find out, without
  anyone's help, where the problem is and what to do. That tells them whether
  they can fix it themselves, or whether the problem is in the network and they
  should wait or contact support. When the problem is in the network, the app
  shows them the admin of their AP, or hkfree.org support when it doesn't know
  them.

  Rule: The member sees where their connection breaks

    Scenario: The connection works
      Given the member's device is connected through hkfree.org with the address "10.107.77.5"
      And their AP router "10.107.77.1" and the internet answer
      When the member runs a check
      Then they see that their connection works

    Scenario: The AP router doesn't answer
      Given the AP router "10.107.77.1" of the member "10.107.77.5" usually answers
      And now it doesn't
      When the member runs a check
      Then they see that the problem is between them and the AP router "10.107.77.1"
      And they are advised to restart the router and check the antenna's power

    Scenario: An outage in the backbone
      Given the path of the member "10.107.10.146" goes through the AP router "10.107.10.149" and the router "10.107.241.36"
      And the AP router "10.107.10.149" answers, but the router "10.107.241.36" doesn't
      When the member runs a check
      Then they see that the problem is in the hkfree.org backbone beyond the router "10.107.10.149"
      And they are advised not to restart anything

  Rule: The member first sees only what matters

    The check result, what to do, whom to contact, the warning about another
    provider and the note about a reported outage stay visible. The individual
    tests, the path and entering an address wait under advanced diagnostics,
    which start folded at every check.

    Scenario: The details are hidden in advanced diagnostics
      Given the AP router "10.107.77.1" of the member "10.107.77.5" doesn't answer
      When the member runs a check
      Then they see that their connection doesn't work, that the problem is at the AP router "10.107.77.1" and what to do
      And they see the results of the individual tests and the path only once they open advanced diagnostics

    Scenario: The member opens advanced diagnostics
      Given the check of the member "10.107.77.5" found that the AP router "10.107.77.1" doesn't answer
      When the member opens advanced diagnostics
      Then they see the results of the individual tests
      And they see their path through the AP router "10.107.77.1"

    Scenario: Advanced diagnostics start folded at every check
      Given the member "10.107.77.5" opened advanced diagnostics at their last check
      When the member runs a check again
      Then the individual tests and the path are folded away again

  Rule: The member sees their path as a map

    Scenario: A working path drawn as a map
      Given the member's device is connected through hkfree.org with the address "10.107.77.5"
      And their whole path answers
      When the member runs a check and opens advanced diagnostics
      Then they see a map from their device through the routers "10.107.77.1" and "10.107.76.1" to the exit "10.107.99.1"
      And they see traffic flowing along the whole map to the internet

    Scenario: The map shows where the path breaks
      Given the path of the member "10.107.10.146" goes through the AP router "10.107.10.149" and the router "10.107.241.36"
      And the AP router "10.107.10.149" answers, but the router "10.107.241.36" doesn't
      When the member runs a check and opens advanced diagnostics
      Then they see the traffic on the map stop at the router "10.107.241.36", marked as not answering

    @backlog
    Scenario: A member who turned off animations sees a still map
      Given the member's device asks apps to reduce motion
      When the member runs a check and opens advanced diagnostics
      Then they see the map of their path without moving traffic

  Rule: The member gets advice according to what doesn't work

    @backlog
    Scenario: Site names don't resolve
      Given the internet answers at the address "1.1.1.1"
      But the name "google.com" doesn't resolve
      When the member runs a check
      Then they see that it is a DNS problem
      And they are advised to set different DNS servers

    @backlog
    Scenario: hkfree.org works, the internet doesn't
      Given the hkfree.org server answers
      But the internet doesn't answer, not even at the address "1.1.1.1"
      When the member runs a check
      Then they see that their path in hkfree.org is fine and the problem is beyond the internet exit

    @backlog
    Scenario: Nothing leaves the device
      Given the device reports a Wi-Fi connection
      But every connection from it fails immediately
      When the member runs a check
      Then they see that nothing leaves their device
      And they are advised to renew the connection and restart the router

  Rule: With a problem in the network, the member knows whom to turn to
    The AP admins' nicknames are part of the shared network map; an admin's
    e-mail is given only to a device inside that admin's AP, which remembers it
    for when the network fails.

    Scenario: The AP admin is known
      Given the AP of the member "10.107.10.146" is looked after by the area admin "jan" with the e-mail "jan@example.org"
      And by the deputy "petr" with the e-mail "petr@example.org"
      And the member has checked their connection while it worked
      When the check finds an outage in the backbone
      Then the member sees the contact of the admin "jan" with the e-mail "jan@example.org"
      And they see the contact of the deputy "petr" with the e-mail "petr@example.org"

    Scenario: The device doesn't know the admin's e-mail yet
      Given the AP of the member "10.107.10.146" is looked after by the area admin "jan" with the e-mail "jan@example.org"
      And the hkfree.org server couldn't give the member's device the admin's e-mail
      When the check finds an outage in the backbone
      Then the member sees the admin "jan" without an e-mail
      And they see the support contact "podpora@hkfree.org" and "608 10 10 70"

    Scenario: Only a device inside the AP gets its admin's e-mail
      Given the AP of the member "10.107.10.146" is looked after by the area admin "jan" with the e-mail "jan@example.org"
      When the device with the address "10.107.77.5" asks the hkfree.org server for the contact of its AP admin
      Then it doesn't get the e-mail "jan@example.org"

    Scenario: The AP admin isn't known
      Given the admin of the AP of the member "10.107.77.5" isn't known
      When the check finds an outage in the backbone
      Then the member sees the support contact "podpora@hkfree.org" and "608 10 10 70"
      And they see that the phone line is open on working days from 9 to 17
      And they see that support can also be reached on the hkfree.org Slack in the channel "#nejdeto"
