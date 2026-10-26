# Design handoff — Calls: REST and SOAP requests beside the code

The spec is [`docs/proposals/calls.md`](../docs/proposals/calls.md). The screens show it. When they disagree, the spec wins on behaviour and the screens win on layout.

What is proposed is six backlog items:

| Item | Screen |
| --- | --- |
| FEAT-160 — Calls: send a REST call and keep the collection in the repository | Calls (2C) |
| FEAT-161 — Environments, secrets and values kept from responses | Calls › Environments |
| FEAT-162 — Import calls from code and from collection files | Calls › Import |
| FEAT-163 — SOAP calls from a WSDL | Calls, a SOAP call |
| FEAT-164 — Share a collection as a file, a zip or a commit | Calls › Share |
| FEAT-165 — Settings › Calls | Settings › Calls (1K) |

FEAT-160 waits for the author's answer to the spec's question 1 (the network boundary). Take the items in the spec's order (§13).

In the repository, this folder sits beside `design_handoff_workbench/` and the other handoffs.

## Contents

```
screens/  PNG renders, 1440×900 @2x, dark theme
source/   the editable artboards (.dc.html) + canvas.json + support.js
```

Open any `source/*.dc.html` straight in a browser: `support.js` is the small stand-alone viewer from the earlier handoffs, and it fills the template. Add `?theme=light` to the address for the light theme. The interactive artboards work there too. `support.js` is not part of Spagitty and nothing in the app should import it.

## Artboards

PNGs in `screens/`, in this order: `01-calls`, `02-import`, `03-environments`, `04-soap-call`, `05-share`, `06-settings-calls`.

| Source | Item | Spec | Shows |
| --- | --- | --- | --- |
| `Main.dc.html` | FEAT-160, 161 | §5, §6.2 | Calls: open calls with ✕, collections, a POST with variables, a JSON response with the keep card, a kept `userId`, *Beautify*. **Interactive**: click a value and keep it, *Forget*, toggle *Beautify* |
| `Scan.dc.html` | FEAT-162 | §8 | Import from Spring controllers: sources, endpoints tagged New, Changed or In orders-api, *Bring in*. **Interactive**: tick rows, filter, switch *As* |
| `Environments.dc.html` | FEAT-161 | §6.1 | Staging's variables by kind and where each is kept, *Kept from responses*. **Interactive**: switch environment, *Ask before sending*, freeze a kept value |
| `SoapCall.dc.html` | FEAT-163 | §7 | A SOAP call from a WSDL: operation picker, generated envelope with WS-Security, XML response with the keep card on an XPath |
| `Share.dc.html` | FEAT-164 | §5.5, §7.3 | One file, zip, or commit; what goes in; the preview. **Interactive**: every choice updates the preview and the button |
| `SettingsCalls.dc.html` | FEAT-165 | §9 | Settings › Calls. The page scrolls; the PNG shows its top |

## Rules to keep

- **The chrome is context, not design.** The title bar, the rail, the toolbar and the status strip were drawn to match the earlier handoffs. Do not change those components to match these artboards. The only chrome change is the rail's new *Calls* row (2C).
- **Fidelity** means structure, copy and colour meaning. Exact sizes and colours come from `metrics.ts` and `app.css`.
- **Method colours**: GET `--ok`, POST blue, PUT `--warn`, PATCH violet, DELETE `--danger`, SOAP `--agent`. Map them to existing tokens, and add none if one is close.
- **Environment colours mean something**: red is for production, and an environment coloured red asks before sending.
- **Kept values are `--ok`** everywhere: the variable name, the dotted underline in a response, the `→ name` tag.
- **Secrets are never drawn in the clear.** Dots, and a lock where a kind is shown.
- **Calm, not toolbox.** No icon rows, no side tabs of features. A tab appears only when it holds something.

## Notes

- The artboards load Atkinson Hyperlegible from Google Fonts. The PNGs were rendered offline, so they use system fonts. Use the app's bundled fonts.
- The `calls` rail icon (two opposed arrows) is in each artboard's script, as `ICONS.calls`.
- **Illustration**: the data in the scripts (services, people, tokens, invoices) is made up.

## Index rows for `agile/README.md`

When the items are recorded, add these rows to the Features table, after FEAT-159. `tools/record.test.ts` checks them.

```
| [FEAT-160](items/FEAT-160-calls-send-a-rest-call-and-keep-the-collection-in-the-repository.md) | Calls: send a REST call and keep the collection in the repository | 2C | Backlog |
| [FEAT-161](items/FEAT-161-environments-secrets-and-values-kept-from-responses.md) | Environments, secrets and values kept from responses | 2C | Backlog |
| [FEAT-162](items/FEAT-162-import-calls-from-code-and-from-collection-files.md) | Import calls from code and from collection files | 2C | Backlog |
| [FEAT-163](items/FEAT-163-soap-calls-from-a-wsdl.md) | SOAP calls from a WSDL | 2C | Backlog |
| [FEAT-164](items/FEAT-164-share-a-collection-as-a-file-a-zip-or-a-commit.md) | Share a collection as a file, a zip or a commit | 2C | Backlog |
| [FEAT-165](items/FEAT-165-settings-calls.md) | Settings › Calls | 1K | Backlog |
```
