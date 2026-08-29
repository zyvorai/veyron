# Client bundle policy — binaries only, no source tree

Customer tarballs from `scripts/package-binary-remote.sh` must **never** require a git clone or compile on the install host. Build happens on **your** remote pack host; the customer gets **artifacts + install scripts only**.

## Bundle types

| Type | Products | What ships | Customer runs |
|------|----------|------------|---------------|
| **A — Native binary** | Veyron, v9s, machina, guestkit, transiva, packetwolf, ragnarok, Axiom, IronWolf | Single executable(s), optional `web/dist` or `frontend/dist`, env example | `./install.sh` → `./binary` or systemd via `install-full.sh` (machina) |
| **B — Container extract** | Veyron, v9s | Binary + UI from OCI image build (still type A at install time) | Same as A |
| **C — Python venv bundle** | **h2kvm**, **forge** | `venv/` with `pip install` already done, wrapper scripts in `bin/`, static UI | `./install.sh`; run `./bin/*` (uses bundled `venv/`) |
| **D — Go multi-binary** | transiva | `bin/hypervisord`, `hyperctl`, … + `dashboard/` | `./bin/hypervisord` |
| **E — K8s cluster add-on** | Veyron, v9s only | `cluster/` YAML + `install-cluster.sh` (not app source) | Cluster admin scripts; app still type A/B |

## What must NOT be in customer tarballs

- Full git tree, `Cargo.toml` / `Makefile` (except optional small `contrib/` snippets machina ships for mkosi defs)
- `target/`, `node_modules/`, `.git/`
- Installers that call `cargo build`, `npm run build`, or `git clone` on the customer host

## Python products (h2kvm, forge) — distribute differently

These are **not** shipped as one static ELF like Rust/Go tools.

1. **Remote build** creates `.pkg-venv` on the pack host (`pip install .` or `requirements.txt`).
2. **Tarball** contains the whole **`venv/`** directory (relocated paths; customer path is fixed at extract dir).
3. **`bin/h2kvm`** (and similar) are **wrappers** that exec `venv/bin/python -m …`.
4. Customer needs **Python 3.10+ system libs** (libvirt, openssl) via `install-client-deps.sh`; they do **not** need to run `pip install` again unless recreating the venv.

h2kvm additionally bundles **Go `h2kweb`** as a native binary; dashboard is static files under `web/dashboard/`.

forge bundles **minimal** `services/api-gateway/` (entry module) + full **venv** + `ui/dist/`.

## Per-product checklist

| Product | Type | Sources in tarball? | Notes |
|---------|------|---------------------|--------|
| Veyron | B→A | No | `veyron` + optional `virtctl`; cluster scripts only |
| v9s | B→A | No | `v9s-web` + `ui/dist` |
| machina | A | No | `machina-daemon`, TUI, `web/dist`; `install-full.sh` = bundle-aware host install |
| guestkit | A | No | `guestkit` binary only |
| transiva | D | No | `bin/*` + `dashboard/` |
| h2kvm | C + Go | No app source | **`venv/` required**; not a single static binary |
| packetwolf | A | No | `packetwolf-api` + `ui/` |
| ragnarok | A | No | `ragnarok` + `frontend/dist` |
| Axiom | A | No | `axiom` (embedded UI) |
| IronWolf | A | No | `ironwolf-web` + `dashboard/dist` |
| forge | C | No full tree | **`venv/`** + thin API tree + `ui/dist` |

## Customer install scripts

| Script | Purpose |
|--------|---------|
| **`install-everything.sh`** | **Recommended.** Runs `install.sh`, host tests, optional `install-full.sh`, smoke test — prints LAN URL |
| `install.sh` | Automatic: deps, config from `.example`, verify binaries, tests; machina also runs `install-full.sh` |
| `install-full.sh` | **Machina only:** production host (systemd, TLS, firewall). Called automatically unless `ZYVOR_AUTO_INSTALL=0` |

Tarballs include `.package-lib/product.meta` (scheme, port, finish hints) and `ZYVOR_INSTALL.txt`.

Do not copy the repo root `install.sh` into tarballs without **bundle mode** (machina).

## Remote pack script naming

Phases say **“Sync to build host”** (rsync for remote *build*, not customer source). Customer-facing docs must say **“extract tarball”**, not “clone repo”.

## Adding a new product

1. Pick type **A**, **C**, or **D** above.
2. Implement `scripts/lib/package-install.sh` to only reference paths **inside the tarball**.
3. In `package-binary-remote.sh` REMOTE_PACK: copy artifacts only; for Python use **venv + wrappers**.
4. Never `cp` the repo’s full `install.sh` unless it detects bundle layout (like machina).

See also: `docs/PACKAGE_BINARY_REMOTE.md` per repo.
