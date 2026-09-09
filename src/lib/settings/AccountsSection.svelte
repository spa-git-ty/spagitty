<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import { settings } from './store.svelte';

	/**
	 * Connecting a hosting account, so the Pull requests screen has something to
	 * read (FEAT-017).
	 *
	 * **A token, not a password and not an OAuth dance.** A personal access
	 * token is issued by the person, scoped by the person, and revoked by the
	 * person without touching anything else they own — and connecting one needs
	 * no browser handoff, no redirect listener, and no client secret shipped
	 * inside a GPL binary that anybody can read. The trade is that they have to
	 * go and make one, which is a paragraph of instructions rather than a design
	 * problem.
	 *
	 * The login is **not** typed. It is read back from the host when the token
	 * is proved, so it cannot be got wrong in a way that would quietly stop
	 * "waiting on you" from meaning anything.
	 *
	 * The token is never held here longer than the moment it is submitted, and
	 * the field is cleared whether or not it worked. It goes to the OS keychain,
	 * and nothing ever reads it back into this screen.
	 */

	let host = $state('github.com');
	let token = $state('');

	const accounts = $derived(settings.accounts);
	const busy = $derived(settings.busy);

	async function connect() {
		const ok = await settings.connectAccount(host.trim(), token);
		// Cleared either way. A token left in a field is a token in the DOM, and
		// one that was refused is no less a secret than one that worked.
		token = '';
		if (ok) host = 'github.com';
	}
</script>

<section class="section" id="accounts">
	<header>
		<h2 class="heading">Accounts</h2>
		<span class="note">
			{accounts.length === 0 ? 'No account is connected.' : 'Connected.'}
		</span>
	</header>

	{#each accounts as account (account.host + account.user)}
		<div class="row">
			<div class="text">
				<div>
					<span class="mono">{account.user}</span> on <span class="mono">{account.host}</span>
				</div>
			</div>
			<Btn
				disabled={busy}
				title="Forget this account and delete its token from the keychain"
				onclick={() => settings.disconnectAccount(account.host, account.user)}
			>
				Disconnect
			</Btn>
		</div>
	{/each}

	<div class="hr"></div>

	<div class="field-row">
		<label class="label" for="account-host">Host</label>
		<input id="account-host" class="field" type="text" placeholder="github.com" bind:value={host} />
	</div>

	<div class="field-row">
		<label class="label" for="account-token">Token</label>
		<!--
			`type="password"` so it is not shoulder-read. It is never read back
			out of the keychain into this screen.
		-->
		<input
			id="account-token"
			class="field"
			type="password"
			autocomplete="off"
			placeholder="a personal access token"
			bind:value={token}
		/>
		<Btn primary disabled={busy || token.trim() === '' || host.trim() === ''} onclick={connect}>
			Connect
		</Btn>
	</div>

	<!--
		Scopes, because nobody can guess them; everything else was prose
		(TASK-044).

		The privacy paragraph before this was five sentences and it was five
		sentences of reassurance. What it actually said — read-only, host you
		named, keychain, deleted on disconnect — is four words a piece and fits
		on one line. A reader who wants the argument can hover.
	-->
	<p class="note">
		Read-only token. Fine-grained: <span class="mono">Pull requests: read</span> +
		<span class="mono">Metadata: read</span>. Classic: <span class="mono">repo</span>.
	</p>

	<p
		class="note"
		title="Spagitty uploads no repository. A connected account adds one request — to the host you named, with the token you issued, for pull requests you can already see in a browser. It never approves, merges or comments. The only other request is the update check under Behaviour."
	>
		Read-only, to the host you named. Token in the keychain; disconnecting deletes it.
	</p>
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 640px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.row,
	.field-row {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.row {
		justify-content: space-between;
	}

	.label {
		width: 48px;
		flex: none;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.text {
		min-width: 0;
	}

	.field {
		background: transparent;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		padding: 3px 6px;
		width: 260px;
	}

	.field:focus {
		outline: none;
		border-color: var(--accent);
	}

	.field::placeholder {
		color: var(--placeholder);
	}

	p {
		margin: 0;
	}
</style>
