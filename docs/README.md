# hkdiag documentation

The documentation follows [Diátaxis](https://diataxis.fr/). Each page answers
one kind of need:

| | For study | For work |
|---|---|---|
| **Practical steps** | [Tutorials](#tutorials): learn by doing | [How-to guides](#how-to-guides): get a task done |
| **Theoretical knowledge** | [Explanation](#explanation): understand why | [Reference](#reference): look things up |

## Tutorials

- [Check your connection with the app](tutorials/01-check-your-connection.md):
  a member's first run, on a phone.
- [Run hkdiag on your own machine](tutorials/02-run-hkdiag-locally.md): an
  admin's first run, from build to the first diagnosis.

## How-to guides

- [Deploy or update on lide.hkfree.org](how-to/deploy-on-lide.md)
- [Serve the app over HTTPS for phones](how-to/serve-over-https.md)
- [Deploy as static files](how-to/deploy-static.md)
- [Configure the support contact and UserDB](how-to/configure-support-and-userdb.md)
- [Read outage reports](how-to/read-outage-reports.md)
- [Add or change a guide article](how-to/edit-the-guide.md)
- [Update the vendored WebLLM library](how-to/update-web-llm.md)
- [Run the tests and refresh fixtures](how-to/run-tests.md)
- [Troubleshoot hkdiag itself](how-to/troubleshoot-hkdiag.md)

## Reference

- [Command line](reference/cli.md)
- [Configuration file](reference/configuration.md)
- [HTTP API](reference/http-api.md)
- [Topology snapshot format](reference/snapshot-format.md)
- [Outage reports](reference/outage-reports.md)
- [Verdicts and findings](reference/verdicts-and-findings.md)
- [Device probes](reference/device-probes.md)
- [PWA: storage, caches, language](reference/pwa.md)
- [Assistant](reference/assistant.md)

## Explanation

- [Architecture: why the diagnosis runs on the device](explanation/architecture.md)
- [How the OSPF model works](explanation/ospf-model.md)
- [Probing from a browser](explanation/browser-probing.md)
- [Security and privacy](explanation/security-and-privacy.md)
- [Design decisions and constraints](explanation/design-decisions.md)

Screenshots are in [screenshots/](screenshots/).
