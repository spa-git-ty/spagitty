<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-082 — Sweep

**Item:** [BUG-082](../items/BUG-082-saving-the-git-identity-clears-the-fields.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG082-01 | Repository open; Settings → You; global scope | Type name and email; save name, then email | Both values stay visible and are written globally | Command, store and mounted tests; desktop check not run |
| SWEEP-BUG082-02 | Same, repository scope | Repeat 01; return to Settings | Both repository overrides remain; global values are unchanged | Command and mounted tests; desktop check not run |
| SWEEP-BUG082-03 | Global identity plus repository override | Clear a local value and save | Key is unset and global value becomes effective | Covered by command/core tests; desktop check not run |
| SWEEP-BUG082-04 | Configuration cannot be written | Edit and save a field | Error is shown and the typed draft remains | Existing store test; desktop check not run |
| SWEEP-BUG082-05 | No repository open | Save global name and email | Global scope remains editable and saved values survive reload | Existing global read path; desktop check not run |
