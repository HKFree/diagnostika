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

  @backlog
  Scenario: The admins see the measured speeds with an outage report
    Given the member measured a download speed of 3 Mbit/s and then reported an outage
    When the admin opens the report
    Then they see the measured speeds

  @backlog
  Scenario: Everyone sees the speeds members measured at each AP
    Given members of the AP "Podůlšany" measured download speeds of 80, 95 and 20 Mbit/s this week
    When anyone looks at the speeds measured in the network
    Then they see the AP "Podůlšany" with its typical and lowest measured speed
    But they don't see which members measured them

  @backlog
  Scenario: Speeds of a part of the network without names grouped by its first backbone router
    Given members behind the router "10.107.77.1", which the member records don't name, measured their speeds
    When anyone looks at the speeds measured in the network
    Then they see those speeds grouped under the router "10.107.77.1"
