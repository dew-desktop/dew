// @ts-check
// Checks the prose in `docs/` against the rules in `style-rules.mjs`.
//
//   node scripts/lint-docs.mjs            every finding, by file
//   node scripts/lint-docs.mjs --summary  a count per rule, to compare rules by cost
//
// A rule at `error` fails the run; `warn` is reported and passes; `off` is skipped.
// The checks read the Markdown source, so a finding names the line to change.
import { readdirSync } from 'node:fs';
import path from 'node:path';
import { RULES } from '../style-rules.mjs';
import { DOCS, parse } from './prose.mjs';

/** @param {string} dir @returns {string[]} */
const markdown = (dir) =>
	readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) return markdown(full);
		return entry.name.endsWith('.md') && !entry.name.startsWith('_') ? [full] : [];
	});

const files = markdown(DOCS).map(parse);
/** @type {{ rule: string, level: string, file: string, line: number, message: string }[]} */
const findings = [];
for (const [id, rule] of Object.entries(RULES)) {
	for (const page of files) {
		const level = typeof rule.level === 'function' ? rule.level(page) : rule.level;
		if (level === 'off') continue;
		for (const { line, message } of rule.check(page)) {
			findings.push({ rule: id, level, file: page.file, line, message });
		}
	}
}

if (process.argv.includes('--summary')) {
	const byRule = new Map();
	for (const f of findings) byRule.set(f.rule, (byRule.get(f.rule) ?? 0) + 1);
	for (const [id, rule] of Object.entries(RULES)) {
		console.log(`${String(byRule.get(id) ?? 0).padStart(4)}  ${id.padEnd(28)} ${rule.source.padEnd(8)} ${rule.summary}`);
	}
} else {
	for (const f of findings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line)) {
		console.log(`${f.file}:${f.line}  ${f.level === 'error' ? 'error' : 'warn '}  ${f.rule}  ${f.message}`);
	}
}

const errors = findings.filter((f) => f.level === 'error').length;
console.log(`\nprose: ${findings.length} finding(s) in ${files.length} page(s), ${errors} error(s)`);
process.exit(errors ? 1 : 0);
