export const meta = {
  name: 'finish-research-then-review',
  description: 'Finish the three unrun research topics, then adversarially review DECISION.md and the plan',
  phases: [
    { title: 'Guard', detail: 'live session-limit probe', model: 'sonnet' },
    { title: 'Research', detail: 'the three topics that never ran', model: 'sonnet' },
    { title: 'Attack', detail: 'evidence / reasoning / completeness', model: 'sonnet' },
    { title: 'Adjudicate', detail: 'merge, rule, and name the plan revisions', model: 'opus' },
  ],
}

const ROOT = '/Users/mgb/RustRoverProjects/compylr'
const PAUSE_AT = 95

const NOSEARCH = [
  'WebSearch is EXHAUSTED for this session (200/200) and will fail. Use WebFetch against known URLs.',
  'Load it with: ToolSearch({query: "select:WebFetch", max_results: 1})',
  'cppreference.com returns 403 to WebFetch; do not waste calls on it.',
  'A local git submodule at ' + ROOT + '/inspiration/py2many is readable source for a transpiler with',
  'thirteen backends -- read it directly rather than fetching anything about it.',
].join('\n')

// ---- guard: same fail-closed shape as cpp-review.workflow.js -------------------------------
const PROBE_SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['ok', 'utilization', 'resets_at_iso', 'method'],
  properties: {
    ok: { type: 'boolean' }, utilization: { type: 'number' },
    resets_at_iso: { type: 'string' }, method: { type: 'string' },
  },
}
function probePrompt(where) {
  return [
    'Read the LIVE Claude session-limit utilization from the claude.ai API. Never from a local log.',
    'Do not read, print, or copy any credential.',
    '1. ToolSearch({query: "select:mcp__claude-in-chrome__tabs_context_mcp,mcp__claude-in-chrome__tabs_create_mcp,mcp__claude-in-chrome__navigate,mcp__claude-in-chrome__javascript_tool,mcp__claude-in-chrome__tabs_close_mcp", max_results: 5})',
    '2. tabs_context_mcp({createIfEmpty:true}); 3. navigate to https://claude.ai/new',
    '4. javascript_tool with exactly:',
    '   const o = await (await fetch("/api/organizations",{credentials:"include"})).json();',
    '   const u = await (await fetch(`/api/organizations/${o[0].uuid}/usage`,{credentials:"include"})).json();',
    '   JSON.stringify({utilization:u.five_hour.utilization, resets_at:u.five_hour.resets_at})',
    '5. tabs_close_mcp the tab, always.',
    'On any failure return ok=false, utilization=-1. Never invent a number. Checkpoint: ' + where,
  ].join('\n')
}
let paused = false, pauseInfo = null
async function guard(where) {
  if (paused) return true
  let r = await agent(probePrompt(where), { label: 'usage-probe:' + where, phase: 'Guard', schema: PROBE_SCHEMA, model: 'sonnet', effort: 'low' })
  if (!r || !r.ok || r.utilization < 0) {
    log('Probe unreadable at "' + where + '" — retrying once.')
    r = await agent(probePrompt(where + ':retry'), { label: 'usage-probe:' + where + ':retry', phase: 'Guard', schema: PROBE_SCHEMA, model: 'sonnet', effort: 'low' })
  }
  if (!r || !r.ok || r.utilization < 0) {
    paused = true
    pauseInfo = { at: where, reason: 'probe-unreadable', utilization: -1, resets_at_iso: '' }
    log('PAUSING: probe unreadable twice at "' + where + '". Fail closed — the probe is an agent, so the likeliest cause is that we are already over the limit.')
    return true
  }
  log('Session limit at "' + where + '": ' + r.utilization + '% used, resets ' + r.resets_at_iso)
  if (r.utilization < PAUSE_AT) return false
  paused = true
  pauseInfo = { at: where, reason: 'over-threshold', utilization: r.utilization, resets_at_iso: r.resets_at_iso }
  log('PAUSING: ' + r.utilization + '% >= ' + PAUSE_AT + '%.')
  return true
}
function checkSegmentHealth(where, results) {
  const dead = results.filter(x => !x).length
  if (dead) log('Segment "' + where + '": ' + dead + '/' + results.length + ' agents returned nothing.')
  if (results.length && dead / results.length >= 0.3) {
    paused = true
    pauseInfo = { at: where, reason: 'segment-failure-rate', utilization: -1, resets_at_iso: '' }
    log('PAUSING: ' + dead + '/' + results.length + ' of "' + where + '" died — systemic.')
    return true
  }
  return false
}


// ---- research: the three topics that never ran ---------------------------------------------

const RESEARCH_SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['topic', 'summary', 'implications_for_compylr'],
  properties: {
    topic: { type: 'string' },
    summary: { type: 'string' },
    key_facts: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false,
        required: ['fact'],
        properties: {
          fact: { type: 'string' },
          source: { type: 'string' },
          confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
        },
      },
    },
    implications_for_compylr: { type: 'string' },
    changes_a_decision: { type: 'boolean', description: 'true if this contradicts something already decided' },
    what_it_changes: { type: 'string' },
    context_file: { type: 'string' },
  },
}

const TOPICS = [
  { key: 'python-native-compilers', q: [
      'Projects compiling Python to native code: Nuitka, Cython, mypyc, Codon, Numba, Pythran, and',
      'the CPython 3.13+ JIT. For each: what subset of Python it accepts, and CRITICALLY how it handles',
      'integer semantics -- Python ints are arbitrary precision, machine ints are not; `//` floors while',
      'C truncates; overflow is impossible in Python and wraps in C. Does each promote to bignum, trap,',
      'wrap, or restrict the subset? And how does each cross back into CPython?',
      'Useful URLs: nuitka.net, cython.readthedocs.io/en/latest/src/userguide/language_basics.html,',
      'mypyc.readthedocs.io/en/latest/int_operations.html, docs.exaloop.io/codon/general/differences,',
      'numba.readthedocs.io/en/stable/reference/pysemantics.html',
      'compylr made this choice explicitly: int is i64 and overflow is a behavior axis. Did anyone else',
      'reach the same answer, and what did the ones who chose differently pay for it?',
    ].join(' ') },
  { key: 'multi-target-transpilers', q: [
      'Source-to-source compilers with MULTIPLE target languages from one IR. The best evidence is',
      'LOCAL: read ' + ROOT + '/inspiration/py2many, which has thirteen backends. Read its actual source --',
      'how is a backend structured, what does the IR look like, what does adding a target cost, and how',
      'does it handle a construct one target has and another does not? Read LANGUAGES.md and the per-',
      'backend directories.',
      'Then compare against Haxe (haxe.org/manual/introduction.html) and Nim as far as WebFetch allows.',
      'The question for compylr: is its frontend/IR/backend/bridge split unusual, and where do these',
      'projects break down as targets multiply?',
    ].join(' ') },
  { key: 'semantics-mismatch', q: [
      'How do transpilers handle semantic mismatch between source and target -- integer division',
      'rounding, remainder sign, overflow, negative indexing, string length units? Read',
      ROOT + '/inspiration/py2many source for how it handles (or ignores) these across its backends.',
      'Is there prior art for making these choices EXPLICIT and CONFIGURABLE PER OPERATION, which is',
      'what compylr calls behavior axes? Or does everyone else silently pick one and document it?',
      'Find concrete examples of projects that got this wrong and what the bug reports looked like.',
      'This is the question of whether compylr\'s central design idea is novel or a reinvention.',
    ].join(' ') },
]

function researchPrompt(t) {
  return [
    'REPO: ' + ROOT + '. READ-ONLY on project code. Write durable output under research/ (tracked); throwaway probes under context/.',
    NOSEARCH,
    '',
    '=== RESEARCH: ' + t.key + ' ===',
    '',
    t.q,
    '',
    'Prefer primary sources and local source code over recollection; your training data may be stale.',
    'Where you cannot establish something, say so rather than estimating. Mark confidence honestly.',
    '',
    'Set `changes_a_decision` true ONLY if you found something that contradicts a decision already',
    'recorded in research/DECISION.md or openspec/changes/add-cpp-backend/design.md -- read both first',
    'so you can tell. That flag is the point of this leg; a topic that changes nothing should say so.',
    '',
    'Write the full writeup to research/' + t.key + '.md and return its path.',
  ].join('\n')
}

// ---- review -------------------------------------------------------------------------------

const REVIEW_SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['lens', 'verdict', 'problems'],
  properties: {
    lens: { type: 'string' },
    verdict: { type: 'string', enum: ['sound', 'sound-with-corrections', 'materially-flawed'] },
    problems: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false,
        required: ['claim_attacked', 'severity', 'finding', 'evidence'],
        properties: {
          claim_attacked: { type: 'string', description: 'quote the sentence attacked' },
          severity: { type: 'string', enum: ['fatal', 'major', 'minor'] },
          finding: { type: 'string' },
          evidence: { type: 'string' },
          correction: { type: 'string' },
        },
      },
    },
    what_holds: { type: 'string', description: 'load-bearing claims you tried to break and could not' },
  },
}

const LENSES = [
  { key: 'evidence', q: [
      'Attack the EVIDENCE. Check that each cited source says what is claimed, in this order:',
      '1. The ~145x ctypes-vs-PyO3 figure from arXiv:2507.00264 -- read research/python-call-overhead.md',
      '   and verify the number, the M1/M2 regime, and that the paper measures what the doc says it does.',
      '2. The C++26 matrix -- re-fetch gcc.gnu.org/projects/cxx-status.html and clang.llvm.org/cxx_status.html.',
      '3. node:ffi v26.1.0 -- re-fetch nodejs.org/api/ffi.html.',
      '4. The nanobind multipliers -- re-fetch nanobind.readthedocs.io/en/latest/benchmark.html.',
      'A number whose source does not state it is FATAL. Also check the three research files just',
      'written by this run.',
    ].join(' ') },
  { key: 'reasoning', q: [
      'Attack the REASONING. Granting every fact, do the conclusions follow?',
      'The sharpest known gap, which DECISION.md admits itself: the 145x figure is ctypes-vs-PyO3, yet',
      'it is used to reject a hub, and the relevant comparison -- nanobind-vs-PyO3 -- is UNMEASURED.',
      'Is that inference sound or is it motivated reasoning?',
      'Then: does "C++ is the target that needs a hub least" follow, or rationalise a decision made on',
      'other grounds? Is "#42 is the root" established or asserted? Is the separate-change split',
      'defensible? Read openspec/changes/add-cpp-backend/ and name every place it and DECISION.md disagree.',
    ].join(' ') },
  { key: 'completeness', q: [
      'Attack the COMPLETENESS.',
      '- Which crates did the audit never open? Enumerate crates/ against the coverage_notes in',
      '  research/audit-findings.json and name the blind spots.',
      '- Which design decisions rest on NO evidence? D2, D5, D6, D7, D8 were never researched. Does',
      '  any of them need to be, or are they safe on reasoning alone?',
      '- Read the three research files this run just produced. Do they change anything the plan assumes?',
      '- What defect class did nobody hunt for? The audit looked only for "claims that are not true".',
      '- What would a compiler engineer or a Python/C++ interop specialist say is obviously absent?',
    ].join(' ') },
]

function reviewPrompt(l, research) {
  return [
    'REPO: ' + ROOT + '. READ-ONLY on project code. Write durable output under research/ (tracked); throwaway probes under context/.',
    NOSEARCH,
    '',
    '=== ADVERSARIAL REVIEW of research/DECISION.md, lens "' + l.key + '" ===',
    '',
    'Read research/DECISION.md in full first. It is a synthesis nobody has reviewed, written by the',
    'same agent that made the decisions it defends -- which is why you are here.',
    '',
    l.q,
    '',
    'Quote the exact sentence attacked in `claim_attacked`. Report only problems you actually',
    'established; an unverified suspicion is not a finding. Use `what_holds` for load-bearing claims',
    'you genuinely tried to break and could not -- that is as useful as the breaks.',
    'Write your working to research/review-' + l.key + '.md.',
    '',
    'RESEARCH COMPLETED EARLIER IN THIS RUN (factor it in):',
    JSON.stringify(research, null, 1),
  ].join('\n')
}

// ---- run ----------------------------------------------------------------------------------

log('Three research topics, then three adversarial lenses on DECISION.md, then adjudication.')

let research = []
if (!await guard('start')) {
  research = (await parallel(TOPICS.map(t => () =>
    agent(researchPrompt(t), { label: 'research:' + t.key, phase: 'Research', schema: RESEARCH_SCHEMA, model: 'sonnet', effort: 'high' })
  ))).filter(Boolean)
  checkSegmentHealth('research', research)
  const changed = research.filter(r => r.changes_a_decision)
  log('Research: ' + research.length + '/3 done. ' + changed.length + ' contradict an existing decision'
    + (changed.length ? ': ' + changed.map(r => r.topic).join(', ') : '.'))
}

let reviews = []
if (!paused && !await guard('after-research')) {
  reviews = (await parallel(LENSES.map(l => () =>
    agent(reviewPrompt(l, research), { label: 'attack:' + l.key, phase: 'Attack', schema: REVIEW_SCHEMA, model: 'sonnet', effort: 'high' })
  ))).filter(Boolean)
  checkSegmentHealth('attack', reviews)
}

let ruling = null
if (reviews.length && !await guard('before-adjudication')) {
  phase('Adjudicate')
  ruling = await agent([
    'REPO: ' + ROOT + '. READ-ONLY on project code; you may write research/REVIEW.md.',
    '',
    '=== ADJUDICATE ===',
    '',
    'Three reviewers attacked research/DECISION.md through evidence, reasoning and completeness lenses,',
    'with three new research topics in hand. Rule on what they found. Read DECISION.md and',
    'openspec/changes/add-cpp-backend/ yourself -- do not take the reviewers on faith, and say where',
    'you disagree with one.',
    '',
    'Produce research/REVIEW.md with:',
    '1. Claims in DECISION.md that are BROKEN and must be retracted or narrowed. Quote and correct.',
    '2. Claims that survived a genuine attempt to break them, naming what was tried.',
    '3. **A concrete revision list for openspec/changes/add-cpp-backend/ -- per artifact, per',
    '   requirement, specific enough to act on without re-deriving it.** This is the most important',
    '   section: the plan artifacts must end up capturing everything learned, including the research.',
    '4. What still has no evidence, and the cheapest experiment for each.',
    '5. A one-line verdict on whether the plan is safe to implement as written.',
    '',
    'Where reviewers disagree, adjudicate rather than average. Return the full text.',
    '',
    'RESEARCH:', JSON.stringify(research, null, 1),
    '', 'REVIEWS:', JSON.stringify(reviews, null, 1),
  ].join('\n'), { label: 'adjudicate', phase: 'Adjudicate', model: 'opus', effort: 'xhigh' })
}

const fatal = reviews.flatMap(r => r.problems || []).filter(p => p.severity === 'fatal')
log('Done: ' + research.length + '/3 research, ' + reviews.length + '/3 lenses, ' + fatal.length + ' fatal problems.')

return {
  paused, pause_info: pauseInfo,
  research: research.map(r => ({ topic: r.topic, changes_a_decision: r.changes_a_decision,
                                 what_it_changes: r.what_it_changes, file: r.context_file,
                                 implications: r.implications_for_compylr })),
  decisions_contradicted: research.filter(r => r.changes_a_decision).map(r => r.topic),
  verdicts: reviews.map(r => ({ lens: r.lens, verdict: r.verdict, problems: (r.problems || []).length })),
  fatal_problems: fatal,
  all_problems: reviews.flatMap(r => r.problems || []),
  what_holds: reviews.map(r => ({ lens: r.lens, holds: r.what_holds })),
  ruling,
}
