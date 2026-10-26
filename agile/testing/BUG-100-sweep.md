<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-100 — Sweep

**Item:** [BUG-100](../items/BUG-100-bruno-yaml-collections-do-not-import.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG100-01 | The DXP collection, saved by Bruno in YAML | Calls › Import › *Choose a Bruno folder…*, pick `DXP` | 227 calls in their folders, ticked; *Bring in* makes the collection and 8 environments | Not run; the author does the sweep |
| SWEEP-BUG100-02 | The ESB collection | *Choose a file…*, pick `ESB/opencollection.yml` | 45 SOAP-shaped POST calls with their XML bodies and `SOAPAction` | Not run |
| SWEEP-BUG100-03 | The DXP collection brought in | Send `auth/login`, then a call under `accounts` | The bearer token from the collection is sent | Not run |
| SWEEP-BUG100-04 | A `.bru` collection | *Choose a Bruno folder…* | Its requests and environments come in | Not run |
