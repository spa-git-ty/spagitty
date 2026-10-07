<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { AgentDefinition } from '../types';
	import { agentColour, agentMonogram } from '../describe';
	let {
		agent,
		running = false,
		large = false
	}: { agent?: AgentDefinition; running?: boolean; large?: boolean } = $props();
</script>

<span
	class="agent-badge"
	class:running
	class:large
	style:--agent-colour={agentColour(agent)}
	title={agent?.displayName}>{agentMonogram(agent)}</span
>

<style>
	.agent-badge {
		display: inline-grid;
		place-items: center;
		width: 26px;
		height: 26px;
		flex-shrink: 0;
		border-radius: 50%;
		border: 1px solid var(--agent-colour);
		color: var(--agent-colour);
		background: color-mix(in srgb, var(--agent-colour) 14%, transparent);
		font: 600 var(--fs-mono) var(--font-mono);
	}
	.large {
		width: 44px;
		height: 44px;
		font-size: var(--fs-ui);
	}
	.running {
		animation: farm-halo 3s ease-in-out infinite;
	}
	@keyframes farm-halo {
		50% {
			box-shadow: 0 0 0 3px color-mix(in srgb, var(--agent-colour) 15%, transparent);
		}
	}
</style>
