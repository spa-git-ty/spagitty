<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-028 — Plan

**Item:** [Development styles depend on request order](../items/BUG-028-dev-styles.md).

## Approach

1. Set Svelte's supported `emitCss` option to false only in development, so
   JavaScript and its scoped styles arrive together. Keep production extraction.
2. Exercise the installed compiler/plugin with a cold CSS request before the
   component, verify scoped grid CSS is included, and check production extraction.
3. Check the running server's output, run frontend validation, and record the
   remaining visual sweep honestly.

## Alternatives and risks

Adding more flex/height rules cannot repair a stylesheet containing Svelte
markup. Prewarming a cache still leaves a request-order dependency. A custom
middleware would couple the app to the plugin's internal cache behavior.

Development now injects component styles from JavaScript; hot component updates
carry CSS with them. Production asset delivery is unchanged. Rollback is to
revert this item's configuration and tests; no authored files need deletion.
