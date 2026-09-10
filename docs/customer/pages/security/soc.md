# Security

## Purpose

Security detections surfaced by the SOC (security operations) pipeline — a dedicated feed for
security-relevant findings, separate from the general [Alerts](alerts.md) page.

## When to use it

- You want a security-focused view rather than general operational alerts
- You're triaging findings from Veyron's SOC detection rules or an integrated SIEM export
- You're checking whether anything security-relevant has fired before a release or audit

## How to get there

- Left rail: **Security → Security**
- Use the top-bar **Search** to jump here directly
- Toggle **list/grid** view with the icon next to the page title

## What you'll see

Columns (list view):

| Column | Shows |
|---|---|
| Name | The detection name/rule that fired |
| Status | Detection status |
| Severity | Detection severity |
| Message | The detection's description |
| Source | Which detector/collector produced it |
| Seen | When it was last observed |

On a healthy cluster with no active detections, this page shows an empty state ("No detections").
That's the normal, expected state — it means nothing has tripped a security detection rule, not
that the page is broken.

Select a row to open the **Inspector** (right panel):
- Severity
- Message
- Source
- Seen

## What you can do

- **ack** — acknowledge a detection, marking it reviewed.

## Related pages

- [Page index](../../PAGE_INDEX.md)
- [Getting started](../../getting-started.md)
- [Alerts](alerts.md)
