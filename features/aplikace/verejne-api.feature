@id:verejne-api
Feature: A public API for the network's measurements
  Other hkfree.org services and anyone curious want to use what the
  diagnostics measure: the network map, what monitoring finds, the outages it
  measured and the outages members reported. The API is described where
  programs look for it. Nothing in it identifies a member.

  Rule: Programs find the API description at the well-known place

    Scenario: The API catalog points to the API description
      When a program asks for the API catalog at "/.well-known/api-catalog"
      Then it learns where the API description is
      And the description covers the network map, the outages, the measured outages and the reported outages

  Rule: The measurements are available to anyone, without members' addresses
    An outage of a device that doesn't answer lasts until network monitoring
    hears from it again, also after the member records' status stops listing
    it (it does so after a week).

    Scenario: The outages network monitoring measured
      Given network monitoring found that the device "sektor jih" of the AP "Podůlšany" stopped answering 20 minutes ago and now answers again
      When a program asks the API for the measured outages of the last day
      Then it gets that outage of "sektor jih" at the AP "Podůlšany" with its start and end

    Scenario: A device that hasn't answered for over a week is still out
      Given network monitoring found that the device "sektor jih" of the AP "Podůlšany" stopped answering 8 days ago
      And the member records' status stopped listing it after a week, while it still doesn't answer
      When a program asks the API for the measured outages of the last day
      Then it gets that outage of "sektor jih" at the AP "Podůlšany" as still going on

    Scenario: A device that answers again after more than a week
      Given network monitoring found that the device "sektor jih" of the AP "Podůlšany" stopped answering 8 days ago
      And the member records' status stopped listing it after a week, and now it answers again
      When a program asks the API for the measured outages of the last day
      Then it gets that outage of "sektor jih" at the AP "Podůlšany" with its start and end

    Scenario: Reported outages without the member's address
      Given the member "10.107.10.146" reported an outage of the router "10.107.241.36" that lasted 20 minutes
      When a program asks the API for the reported outages of the last day
      Then it gets the outage of the router "10.107.241.36"
      But not the address "10.107.10.146"

  Rule: Other sites may read the shared measurements, not answers about the visitor

    Scenario: A page on another site reads the outages
      When a page on another site asks the API for the outages
      Then its browser lets it read the answer

    Scenario: A page on another site can't learn the visitor's address
      When a page on another site asks who its visitor is
      Then its browser doesn't let it read the answer
