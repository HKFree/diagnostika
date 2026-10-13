@id:prehled-vypadku
Feature: An outage overview for each member
  A member wants to see the outages that concern them: at their own AP, or on
  the paths their traffic can take to the internet, to tell whether their
  problem affects others too. Outages elsewhere in the network aren't shown,
  and members' addresses never are. The app shows what is going on now and
  what ended in the last 24 hours; older outages are in the member records
  and in network monitoring, where the app links to. Outages of up to 15
  minutes are ignored.

  Rule: The member sees the outages that concern them

    Scenario: A reported outage on the member's path, without the reporter's address
      Given the member "10.107.10.146" reported an outage of the router "10.107.241.36" that lasted 20 minutes
      When the member "10.107.10.147" views the outage overview
      Then they see an outage of the router "10.107.241.36" that lasted 20 minutes
      But they don't see the address "10.107.10.146"

    Scenario: An outage elsewhere in the network isn't shown
      Given the member "10.107.10.146" reported an outage of the router "10.107.241.36" that lasted 20 minutes
      When the member "10.107.77.5" views the outage overview
      Then they don't see an outage of the router "10.107.241.36"

    Scenario: Outages of up to 15 minutes aren't shown
      Given the member "10.107.10.146" reported an outage of the router "10.107.10.149" that lasted 10 minutes
      When the member "10.107.10.147" views the outage overview
      Then they don't see an outage of the router "10.107.10.149"

    Scenario: The overview shows only recent outages
      Given outages on the path of the member "10.107.77.5" were reported 2 hours ago and 2 days ago
      When the member "10.107.77.5" views the outage overview
      Then they see only the outage from 2 hours ago

    Scenario: Older outages are in the member records and in network monitoring
      When the member "10.107.10.146" views the outage overview
      Then they see a link to the page of their AP "Podůlšany" in the member records
      And a link to the history in network monitoring

    Scenario: The most often reported routers on the member's paths
      Given in the last day 3 members reported an outage of the router "10.107.76.1"
      When the member "10.107.77.5" views the outage overview
      Then they see the router "10.107.76.1" among the most often reported, with 3 reports

  Rule: The member sees what network monitoring finds at their AP and on their paths

    Scenario: The overview shows where network monitoring reports a problem now
      Given the member records' status reports packet loss at the AP "Podůlšany"
      When the member "10.107.10.146" views the outage overview
      Then they see the AP "Podůlšany" among the places with a problem right now

    Scenario: A problem at another AP isn't shown
      Given the member records' status reports packet loss at the AP "Lipky"
      When the member "10.107.10.146" views the outage overview
      Then they don't see the AP "Lipky"

    Scenario: The overview lists the outages network monitoring measured
      Given network monitoring found that the device "sektor jih" of the AP "Podůlšany" stopped answering and later answered again
      When the member "10.107.10.146" views the outage overview
      Then they see that outage of "sektor jih" at the AP "Podůlšany" with when it started and ended, measured by network monitoring
