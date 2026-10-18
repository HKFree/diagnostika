@id:mereni-rychlosti
Feature: Measuring the connection's speed
  A member whose connection works but feels slow wants to know how fast it
  really is between them and hkfree.org, so they can tell a slow line from a
  slow website or a busy Wi-Fi. A speed test moves a lot of data, so it runs
  only when the member asks for it, and only over hkfree.org.

  Rule: The member measures their speed on request

    Scenario: The member measures their connection's speed
      Given the member's device is connected through hkfree.org with the address "10.107.77.5"
      And the speed test server answers
      When the member measures the speed of their connection
      Then they see the download and upload speed in Mbit/s and the response time in ms

    Scenario: The speed test server doesn't answer
      Given the member's device is connected through hkfree.org with the address "10.107.77.5"
      But the speed test server doesn't answer
      When the member measures the speed of their connection
      Then they see that the speed couldn't be measured

    Scenario: Not on hkfree.org
      Given the member's device is connected through mobile data
      When the member runs a check and opens advanced diagnostics
      Then they can't start a speed test
      And they see that it measures only connections through hkfree.org

  Rule: Measured speeds are shared per AP, without members' addresses
    After a speed test over hkfree.org the app sends the result to the
    server like an outage report, without the member's address. Everyone sees
    the speeds per AP, the typical (median) and the lowest, once at least 3
    were measured there in the last week, so no single member's line stands
    out. A part of the network the member records don't name is grouped under
    its AP router.

    Scenario: Everyone sees the speeds members measured at each AP
      Given members of the AP "Podůlšany" measured download speeds of 80, 95 and 20 Mbit/s this week
      When anyone looks at the speeds measured in the network
      Then they see the AP "Podůlšany" with a typical download speed of 80 Mbit/s and the lowest of 20 Mbit/s
      But they don't see which members measured them

    Scenario: Speeds of a part of the network without names grouped by its AP router
      Given members behind the router "10.107.77.1", which the member records don't name, measured download speeds of 50, 60 and 70 Mbit/s this week
      When anyone looks at the speeds measured in the network
      Then they see a typical download speed of 60 Mbit/s grouped under the router "10.107.77.1"

    Scenario: Too few measurements aren't shown
      Given one member of the AP "Podůlšany" measured a download speed of 80 Mbit/s this week
      When anyone looks at the speeds measured in the network
      Then they don't see the AP "Podůlšany" among the speeds

    Scenario: The member compares their speed with their AP
      Given members of the AP "Podůlšany" measured download speeds of 80, 95 and 20 Mbit/s this week
      And the member's device is connected through hkfree.org with the address "10.107.10.146"
      And the speed test server answers
      When the member measures the speed of their connection
      Then they see that members of their AP typically measure 80 Mbit/s download

  Rule: An outage report carries the member's last speed
    A speed the member measured in the 24 hours before an outage goes with
    its report, so the admins see whether the line was slow before it failed.

    Scenario: The admins see the measured speeds with an outage report
      Given the member "10.107.10.146" measured a download speed of 3 Mbit/s and then reported an outage
      When the admin lists the reports
      Then they see the download speed of 3 Mbit/s with the member's outage
