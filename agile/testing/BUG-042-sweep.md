<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-042 — Manual sweep

**Item:** [`agile/items/BUG-042-a-corporate-ca-is-not-trusted.md`](../items/BUG-042-a-corporate-ca-is-not-trusted.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG042-01 | Release build; a GitLab signed by a CA in the OS store | 1. Settings, add a GitLab token for that host | The token is accepted; no "could not reach" | P1 | |
| SWEEP-BUG042-02 | Release build | 1. Open Pull requests on a github.com repository | Pull requests load as before | P1 | |
