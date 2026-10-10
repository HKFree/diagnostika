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

    @backlog
    Scenario: The AP admin is known
      Given the AP of the member "10.107.77.5" is looked after by "Jan Novák"
      When the check finds an outage in the backbone
      Then the member sees the contact of the admin "Jan Novák"

    @backlog
    Scenario: The AP admin isn't known
      Given the admin of the AP of the member "10.107.77.5" isn't known
      When the check finds an outage in the backbone
      Then the member sees the support contact "podpora@hkfree.org" and "608 10 10 70"
      And they see that the phone line is open on working days from 9 to 17
      And they see that support can also be reached on the hkfree.org Slack in the channel "#nejdeto"
