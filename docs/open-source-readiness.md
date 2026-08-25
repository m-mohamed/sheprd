# Open-source readiness

| Surface | Contract | Evidence |
|---|---|---|
| Product boundary | Herdr runtime, Sheprd project router | README, agent guide, product foundation |
| Runtime safety | explicit focus/create, no implicit reshape | CLI tests and Herdr wrappers |
| Configuration | project roots and selected agent only | `init` and `show-config` |
| Failure behavior | structured JSON errors, no mutation on failed resolution | CLI failure smoke |
| Active fleet | HQ Sol/Luna launcher with explicit scope and receipts | HQ workflow documentation |
| Operator control | Ratatui cockpit over task/runtime/evidence surfaces | dotfiles factory-ui |

Retired peer-agent and legacy fleet modes are intentionally absent from the
public contract.
