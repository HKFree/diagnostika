# How to deploy the app as static files

You can serve the app from any web server without running `hkdiag serve`.
Outage reports need the running server, so they are not stored in this mode;
the app keeps them on the device and drops them when the server rejects them.

1. Export the app and a first snapshot:

   ```
   hkdiag export --web /var/www/diag
   ```

   This writes `index.html`, the scripts, icons, `sw.js` (with its build hash),
   `vendor/web-llm/index.js` and `topology.json`.

2. Refresh the snapshot regularly, for example every minute from cron:

   ```
   * * * * * /usr/local/bin/hkdiag export --out /var/www/diag/topology.json
   ```

   The file is replaced atomically, so clients never read a half-written file.

3. Add the one dynamic endpoint the app uses, so it can tell your members'
   addresses (without it, members enter their address once by hand):

   ```nginx
   location = /api/whoami {
       default_type application/json;
       return 200 '{"ip":"$remote_addr"}';
   }
   ```

4. Serve it over HTTPS on its own virtual host (see
   [Serve the app over HTTPS](serve-over-https.md)).

Without `/api/whoami` the app still works: a successfully loaded
`topology.json` counts as "server reachable".
