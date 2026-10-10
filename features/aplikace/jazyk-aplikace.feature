@id:jazyk-aplikace @backlog
Feature: The app's language
  The app speaks Czech. Members who don't speak Czech get it in English,
  according to their device's settings, and can also switch the language.

  Scenario: A phone set to English
    Given the member's phone is set to English
    When they open the app
    Then the app is in English

  Scenario: A phone set to another language
    Given the member's phone is set to German
    When they open the app
    Then the app is in Czech

  Scenario: The member switches the language
    Given the member's phone is set to Czech
    And they switch the app to English
    When they open the app again
    Then the app is in English
