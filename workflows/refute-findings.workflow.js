export const meta = {
  name: 'refute-audit-findings',
  description: 'Adversarially refute the 24 unverified compylr audit findings, one agent per dimension',
  phases: [
    { title: 'Guard', detail: 'live session-limit probe', model: 'sonnet' },
    { title: 'Refute', detail: 'one skeptic per audit dimension, three lenses each', model: 'sonnet' },
  ],
}

const ROOT = '/Users/mgb/RustRoverProjects/compylr'
const FINDINGS = ROOT + '/research/audit-findings.json'
const PAUSE_AT = 95

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

// ---- refutation -----------------------------------------------------------------------------
const VERDICT_SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['dimension', 'verdicts'],
  properties: {
    dimension: { type: 'string' },
    verdicts: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false,
        required: ['title', 'correctness_refuted', 'intent_refuted', 'materiality_refuted', 'final_refuted', 'reasoning'],
        properties: {
          title: { type: 'string', description: 'must match the finding title verbatim' },
          correctness_refuted: { type: 'boolean', description: 'is the technical claim wrong or the cited evidence not what it says?' },
          intent_refuted: { type: 'boolean', description: 'is this documented deliberate design rather than a defect?' },
          materiality_refuted: { type: 'boolean', description: 'is the path unreachable or the contradicted claim not actually made?' },
          final_refuted: { type: 'boolean', description: 'true when 2+ lenses refuted, or when correctness alone is decisively refuted' },
          reasoning: { type: 'string' },
          corrected_claim: { type: 'string', description: 'the accurate narrower version when partly right; else empty' },
        },
      },
    },
  },
}

const DIMENSIONS = ['ts-frontend', 'ts-go-bridge', 'demo-integrity', 'enforcement-tests', 'python-rust-path', 'spec-vs-reality', 'generated-docs']

function refutePrompt(dim) {
  return [
    'REPO: ' + ROOT,
    'READ-ONLY on project code. You may run read-only commands for evidence — `cargo run -q -p',
    'compylr-cli -- ...`, grep, sed, `go build` in a scratch dir. Write nothing under crates/,',
    'frontends/, demo/, openspec/, scripts/. Durable writeups go in research/ (tracked); throwaway probes in context/ (gitignored).',
    '',
    '=== YOUR ASSIGNMENT: adversarially REFUTE the "' + dim + '" findings ===',
    '',
    'Read ' + FINDINGS + ' and take the "' + dim + '" array. Those findings came from an audit agent',
    'and NOBODY HAS CHECKED THEM. Your job is to kill them.',
    '',
    'Apply THREE independent lenses to each finding and record each separately:',
    '',
    ' • correctness — Is the technical claim actually true? Open the cited files yourself and check',
    '   the line numbers. Re-run any command the finding claims to have run. A finding whose cited',
    '   evidence does not say what it claims is REFUTED on this lens.',
    ' • intent — Is this documented, deliberate design rather than a defect? Check CLAUDE.md, module',
    '   doc comments, openspec/specs/, and openspec/changes/archive/. compylr deliberately has several',
    '   behaviours that look like bugs until you know why — a stricter-than-Python rule, a mode that',
    '   is not a mode, an unimplemented option declared on purpose. Those are NOT defects.',
    ' • materiality — Even if true, does it matter? Is the code path reachable from the accepted',
    '   subset? Is the contradicted claim actually made anywhere load-bearing?',
    '',
    'final_refuted = true when 2 or more lenses refute, OR when correctness alone is decisively wrong.',
    '',
    'Default to refuting when genuinely uncertain. Set final_refuted=false only when you have',
    'INDEPENDENTLY confirmed the finding is real, accurate, and matters. If a finding is partly right,',
    'set final_refuted=false and put the accurate narrower version in corrected_claim.',
    '',
    'Return one verdict per finding, `title` matching VERBATIM. Do not add or drop findings.',
  ].join('\n')
}

// ---- run ------------------------------------------------------------------------------------
log('Refuting 24 unverified findings across 7 dimensions. One skeptic per dimension, three lenses each.')

const results = []
if (!await guard('start')) {
  // Two chunks so there is a checkpoint partway, rather than one 7-wide fan-out with no exit.
  const first = DIMENSIONS.slice(0, 4), second = DIMENSIONS.slice(4)
  const a = await parallel(first.map(d => () =>
    agent(refutePrompt(d), { label: 'refute:' + d, phase: 'Refute', schema: VERDICT_SCHEMA, model: 'sonnet', effort: 'high' })))
  results.push(...a)
  if (!checkSegmentHealth('refute-chunk-1', a) && !await guard('midpoint')) {
    const b = await parallel(second.map(d => () =>
      agent(refutePrompt(d), { label: 'refute:' + d, phase: 'Refute', schema: VERDICT_SCHEMA, model: 'sonnet', effort: 'high' })))
    results.push(...b)
    checkSegmentHealth('refute-chunk-2', b)
  }
}

const byDim = {}
let refuted = 0, confirmed = 0
for (const r of results.filter(Boolean)) {
  byDim[r.dimension] = r.verdicts
  for (const v of r.verdicts) { if (v.final_refuted) refuted++; else confirmed++ }
}
const covered = Object.keys(byDim)
const missing = DIMENSIONS.filter(d => !covered.includes(d))
log('Refutation: ' + confirmed + ' findings CONFIRMED, ' + refuted + ' refuted. Dimensions not reached: ' + (missing.join(', ') || 'none'))

return {
  paused, pause_info: pauseInfo,
  dimensions_covered: covered,
  dimensions_not_reached: missing,
  confirmed_count: confirmed,
  refuted_count: refuted,
  verdicts_by_dimension: byDim,
}
