@id:historie-mereni
Feature: The history of measurements
  A member or an admin wants to see how a router on a path behaved over the
  last day or week: whether its response time grew or it lost packets, and
  since when. A member also wants to see how much their own connection
  downloaded and uploaded, to tell a problem in the network from a line busy
  at home. Network monitoring measures both. A router's history is shared like
  the network map; a connection's traffic belongs to its member and reaches
  only them.

  Rule: Anyone sees the measurement history of the routers on a path

    Scenario: A router's response time and packet loss over the last day
      Given network monitoring measured the router "10.107.76.1" answering in 2 ms over the last day, with 30 % packet loss in the last hour
      When the member views the path of their address "10.107.77.5" and opens the history of the router "10.107.76.1"
      Then they see a chart of the response time and packet loss of "10.107.76.1" over the last day
      And the chart shows packet loss up to 30 %

    Scenario: A router's history over the last week
      Given network monitoring measured the router "10.107.76.1" answering in 2 ms over the last week
      When the member views the path of their address "10.107.77.5" and opens the history of the router "10.107.76.1" over the last week
      Then they see a chart of the response time and packet loss of "10.107.76.1" over the last week

    Scenario: A program reads a router's history
      Given network monitoring measured the router "10.107.76.1" answering in 2 ms over the last day, with 30 % packet loss in the last hour
      When a program asks the API for the measurement history of the router "10.107.76.1"
      Then it gets its response time and packet loss over the last day

    Scenario: No measurement history of a member's device
      When a program asks the API for the measurement history of the address "10.107.77.5"
      Then it gets no measurements of that address

  Rule: The member sees their own connection's traffic, and nobody else's

    Scenario: The member sees their connection's download and upload
      Given network monitoring measured the connection of the member "10.107.10.146" downloading up to 40 Mbit/s and uploading up to 5 Mbit/s over the last day
      When the member asks for their connection's traffic
      Then they see a chart of their download and upload over the last day, up to 40 Mbit/s down and 5 Mbit/s up

    Scenario: Only the member's own traffic
      Given network monitoring measured the connection of the member "10.107.10.146" downloading up to 40 Mbit/s and uploading up to 5 Mbit/s over the last day
      When the device with the address "10.107.10.140" asks the hkfree.org server for its connection's traffic
      Then it doesn't get the traffic of "10.107.10.146"

    Scenario: Traffic is shown only on the hkfree.org network
      Given the member's device is connected through mobile data
      When the member runs a check and opens advanced diagnostics
      Then they can't ask for their connection's traffic
