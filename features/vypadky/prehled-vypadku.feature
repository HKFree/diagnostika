@id:prehled-vypadku
Feature: An outage overview for everyone
  Anyone who opens the app wants to see which outages members reported, to
  tell whether their problem affects others too. Members' addresses are not in
  the overview.

  Scenario: A reported outage without the member's address
    Given the member "10.107.10.146" reported an outage of the router "10.107.241.36" that lasted 1 minute
    When anyone views the outage overview
    Then they see an outage of the router "10.107.241.36" that lasted 1 minute
    But they don't see the address "10.107.10.146"

  Scenario: The overview shows the last week
    Given one outage was reported 2 days ago and another 8 days ago
    When anyone views the outage overview
    Then they see only the outage from 2 days ago

  Scenario: The most often reported routers
    Given in the last week 3 members reported an outage of the router "10.107.76.1"
    When anyone views the outage overview
    Then they see the router "10.107.76.1" among the most often reported, with 3 reports

  Scenario: The overview shows where network monitoring reports a problem now
    Given the member records' status reports packet loss at the AP "Podůlšany"
    When anyone views the outage overview
    Then they see the AP "Podůlšany" among the places with a problem right now

  Scenario: The overview lists the outages network monitoring measured
    Given network monitoring found that the device "sektor jih" of the AP "Podůlšany" stopped answering and later answered again
    When anyone views the outage overview
    Then they see that outage of "sektor jih" at the AP "Podůlšany" with when it started and ended, measured by network monitoring
