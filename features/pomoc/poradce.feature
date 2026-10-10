@id:poradce @backlog
Feature: Advice for helping oneself
  A member wants to ask in their own words how to find and fix the problem,
  and get a clear procedure even without a connection. The answer is based on
  the hkfree.org guide and their last check, and the question doesn't leave
  the member's device.

  Scenario: How to restart the router and antenna
    Given the member's device is not connected to the internet
    When the member asks "How do I restart the router and antenna properly?"
    Then they get a procedure in which the antenna is plugged in first and the router after it
    And they see which guide article the advice comes from

  Scenario: The advice follows on from the last check
    Given the member's last check found an outage in the backbone
    When the member asks "What should I do?"
    Then they see the result of their last check
    And the advice not to restart anything

  Scenario: The guide doesn't know the answer
    When the member asks "Who won the football match yesterday?"
    Then they learn that the guide has nothing on it
    And they are advised to ask differently or contact support

  Scenario: An answer in its own words from a language model
    Given the member downloaded a language model from hkfree.org to their device
    When they ask "Why is the internet slow?"
    Then they get an answer in its own words, based on the guide
    And they see which guide articles it is based on
