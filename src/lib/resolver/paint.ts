// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Syntax colours for the resolver's lines (FEAT-102), from the shared
 * highlighter (FEAT-064). A run of lines is read as one block, so a comment or
 * string opened on one line carries to the next; a Svelte or Vue fragment,
 * which may start anywhere in the file, is read line by line as script.
 */

import { detectLanguage, tokenize, tokenizeLines, type Token } from '../diff/highlight';

export function languageOf(path: string): string {
	return detectLanguage(path);
}

export function paint(lines: string[], language: string): Token[][] {
	if (language === 'svelte' || language === 'vue') return lines.map((line) => tokenize(line, language));
	return tokenizeLines(lines, language);
}
