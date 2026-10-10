# How to troubleshoot hkdiag itself

| Symptom | Cause and fix |
|---|---|
| `error: … has mode 644; it may contain secrets` | `chmod 600` the config file. |
| `error: … is not owned by the current user` | Run hkdiag as the file's owner. |
| `error: invalid config … (line N): unknown field` | A misspelled key; all sections reject unknown keys. |
| `FRR vty rejected the password` | Wrong `[frr] password` (or `HKDIAG_FRR_PASSWORD`). |
| `cannot connect to FRR vty …` | The machine can't reach `10.107.3.1:2604`. Test it: `bash -c 'echo > /dev/tcp/10.107.3.1/2604'`. |
| `/topology.json` returns 502 | FRR unreachable and nothing cached yet. The server logs the reason; after a failure it waits 15 s before trying again, serving the last good snapshot if it has one. |
| `cannot listen on …: Cannot assign requested address` | The `listen` address isn't on this machine (for example a VPN interface that went down). |
| Sojka data missing (`warning: Sojka: …`) | Sojka unreachable or answering oddly; diagnosis continues without it. |
| `outage reports disabled: …` at start | The reports database can't be opened; check `reports_db` and the directory permissions. |
| The app asks for the address | The server sees the client as loopback or can't tell (proxy without `X-Real-IP`). |
| The app shows **jiná síť** (other network) on hkfree.org | The address the server sees isn't in the snapshot; check the proxy's `X-Real-IP`. |
| The app doesn't work offline | Not served over HTTPS (or localhost), so no service worker. |
| An installed app shows the old version | A new version takes over only once all its tabs or windows are closed. |
