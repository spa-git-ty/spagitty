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
		The scopes stay (TASK-038). Everything else in this section was prose a
		reader could skip; this is the one paragraph that is a set of
		instructions somebody has to follow to get the screen working, and
		hiding it behind a hover would cost them a trip to the host's
		documentation.
	-->
	<p class="note">
		A personal access token, read-only. Fine-grained needs
		<span class="mono">Pull requests: read</span> and <span class="mono">Metadata: read</span>;
		classic needs <span class="mono">repo</span>. Enterprise hosts go in Host.
	</p>

	<!--
		The privacy promise, kept and shortened (TASK-038).
		 
		Four paragraphs stood here. They were not padding — each said something
		true and load-bearing: repositories are never uploaded, the token goes
		only to the host it was issued for, reads never write, the update check
		is the only other request, and the token lives in the keychain. This is
		the one place in the application a reader comes to ask what leaves the
		machine, so none of those claims could be dropped.
		 
		What could go is the *saying it twice*. Each claim is now one clause,
		and the list of them is the paragraph. Nothing here is a sentence about
		another sentence.
	-->
	<p class="note">
		Spagitty reads your repositories from disk and uploads none of them. A connected account adds
		one request: to the host you named, with the token you issued, for pull requests you can
		already see in a browser. It reads — it never approves, merges or comments. The token is in
		this machine's keychain, never in a file; disconnecting deletes it. The only other request is
		the update check under Behaviour, which can be turned off.
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
