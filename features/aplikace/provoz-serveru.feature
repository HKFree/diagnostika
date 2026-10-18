@id:provoz-serveru
Feature: Running the server
  The admin who runs the diagnostics wants to know whether members use it and
  whether outage reports reach the server, without waiting for a real outage.
  What the server records about requests doesn't identify members.

  Rule: The admin checks that outage reports would be stored, without storing one
    A trial report goes through every check a member's report goes through,
    up to the database, and is then thrown away.

    Scenario: A trial report the server would store
      When the admin sends the server a trial outage report
      Then the server says it would store the report
      And the report isn't stored

    Scenario: A trial report the server wouldn't store
      When the admin sends the server a trial outage report with a verdict members' apps don't report
      Then the server says it wouldn't store the report, because of the verdict

  Rule: The server records each request without members' addresses

    Scenario: The server log shows each request
      Given the member's device is connected through hkfree.org with the address "10.107.77.5"
      And their AP router "10.107.77.1" and the internet answer
      When the member runs a check
      Then the server log shows the app's requests from the hkfree.org network with their results
      But not the address "10.107.77.5"

    Scenario: The server log shows what happened to reported outages
      Given the member "10.107.10.146" reported an outage of the router "10.107.241.36" that lasted 20 minutes
      Then the server log shows that 1 report was received from the hkfree.org network and 1 stored
      But not the address "10.107.10.146"
