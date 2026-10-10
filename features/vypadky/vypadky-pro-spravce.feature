@id:vypadky-pro-spravce @backlog
Feature: Outage reports for admins
  A network admin wants to see from the reported outages which routers fail
  most often and which members are affected, so they know where to start
  repairs.

  Scenario: The most often reported router
    Given in the last week 3 members reported an outage of the router "10.107.241.36"
    And 1 member an outage of the router "10.107.142.1"
    When the admin lists the reports of the last week
    Then they see the router "10.107.241.36" as the most often reported, with 3 reports

  Scenario: The admin sees whom the outage affected
    Given the member "10.107.10.146" reported an outage of the router "10.107.241.36"
    When the admin lists the reports
    Then they see the member's address "10.107.10.146" with the outage
