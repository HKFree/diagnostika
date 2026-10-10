# Tutorial: check your connection with the app

In this tutorial you install the hkfree.org diagnostics app on your phone, run
your first check, and see where the app keeps what it learns, so it can help
you later when your connection is down. It takes about five minutes.

You need a phone (or laptop) connected to your home hkfree.org network and
the app's address from your admins.

## 1. Open the app at home

Connect the phone to your home Wi-Fi and open the app's address in the
browser. The app starts a check by itself.

At the top right you'll see a badge. It should say **hkfree.org**: the app
recognised that you're on the hkfree.org network. If it says **jiná síť**
(other network), you're on mobile data; switch to your home Wi-Fi.

## 2. Read your first result

After a few seconds the big box shows the result, for example **Připojení
funguje** (Your connection works). Under it, **Co dělat** (What to do) lists
the next steps, if any.

Scroll down. You'll see four checks (device, hkfree.org server, internet by
IP address, internet by name), then **Vaše cesta** (Your path): the routers
your connection uses, from your AP router to the internet exit. Most show
**dostupný** (reachable); some show **nelze otestovat** (not testable), which
is normal.

![Check](../screenshots/11-cs-healthy-hkfree-phone.png)

## 3. Run it once more

Tap **Zkontrolovat znovu** (Run check again). The app now remembers which
routers on your path normally answer. That memory is what later lets it say
*where* an outage is.

## 4. Look at your path

Open the **Cesta** (Path) tab. It lists your path to the internet with each
router's state from the monitoring server, and below that the **Zjištění**
(Findings), for example routers without a backup route.

## 5. Install the app

> This needs the app on an HTTPS address. While it runs on
> `http://10.107.252.72:9000`, the install option doesn't appear and the app
> doesn't work offline; skip this step until hkfree.org has the HTTPS domain.

Tap **Nainstalovat aplikaci** (Install app), or use the browser's "Add to
home screen". The app now opens like any other app, also when you have no
connection.

## 6. Ask the assistant

Open **Pomoc** (Help) and tap a suggested question, for example *Jak správně
restartovat router a anténu?* (How do I restart the router and antenna?). The
answer comes from the guide built into the app and works offline.

## What you've achieved

The app now knows your address, your path and how your routers normally
answer. Next time your connection fails:

1. Open the app (from the home screen; it works without a connection).
2. Read the result and follow **Co dělat**.
3. If the problem was in the network, the app saves it and reports it to
   the admins when the connection is back. The **Výpadky** (Outages) tab shows
   reported outages.

To understand what the app can and can't tell from a phone, read
"Probes from the device" in the [README](../../README.md#probes-from-the-device).
