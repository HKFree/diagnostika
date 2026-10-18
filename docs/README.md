# hkdiag documentation

*Česky: [cs/README.md](cs/README.md)*

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
- [Serve the app over HTTPS behind Apache](how-to/serve-over-https.md)
- [Add MikroTik routers as extra views of the OSPF database](how-to/add-mikrotik-routers.md)
- [Mirror the assistant's language models](how-to/mirror-assistant-models.md)
- [Use the public API](how-to/use-the-public-api.md)
- [Deploy as static files](how-to/deploy-static.md)
- [Configure the support contact and UserDB](how-to/configure-support-and-userdb.md)
- [Read outage reports](how-to/read-outage-reports.md)
- [Add or change a guide article](how-to/edit-the-guide.md)
- [Update the vendored WebLLM library](how-to/update-web-llm.md)
- [Run the tests and refresh fixtures](how-to/run-tests.md)
- [Troubleshoot hkdiag itself](how-to/troubleshoot-hkdiag.md)

## Explanation

- [How the app learns the member's address, and when it can't](explanation/member-address.md)
- [Hidden parts of the network](explanation/hidden-parts.md)
- [When the network splits](explanation/split-network.md)

## Reference and explanation (more)

Being rewritten from the specification (`features/`), in Czech and English.
Until then, the README describes configuration, the HTTP API and the security
model.

Screenshots are in [screenshots/](screenshots/).
