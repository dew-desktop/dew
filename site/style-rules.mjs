// @ts-check
// THE PROSE RULES FOR `docs/`, each with the check that enforces it.
//
// `source` says where a rule comes from: Roblox's documentation (its STYLE.md, its
// markdownlint config, and what its pages measurably do), Dew's own house style,
// or both. `level` is `error`, `warn` or `off`, or a function of the page. A page's
// kind follows its topic: `spec/` is the specification, `reference/` is reference,
// and everything else is a guide.
//
// Chosen 2026-10-13: headings are labels; a page says what before why, and a spec
// page keeps its reasoning in a closing "Design notes" section; emphasis is bold for
// a defined term and callouts, with RFC 2119 keywords in capitals in spec pages
// only; prose is in American spelling, as the API names beside it are. `ADOPTED`
// at the end of this file lists the rules that fail the build. The rest are
// heuristics whose findings need judgement, and only warn.
import { sentences, words } from './scripts/prose.mjs';

/** @typedef {ReturnType<typeof import('./scripts/prose.mjs').parse>} Page */
/** @typedef {{ line: number, message: string }} Finding */
/**
 * @typedef {object} Rule
 * @property {'Roblox' | 'Dew' | 'Both'} source
 * @property {string} summary
 * @property {'error' | 'warn' | 'off' | ((page: Page) => 'error' | 'warn' | 'off')} level
 * @property {(page: Page) => Iterable<Finding>} check
 */

/** @param {Page} page */
const kind = (page) => (page.topic === 'spec' ? 'spec' : page.topic === 'reference' ? 'reference' : 'guide');
/** @param {Page} page */
const headings = (page) => page.blocks.filter((b) => /^h[2-4]$/.test(b.kind));
/** @param {Page} page */
const text = (page) => page.blocks.filter((b) => b.kind === 'paragraph' || b.kind === 'item');

/** @param {Page} page @param {RegExp} pattern @param {(match: string) => string} message */
function* matches(page, pattern, message) {
	for (const block of text(page)) {
		for (const m of block.text.replace(/`[^`]*`/g, (c) => ' '.repeat(c.length)).matchAll(pattern)) {
			yield { line: block.line, message: message(m[0]) };
		}
	}
}

/** @type {Record<string, Rule>} */
export const RULES = {
	// STRUCTURE ---------------------------------------------------------------

	'intro-before-sections': {
		source: 'Roblox',
		summary: 'a page opens with prose that says what the subject is, before its first heading',
		level: 'warn',
		*check(page) {
			const first = page.blocks[0];
			if (first && /^h[2-4]$/.test(first.kind)) yield { line: first.line, message: 'the page starts with a heading; open with what the subject is' };
		},
	},
	'what-before-why': {
		source: 'Roblox',
		summary: 'the first section says what the thing is or does; rationale comes after',
		level: 'warn',
		*check(page) {
			const first = headings(page)[0];
			if (first && /^(why|how this|what this does not)\b/i.test(first.text)) yield { line: first.line, message: `first section is rationale: "${first.text}"` };
		},
	},
	'no-rationale-sections': {
		source: 'Dew',
		summary: 'no section that defends a design; a reason that helps the reader is one sentence, inline',
		level: 'warn',
		*check(page) {
			for (const h of headings(page)) {
				if (/^(why\b|design notes|rationale|background|history|motivation)/i.test(h.text)) yield { line: h.line, message: `"${h.text}"` };
			}
		},
	},
	'reader-vocabulary': {
		source: 'Dew',
		summary: 'no words from inside Dew: crates, internal functions, guests, backlog, conformance; Roblox by name',
		level: 'warn',
		check: (page) =>
			matches(
				page,
				/\b(crates\/\w+|dew_host|guests?|the engine|engine's|upstream|backlog|test doubles?|display list|rasteri[sz]\w*|registry|conform(ing|ance)|mods?)\b/gi,
				(m) => `"${m}": ${/engine/i.test(m) ? 'say Roblox' : /mods?$/i.test(m) ? 'say applet' : 'a reader does not need this word'}`,
			),
	},
	'dew-not-the-host': {
		source: 'Dew',
		summary: 'guides and reference say "Dew", not "the host"',
		level: (page) => (kind(page) === 'spec' ? 'off' : 'warn'),
		check: (page) => matches(page, /\b(the|a) host\b/gi, (m) => `"${m}": say Dew`),
	},
	'paragraph-length': {
		source: 'Roblox',
		summary: 'a paragraph is at most 80 words (Roblox pages peak at 86; Dew spec pages reach 135)',
		level: 'warn',
		*check(page) {
			for (const b of page.blocks.filter((b) => b.kind === 'paragraph')) {
				const n = words(b.text).length;
				if (n > 80) yield { line: b.line, message: `${n} words in one paragraph` };
			}
		},
	},
	'sentence-length': {
		source: 'Both',
		summary: 'a sentence is at most 35 words',
		level: 'warn',
		*check(page) {
			for (const b of text(page)) for (const s of sentences(b.text)) {
				const n = words(s).length;
				if (n > 35) yield { line: b.line, message: `${n}-word sentence: "${s.slice(0, 60)}..."` };
			}
		},
	},
	'lists-for-enumerations': {
		source: 'Roblox',
		summary: 'four or more parallel items in one sentence become a list',
		level: 'warn',
		*check(page) {
			for (const b of page.blocks.filter((b) => b.kind === 'paragraph')) {
				const spans = b.text.match(/`[^`]+`(?:,| and| or)/g) ?? [];
				if (spans.length >= 3) yield { line: b.line, message: `${spans.length + 1} items enumerated inline` };
			}
		},
	},

	// HEADINGS ----------------------------------------------------------------

	'heading-length': {
		source: 'Roblox',
		summary: 'a heading is a label of at most five words (Roblox averages 1.3 to 2.7; Dew spec 7.5)',
		level: 'warn',
		*check(page) {
			for (const h of headings(page)) {
				const n = h.text.replace(/`[^`]*`/g, 'X').split(/\s+/).length;
				if (n > 5) yield { line: h.line, message: `${n}-word heading: "${h.text}"` };
			}
		},
	},
	'heading-punctuation': {
		source: 'Roblox',
		summary: 'a heading has no sentence punctuation: no ; , : . ? or !',
		level: 'warn',
		*check(page) {
			for (const h of headings(page)) if (/[;,:.?!]/.test(h.text.replace(/`[^`]*`/g, ''))) yield { line: h.line, message: `"${h.text}"` };
		},
	},
	'heading-sentence-case': {
		source: 'Both',
		summary: 'headings and titles are in sentence case',
		level: 'warn',
		*check(page) {
			const all = [...headings(page)];
			if (page.frontmatter.title) all.unshift({ line: 2, text: page.frontmatter.title, kind: 'title' });
			for (const h of all) {
				const ws = h.text.replace(/`[^`]*`|\([^)]*\)/g, '').split(/\s+/).slice(1).filter((w) => /^[a-z]/i.test(w));
				const capped = ws.filter((w) => /^[A-Z][a-z]/.test(w) && !/^(Dew|DataModel|Luau|Roblox|Aether|Windows|Starlight|Stripe|GitHub|Vello|I)$/.test(w));
				if (capped.length >= 2) yield { line: h.line, message: `title case: "${h.text}"` };
			}
		},
	},
	'heading-no-numbers': {
		source: 'Roblox',
		summary: 'headings are not numbered by hand; a task page sets `steps: true`',
		level: 'warn',
		*check(page) {
			for (const h of headings(page)) if (/^\d+\.\s/.test(h.text)) yield { line: h.line, message: `"${h.text}"` };
		},
	},
	'heading-no-questions': {
		source: 'Roblox',
		summary: 'a heading names its section rather than asking a question',
		level: 'warn',
		*check(page) {
			for (const h of headings(page)) if (/\?\s*$/.test(h.text)) yield { line: h.line, message: `"${h.text}"` };
		},
	},

	// VOICE -------------------------------------------------------------------

	'second-person-in-guides': {
		source: 'Roblox',
		summary: 'a guide speaks to the reader, as "you" or in the imperative, never about them',
		level: (page) => (kind(page) === 'guide' ? 'error' : 'off'),
		check: (page) =>
			matches(page, /\b(the reader|the user(?! interface)|an? (applet |mod )?author|the author|an? developer|the developer)\b/gi, (m) => `"${m}": address the reader directly`),
	},
	'no-we': {
		source: 'Roblox',
		summary: '"we" and "our" are not used; the subject is the reader, the host or the applet',
		level: 'warn',
		check: (page) => matches(page, /\b(we|our|we're|us)\b/gi, (m) => `"${m}"`),
	},
	'active-voice': {
		source: 'Roblox',
		summary: 'active voice: the sentence names who acts',
		level: 'warn',
		check: (page) => matches(page, /\b(is|are|was|were|be|been|being)\s+(\w+ly\s+)?(\w+ed|given|taken|written|shown|known|seen|made|done|built|run|held|kept|read)\b/gi, (m) => `passive: "${m}"`),
	},
	'modal-verbs': {
		source: 'Roblox',
		summary: 'must for requirements, should for recommendations, can for options; never "may"',
		level: 'warn',
		check: (page) => matches(page, /\bmay\b/gi, () => '"may": use "can", "might" or "must"'),
	},
	'no-backstory': {
		source: 'Dew',
		summary: 'no history or planning vocabulary: say what the thing is, not what it replaced',
		level: 'warn',
		check: (page) =>
			matches(
				page,
				/\b(used to|previously|the previous (shape|version|design|approach|form)|originally|until now|the first attempt|mid-migration|milestones?|sprints?|ADR-?\d+)\b/gi,
				(m) => `"${m}"`,
			),
	},
	'no-minimisers': {
		source: 'Dew',
		summary: 'never "simply", "just", "obviously", "of course", "easy"',
		level: 'warn',
		check: (page) => matches(page, /\b(simply|just|obviously|of course|easy|easily)\b/gi, (m) => `"${m}"`),
	},

	// FORMATTING --------------------------------------------------------------

	'bold-for-terms': {
		source: 'Roblox',
		summary: 'bold marks a term where it is defined, or a UI label; not a sentence',
		level: 'warn',
		*check(page) {
			for (const b of text(page)) for (const m of b.text.matchAll(/\*\*([^*]+)\*\*/g)) {
				const n = m[1].split(/\s+/).length;
				if (n > 4) yield { line: b.line, message: `${n} words in bold: "${m[1].slice(0, 50)}"` };
			}
		},
	},
	'no-caps-emphasis': {
		source: 'Roblox',
		summary: 'no capitals for emphasis; a spec page may use the RFC 2119 keywords MUST, SHOULD, REQUIRED',
		level: 'warn',
		*check(page) {
			const rfc = kind(page) === 'spec' ? /\b(MUST|SHOULD)( NOT)?\b|\b(REQUIRED|OPTIONAL)\b/g : /(?!)/g;
			for (const b of text(page)) {
				const plain = b.text.replace(/`[^`]*`/g, '').replace(rfc, '');
				for (const m of plain.matchAll(/\b(?!API|URL|UI|DPI|CPU|GPU|RAM|ASCII|MIT|TOML|JSON|PNG|HTML|CSS|ID|OS|FPS|ABI|FFI|VM|CLI|SDK|RFC|AA|UTF|URI|DOM)[A-Z]{3,}\b/g)) {
					yield { line: b.line, message: `"${m[0]}"` };
				}
			}
		},
	},
	'no-dashes': {
		source: 'Dew',
		summary: 'no dashes as punctuation: no " -- ", em dash or en dash',
		level: 'warn',
		check: (page) => matches(page, /\s--\s|—|–/g, () => 'dash used as punctuation'),
	},
	'ascii-only': {
		source: 'Dew',
		summary: 'plain ASCII: no curly quotes, arrows or symbols',
		level: 'warn',
		*check(page) {
			for (const [i, line] of page.lines.entries()) for (const m of line.matchAll(/[^\x00-\x7F]/g)) yield { line: i + 1, message: `U+${m[0].codePointAt(0)?.toString(16).toUpperCase().padStart(4, '0')}` };
		},
	},
	'no-possessive-code': {
		source: 'Roblox',
		summary: 'no possessive on a code name: "the size of `Frame`", not "`Frame`\'s size"',
		level: 'warn',
		*check(page) {
			for (const b of text(page)) for (const m of b.text.matchAll(/`[^`]+`'s\b/g)) yield { line: b.line, message: m[0] };
		},
	},
	'numbers-as-words': {
		source: 'Roblox',
		summary: 'one to nine are spelled out unless they carry a unit',
		level: 'warn',
		check: (page) => matches(page, /(?<![\w.,/-])[1-9](?![\w.,%:/-]|\s*(px|ms|fps|x|KB|MB|GB|seconds?|minutes?|bytes?|pixels?|bits?)\b)/g, (m) => `"${m}"`),
	},
	'parentheses': {
		source: 'Dew',
		summary: 'colons and full stops over parenthetical asides: at most one aside per paragraph',
		level: 'warn',
		*check(page) {
			for (const b of text(page)) {
				const n = (b.text.replace(/`[^`]*`|\]\([^)]*\)/g, '').match(/\(/g) ?? []).length;
				if (n > 1) yield { line: b.line, message: `${n} parenthetical asides` };
			}
		},
	},
	'american-spelling': {
		source: 'Roblox',
		summary: 'American spelling in prose, as in the API names beside it: color, behavior, initialize',
		level: 'warn',
		check: (page) =>
			matches(
				page,
				/\b(colour\w*|behaviour\w*|honour\w*|favour\w*|centre[sd]?|licence[sd]?|grey|catalogue[sd]?|artefacts?|analys(e|es|ed|ing)|(initiali|seriali|normali|customi|organi|recogni|optimi|minimi|maximi|summari|rasteri|synchroni|virtuali|authori|categori|priori|speciali|standardi|utili|visuali|memori|tokeni|parameteri|generali|locali|finali|materiali|stabili|emphasi|critici|apologi|reali)s(e|es|ed|ing|ation|ations|er|ers))\b/gi,
				(m) => `"${m}"`,
			),
	},
	'fence-language': {
		source: 'Roblox',
		summary: 'every fenced code block names its language (markdownlint MD040)',
		level: 'warn',
		*check(page) {
			for (const f of page.fences) if (!f.lang) yield { line: f.line, message: 'fence without a language' };
		},
	},
	'no-h1-in-body': {
		source: 'Roblox',
		summary: 'the title is the front matter title; the body has no H1 (markdownlint MD025)',
		level: 'warn',
		*check(page) {
			for (const b of page.blocks) if (b.kind === 'h1') yield { line: b.line, message: `"${b.text}"` };
		},
	},
	'no-horizontal-rules': {
		source: 'Roblox',
		summary: 'sections are separated by headings, not by --- rules',
		level: 'warn',
		*check(page) {
			for (const [i, line] of page.lines.entries()) if (i >= page.start && /^\s*(---|\*\*\*|___)\s*$/.test(line)) yield { line: i + 1, message: 'horizontal rule' };
		},
	},
	'lede-is-one-sentence': {
		source: 'Dew',
		summary: 'the front matter description is one sentence of at most 12 words',
		level: 'warn',
		*check(page) {
			const d = page.frontmatter.description;
			if (!d) yield { line: 1, message: 'no description' };
			else if (d.split(/\s+/).length > 12 || /[.!?]\s+[A-Z]/.test(d)) yield { line: 3, message: `${d.split(/\s+/).length} words: "${d}"` };
		},
	},
};

/** The rules that fail the build. Everything else warns. */
const ADOPTED = [
	'intro-before-sections',
	'what-before-why',
	'no-rationale-sections',
	'reader-vocabulary',
	'dew-not-the-host',
	'paragraph-length',
	'sentence-length',
	'heading-length',
	'heading-punctuation',
	'heading-sentence-case',
	'heading-no-numbers',
	'heading-no-questions',
	'no-we',
	'modal-verbs',
	'no-backstory',
	'no-minimisers',
	'bold-for-terms',
	'no-caps-emphasis',
	'no-dashes',
	'ascii-only',
	'no-possessive-code',
	'parentheses',
	'american-spelling',
	'fence-language',
	'no-h1-in-body',
	'no-horizontal-rules',
	'lede-is-one-sentence',
];
for (const id of ADOPTED) {
	const level = RULES[id].level;
	// A rule that applies to some kinds of page only keeps that scope.
	RULES[id].level = typeof level === 'function' ? (page) => (level(page) === 'off' ? 'off' : 'error') : 'error';
}
