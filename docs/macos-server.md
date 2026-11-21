<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 -->
# Veyron, Kairon and FluxVM on one Mac

Run the whole stack on a single Apple Silicon Mac: [FluxVM](https://github.com/zyvorai/zyvor-fluxvm) runs the VMs
(Virtualization.framework), [Kairon](https://github.com/zyvorai/zyvor-kairon) turns Kubernetes `Machine`s into FluxVM
calls, and Veyron is the console and API on top. Each piece runs as a launchd **LaunchAgent** of the logged-in user, not a
LaunchDaemon: FluxVM restores saved VM state only in an unlocked login session.

> **Status.** Each piece has its own installer and offline test. The full chain below has **not** been run end to end
> on a Mac. Treat the steps as the intended recipe, and report what differs.

`veyron-darwin-arm64` is a release binary and the console is embedded. [macos-app.md](macos-app.md) is the native
*client* window; this page is about running the *server* on a Mac.

## How the pieces connect

```
Veyron (api-serve, :5151) ──Kubernetes API──▶ Kairon CRDs (Machine, …) ◀── kairon-controller
        │                                                    ▲
        └── console relay (:8090, bearer token) ──▶ kairon-node ──▶ FluxVM daemon (127.0.0.1:7788, vz)
```

Veyron never calls FluxVM directly. It needs a **Kubernetes API with the Kairon CRDs**; there is no standalone or demo
mode. On a Mac, FluxVM can provide that cluster itself: `fluxctl kube up` runs a single-node k3s in a VM.

## 1. FluxVM daemon and a Kubernetes cluster

```bash
# FluxVM's prebuilt macOS tarballs and Homebrew formula were withdrawn after 0.4.21: build from source
# (xcode-select --install && brew install hivex && cargo build --release -p fluxctl), or use FluxVM for Mac.
fluxctl service install                # the daemon as a LaunchAgent, API on 127.0.0.1:7788
fluxctl doctor                         # expect the daemon ok and no "locked" warning

fluxctl kube up --kubeconfig ~/.kube/veyron-mac     # k3s in a VM; the first run downloads k3s and takes minutes
export KUBECONFIG=~/.kube/veyron-mac
kubectl get nodes
```

## 2. Kairon

From a Kairon checkout (or its release assets), against that cluster:

```bash
kubectl apply -f deploy/crd.yaml -f deploy/rbac.yaml -f deploy/macos/rbac.yaml
```

Run `kairon-controller` (in the cluster via the Helm chart, or on any host that can reach the API), then run
`kairon-node` on this Mac under launchd with `scripts/macos-node.sh` from the Kairon repo
(`docs/runbook-macos-node.md` there). Give it a console token so Veyron can reach VNC, exec and logs:

```
# ~/.config/kairon/kairon-node.env  (chmod 600)
NODE_NAME=mac-1
KAIRON_KUBE_URL=https://<k3s api>:6443
KAIRON_KUBE_TOKEN=<kairon-node ServiceAccount token>
KAIRON_KUBE_CA=/path/to/ca.pem
KAIRON_NODE_CONSOLE_TOKEN=<random token>
```

The node registers this Mac as a Kubernetes Node (`kubernetes.io/os=darwin`, `kairon.zyvor.dev/backend.vz=true`).
`kubectl get node mac-1 --show-labels` should show it Ready.

## 3. Veyron

```bash
mkdir -p ~/.config/veyron && chmod 700 ~/.config/veyron
cat > ~/.config/veyron/veyron.env <<'EOF2'
VEYRON_API_KEY=<a long random key>
KUBECONFIG=/Users/you/.kube/veyron-mac
KAIRON_NODE_CONSOLE_TOKEN=<the same token kairon-node uses>
VEYRON_KAIRON_NODE_URL=http://127.0.0.1:8090
EOF2
chmod 600 ~/.config/veyron/veyron.env

scripts/macos-server.sh install --bin "$(command -v veyron)"
scripts/macos-server.sh status
curl -s http://127.0.0.1:5151/api/v1/health
open http://127.0.0.1:5151/console
```

`VEYRON_KAIRON_NODE_URL` matters on a single Mac: the default template (`http://{node_ip}:8090`) looks the node up by its
Kubernetes Node address. Veyron.app accepts plain `http` only for `localhost`; for HTTPS or other machines pass
`--tls-cert`/`--tls-key` and `--host 0.0.0.0` to the installer.

`scripts/macos-server.sh` keeps the API key out of the plist (the agent runs `sh -c '. ENV_FILE; exec veyron api-serve ...'`),
refuses an env file that anyone else can read, logs to `~/Library/Logs/veyron.log`, and has `render` (print the plist) and
`uninstall`. `scripts/test-macos-server.sh` checks it offline.

## Limits to know

- A Mac guest needs an Apple Silicon host. Veyron features that rely on QEMU or KVM (live migration, eBPF, GPU passthrough,
  CPU pinning) are not available on the `vz` backend.
- Restoring a saved VM needs an unlocked login session; on a headless Mac use a dedicated auto-login user
  (see FluxVM's `deploy/launchd-notes.md`).
- Building Veyron from source needs the `guestkit` checkout next to it (path dependency) and `frontend/dist` built first;
  prefer the release binary.
