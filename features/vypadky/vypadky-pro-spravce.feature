@id:vypadky-pro-spravce @backlog
Feature: Outage reports for admins
  A network admin wants to see from the reported outages which routers fail
  most often and which members are affected, so they know where to start
  repairs. Only an admin who proves who they are with their hkfree.org
  account sees reports with members' addresses, and they see all of them. On
  their device they stay logged in for 30 days.

  Scenario: The most often reported router
    Given in the last week 3 members reported an outage of the router "10.107.241.36"
    And 1 member an outage of the router "10.107.142.1"
    When the admin lists the reports of the last week
    Then they see the router "10.107.241.36" as the most often reported, with 3 reports

  Scenario: The admin sees whom the outage affected
    Given the member "10.107.10.146" reported an outage of the router "10.107.241.36"
    When the admin lists the reports
    Then they see the member's address "10.107.10.146" with the outage

  Scenario: A member without an admin role doesn't see reports with addresses
    Given the member "10.107.77.5" proved who they are with their hkfree.org account but isn't an admin
    When they want to list the reports
    Then they learn that only admins see reports with members' addresses

  Scenario: An admin stays logged in for 30 days
    Given the admin logged in on their phone 29 days ago
    When they list the reports
    Then they see them without logging in again

  Scenario: An admin logs in through hkfree.org single sign-on
    Given the member "10.107.10.146" has an hkfree.org account with the role "SO"
    When they log in to the app
    Then they log in through hkfree.org single sign-on, without a separate password for the app

  Scenario: The admin sees what the member's check found
    Given the member "10.107.10.146" reported an outage of the router "10.107.241.36"
    When the admin opens the report
    Then they see which tests failed, how long each router took to answer, and how old the network map was

  Scenario: The admin sees when the app's data sources fail
    Given the router "10.107.10.129" whose OSPF database the map reads hasn't answered since 10:15
    When the admin looks at the state of the app
    Then they see that the map has been missing this router's view since 10:15
