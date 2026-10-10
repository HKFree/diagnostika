@id:hlaseni-vypadku
Feature: Reporting outages
  When a member has an outage in the hkfree.org network, the admins often
  never hear about it. The app remembers the outage and reports it by itself
  once the connection is back, so the admins know where and when the network
  fails. The report goes without asking; only admins see the member's address
  in it.

  Rule: Only outages in the hkfree.org network are reported

    Scenario: A backbone outage is reported once the connection is back
      Given the member "10.107.10.146" found that the router "10.107.241.36" on their path doesn't answer
      When their connection is restored
      Then the member sees that the outage was reported
      And the admins see the outage of the router "10.107.241.36" from the member "10.107.10.146"

    Scenario: A problem in the member's device isn't reported
      Given the check found that the device of the member "10.107.77.5" isn't connected to any network
      When their connection is restored
      Then the admins see no outage from the member "10.107.77.5"

  Rule: An outage is reported once the member is back on hkfree.org

    Scenario: The member meanwhile connected through mobile data
      Given the member "10.107.10.146" found an outage of the router "10.107.241.36"
      And then connected through mobile data
      When they run a check
      Then the outage isn't reported yet

    Scenario: The check repeats by itself during an outage
      Given the member found an outage of the router "10.107.241.36"
      And the app stayed open
      When the connection is restored 2 minutes later
      Then within a minute the member sees that the outage was reported

    Scenario: An outage older than a month isn't reported
      Given the member found an outage 31 days ago and hasn't opened the app since
      When their connection is restored
      Then the admins don't see this outage
