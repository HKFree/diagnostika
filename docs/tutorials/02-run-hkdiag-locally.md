# Tutorial: run hkdiag on your own machine

In this tutorial you build hkdiag, look at the live hkfree.org backbone,
diagnose an address from the command line, and open the app in your browser.
It takes about fifteen minutes.

You need a Linux machine on the hkfree.org network (it must reach
`10.107.3.1`), Rust 1.75 or newer, and the read-only FRR vty password.

## 1. Build

```
git clone git@github.com:HKFree/diagnostika.git
cd diagnostika
cargo build --release
```

Use the committed `Cargo.lock` as is: it pins versions that build with Rust
1.75.

## 2. Write a config

```
mkdir -p ~/.config/hkdiag
install -m 600 config.example.toml ~/.config/hkdiag/config.toml
```

Edit `~/.config/hkdiag/config.toml`: set `[frr] password`, and delete the
`[userdb]` section for now (it needs an API key). hkdiag refuses a config
file that other users can read; that's why it's mode 600.

## 3. Look at the backbone

```
./target/release/hkdiag topology | head
```

You'll see about 200 routers with their neighbour counts and, at the end,
the internet exits (`exit: … E1/E2`). This is the live OSPF database read from
the router at `10.107.3.1`.

## 4. Diagnose an address

Pick a member address and run:

```
./target/release/hkdiag diagnose 10.107.77.5
```

The output shows which subnet and router the address belongs to, the path to
the internet exit with OSPF costs and ping results, the findings, and a
verdict line at the end. Try an address that doesn't exist, for example
`10.107.77.240`, and see the *not routed* verdict.

## 5. Open the app

```
./target/release/hkdiag serve
```

Open <http://127.0.0.1:8087>. Localhost counts as a secure context, so the
full app works, including offline mode. Because the server sees you as
`127.0.0.1`, the app asks for your hkfree.org address: enter the one you used
in step 4 and press **Použít** (Use).

Run the check twice, then open the **Cesta** (Path) tab. It shows the same path
as the CLI in step 4, but computed in the browser from `topology.json`.

## 6. Watch it work offline

In the browser's developer tools, set the network to *Offline* and reload.
The app still opens and runs a check against its saved snapshot.

## What you've achieved

You've seen both halves of hkdiag: the server reading OSPF and exporting a
snapshot, and the app analysing it on the device. Continue with:

- [Deploy or update on lide.hkfree.org](../how-to/deploy-on-lide.md)
- Architecture: `docs/architecture/` (`npx likec4 serve docs/architecture`) and the decisions in `docs/adrs/`
