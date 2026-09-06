// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, fire, render, type Mounted } from '../../testing/mount';
vi.mock('$lib/api');
import { settings, SECTIONS } from '$lib/settings/store.svelte';
import Page from './+page.svelte';
let view: Mounted;
beforeEach(()=>{ vi.clearAllMocks();  history.replaceState(null,'','#appearance'); });
afterEach(()=>{view?.destroy();});
it('follows the URL fragment and renders each selected settings section',()=>{
 view=render(Page,{}); expect(settings.section).toBe('appearance');
 for(const section of SECTIONS){
  const button=view.all('button').find(b=>b.textContent?.trim()===section.label)!; click(button);
  expect(settings.section).toBe(section.id); expect(location.hash).toBe(`#${section.id}`);
  expect(view.get('.body').children.length).toBeGreaterThan(0);
 }
 history.replaceState(null,'','#behaviour'); fire(window,'hashchange'); expect(settings.section).toBe('behaviour');
});
