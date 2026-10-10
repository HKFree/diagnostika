# Návod: aplikace přes HTTPS pro telefony

Telefon si aplikaci nainstaluje a zapne provoz bez připojení jen na adrese
HTTPS. hkdiag nechte na místní adrese a předřaďte mu webový server s TLS.

1. hkdiag ať naslouchá lokálně a věří proxy:

   ```toml
   [web]
   listen = "127.0.0.1:8087"
   trusted_proxies = ["127.0.0.1"]
   ```

2. Dejte aplikaci vlastní virtuální host. Používá absolutní cesty
   (`/app.js`, `/topology.json`), takže nemůže běžet v podadresáři:

   ```nginx
   server {
       listen 443 ssl;
       server_name diag.hkfree.org;
       # ssl_certificate / ssl_certificate_key: vaše stávající nastavení

       # Volitelné, doporučené: jen síť hkfree.org.
       # allow 10.107.0.0/16;
       # deny all;

       location / {
           proxy_pass http://127.0.0.1:8087;
           proxy_set_header X-Real-IP $remote_addr;   # musí přepsat, nikdy připojit
       }
   }
   ```

3. Načtěte novou konfiguraci nginx a ze zařízení člena otevřete
   `https://diag.hkfree.org/api/whoami`: musí ukázat adresu zařízení v
   hkfree.org, ne `127.0.0.1`.

Poznámky:

- hkdiag věří jen `X-Real-IP`, jen od `trusted_proxies`, a `X-Forwarded-For`
  ignoruje. Důvěryhodná proxy, která `X-Real-IP` nepošle, způsobí, že adresa
  není známá, místo aby všichni členové vypadali jako proxy.
- Omezení hostu na `10.107.0.0/16` drží mapu sítě v hkfree.org, ale členové
  na mobilních datech si pak aplikaci neobnoví. Jejich nainstalovaná kopie
  ale bez připojení funguje dál.
