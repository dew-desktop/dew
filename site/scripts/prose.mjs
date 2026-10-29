// @ts-check
// Reads a page of `docs/` into the pieces the prose rules look at.
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const DOCS = fileURLToPath(new URL('../../docs/', import.meta.url));

/**
 * One page, split into what the rules look at. Code, front matter and HTML
 * comments are blanked rather than removed, so line numbers stay true.
 *
 * @param {string} file
 */
export function parse(file) {
	const raw = readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
	const relative = path.relative(DOCS, file).replace(/\\/g, '/');
	const lines = raw.split('\n');
	const frontmatter = {};
	let start = 0;
	if (lines[0] === '---') {
		start = lines.indexOf('---', 1) + 1;
		for (const line of lines.slice(1, start - 1)) {
			const m = /^(\w+):\s*(.*)$/.exec(line);
			if (m) frontmatter[m[1]] = m[2].replace(/^["']|["']$/g, '');
		}
	}
	/** @type {{ line: number, text: string, kind: string }[]} */
	const blocks = [];
	/** @type {{ line: number, lang: string }[]} */
	const fences = [];
	let fence = null;
	let comment = false;
	let current = null;
	const flush = () => {
		if (current) blocks.push(current);
		current = null;
	};
	lines.forEach((text, i) => {
		const line = i + 1;
		if (i < start) return;
		if (fence) {
			if (/^\s*```/.test(text)) fence = null;
			return;
		}
		const open = /^\s*```(\S*)/.exec(text);
		if (open) {
			flush();
			fence = true;
			fences.push({ line, lang: open[1] });
			return;
		}
		if (comment || text.includes('<!--')) {
			comment = !text.includes('-->');
			return;
		}
		if (!text.trim()) return flush();
		const heading = /^(#{1,6})\s+(.*)$/.exec(text);
		if (heading) {
			flush();
			blocks.push({ line, text: heading[2], kind: `h${heading[1].length}` });
			return;
		}
		const item = /^\s*([-*+]|\d+\.)\s+(.*)$/.exec(text);
		if (item) {
			flush();
			current = { line, text: item[2], kind: 'item', marker: item[1] };
			return;
		}
		const kind = text.startsWith('|') ? 'table' : text.startsWith('>') ? 'quote' : 'paragraph';
		if (current && current.kind === kind) current.text += ` ${text.trim()}`;
		else {
			flush();
			current = { line, text: text.trim(), kind };
		}
	});
	flush();
	const topic = relative.split('/')[0];
	return { file: relative, raw, lines, start, frontmatter, blocks, fences, topic };
}

/** Prose with code spans and link targets taken out. @param {string} text */
export const prose = (text) =>
	text
		.replace(/`[^`]*`/g, 'CODE')
		.replace(/\]\([^)]*\)/g, ']')
		.replace(/[\[\]]/g, '');

/** @param {string} text */
export const words = (text) => prose(text).match(/[A-Za-z][A-Za-z'-]*/g) ?? [];

/** @param {string} text */
export const sentences = (text) =>
	prose(text)
		.split(/(?<=[.!?])\s+(?=[A-Z(])/)
		.filter((s) => /[A-Za-z]/.test(s));

