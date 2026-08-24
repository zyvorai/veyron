# Licensing & evaluation (Veyron)

Veyron evaluation builds (`--features trial` / customer tarball with trial
enforcement) use a **signed trial token** — the same design as Ragnarok / Aurora
/ Argus Enterprise.

## Customer install

**Current evaluation release:** [`v0.2.0-trial`](https://github.com/ssahani/Veyron/releases/tag/v0.2.0-trial) (tarball includes `trial.token`).

1. Extract the tarball. It should include **`trial.token`** next to `veyron`.
2. Keep that file beside the binary (or copy to `~/.config/veyron/trial.token`).
3. Optional override:
   ```bash
   export VEYRON_TRIAL_TOKEN="$(cat trial.token)"
   ```
4. Run as usual (`./install.sh`, then `veyron` / env from `veyron.env`).

Expiry is inside the JWT. Deleting local config does not extend the trial.
After expiry, email **sales@zyvor.dev** for a renewed `trial.token`.

## Sales (private repo only)

```bash
cargo run --features trial --bin trial-tool -- keygen
cargo run --features trial --bin trial-tool -- issue --who "Acme" --days 30 -o trial.token
```

Embed the printed public key in `src/trial.rs` (`TRIAL_PUBLIC_KEY_B64`). Never ship
the private PKCS8 or `trial-tool` in customer packages.

See also: [PACKAGE_BINARY_REMOTE.md](PACKAGE_BINARY_REMOTE.md).
