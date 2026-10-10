# How to serve the app over HTTPS for phones

A phone only installs the app and enables offline mode on an HTTPS origin.
Keep hkdiag on a local address and put a TLS-terminating web server in front.

1. Make hkdiag listen locally and trust the proxy:

   ```toml
   [web]
   listen = "127.0.0.1:8087"
   trusted_proxies = ["127.0.0.1"]
   ```

2. Give the app its own virtual host. It uses absolute paths (`/app.js`,
   `/topology.json`), so it can't live under a sub-path:

   ```nginx
   server {
       listen 443 ssl;
       server_name diag.hkfree.org;
       # ssl_certificate / ssl_certificate_key: your existing setup

       # Optional, recommended: only the hkfree.org network.
       # allow 10.107.0.0/16;
       # deny all;

       location / {
           proxy_pass http://127.0.0.1:8087;
           proxy_set_header X-Real-IP $remote_addr;   # must overwrite, never append
       }
   }
   ```

3. Reload nginx and open `https://diag.hkfree.org/api/whoami` from a member
   device: it must show the device's hkfree.org address, not `127.0.0.1`.

Notes:

- hkdiag believes only `X-Real-IP`, only from `trusted_proxies`, and ignores
  `X-Forwarded-For`. A trusted proxy that sends no `X-Real-IP` makes the
  address unknown, rather than every member looking like the proxy.
- Restricting the vhost to `10.107.0.0/16` keeps the network map private,
  but members on mobile data then can't refresh the app. Their installed copy
  still works offline.
