// SPDX-License-Identifier: GPL-3.0-or-later

// Hello — a Spagitty extension.

import { defineExtension, run } from '@spagitty/extension-sdk';

export const extension = defineExtension({
	commands: {
		async hello(ctx) {
			const greeting = String(ctx.settings.greeting ?? 'Hello');
			const repository = ctx.context.repository;
			if (!repository) return { message: `${greeting}!` };
			const described = await ctx.host.describeRepository(repository);
			await ctx.host.notify(`${greeting} from ${described.branch ?? 'a detached HEAD'}`);
			return { message: `${greeting}, ${described.name ?? 'repository'}` };
		}
	},
	panels: {
		async about({ context, host, settings }) {
			const rows: { label: string; value: unknown }[] = [{ label: 'Greeting', value: settings.greeting ?? 'Hello' }];
			if (context.repository) {
				const described = await host.describeRepository(context.repository);
				rows.push({ label: 'Branch', value: described.branch ?? 'detached' });
			}
			return { title: 'Hello', rows };
		}
	}
});

if (import.meta.main) run(extension);
