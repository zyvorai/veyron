# Windows domain join (Secret refs)

Example Secrets for **VeyronVM** / blueprint specs that use `userDataSecretRef`, `windows.sysprepSecretRef`, and `windows.domainJoinSecretRef` instead of inline passwords.

## Apply

```bash
kubectl apply -f secrets.yaml
```

Edit placeholder values before production use. Wire Secret names into your `VeyronBlueprint` or `VeyronVM` spec — see [docs/architecture.md](../../docs/architecture.md) and [operator/config/samples/windows-ad-blueprint.yaml](../../operator/config/samples/windows-ad-blueprint.yaml).

## Related

- Windows production runbook: [docs/windows.md](../../docs/windows.md)
- Built-in role blueprints: `veyron blueprints list` (jumpbox, ad, ad-member, rds, iis, sql, dev)
