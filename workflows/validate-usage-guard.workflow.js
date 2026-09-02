export const meta = {
  name: 'validate-usage-guard',
  description: 'Run the production guard() against the live session bar and report what it decided',
  phases: [{ title: 'Guard', detail: 'the real guard block, copied verbatim from cpp-review.workflow.js' }],
}

// ---------------------------------------------------------------- usage guard
//
// One number and one timestamp:
//
//     GET https://claude.ai/api/organizations/{org_uuid}/usage
//     -> { "five_hour": { "utilization": 0-100, "resets_at": "<iso8601>" }, ... }
//
// `five_hour.utilization` IS the session-limit bar the Claude settings page draws. At or above
// PAUSE_AT we stop launching segments and hand back `resets_at` so the caller can arm a resume.
//
// Everything this replaced -- empirical token ceilings, Opus weighting, overage tiers, summing
// message.usage out of ~/.claude/projects -- was scaffolding for not being able to see this
// number. It is all gone on purpose. Do not reintroduce a token estimate: it was wrong twice
// (once by inventing a ceiling, once by reading a stale overage flag out of a log record and
// reporting $0 while real spend was accruing).
//
// TRANSPORT. The endpoint is cookie-authenticated and the session cookie is httpOnly, so the
// probe cannot curl it. It goes through the user's own logged-in Chrome via the claude-in-chrome
// MCP and runs fetch() in the page's context, where the cookies already apply -- no credential is
// ever read, copied, or handled. Verified working from inside a workflow subagent
// (run wf_6cacbc93-4b2: ok=true, utilization=100, 6 tool calls, 23s).
//
// LIMITATION. This needs a live, logged-in Chrome, so it does not work headless or under cron.
// A headless guard would need the sessionKey cookie supplied out of band.

const CFG = (typeof args === 'object' && args) ? args : {}
//: Pause when the session bar is this full. "Within 5%" of the limit.
const PAUSE_AT = CFG.pauseAtUtilization || 95

const PROBE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['ok', 'utilization', 'resets_at_iso', 'method'],
  properties: {
    ok: { type: 'boolean', description: 'true only on a real 200 from the live endpoint' },
    utilization: { type: 'number', description: 'five_hour.utilization 0-100; -1 when unknown' },
    resets_at_iso: { type: 'string', description: 'five_hour.resets_at verbatim, or ""' },
    method: { type: 'string', description: 'how it was obtained, naming any step that failed' },
  },
}

function probePrompt(where) {
  return [
    'Read the LIVE Claude session-limit utilization. It must come from the live claude.ai API —',
    'never from a local log file, and never from your own recollection. Do not read, print, or',
    'copy any credential; the browser holds the session and that is where the request belongs.',
    '',
    'METHOD, exactly:',
    '1. Load the browser tools in ONE call:',
    '   ToolSearch({query: "select:mcp__claude-in-chrome__tabs_context_mcp,mcp__claude-in-chrome__tabs_create_mcp,mcp__claude-in-chrome__navigate,mcp__claude-in-chrome__javascript_tool,mcp__claude-in-chrome__tabs_close_mcp", max_results: 5})',
    '2. tabs_context_mcp({createIfEmpty: true}) for a tabId.',
    '3. navigate that tab to https://claude.ai/new — the app must load for cookies to apply.',
    '4. javascript_tool on that tab, this exact code:',
    '',
    '     const o = await (await fetch("/api/organizations", {credentials:"include"})).json();',
    '     const u = await (await fetch(`/api/organizations/${o[0].uuid}/usage`, {credentials:"include"})).json();',
    '     JSON.stringify({utilization: u.five_hour.utilization, resets_at: u.five_hour.resets_at})',
    '',
    '5. tabs_close_mcp the tab you created. Always, even on failure.',
    '',
    'Return five_hour.utilization and five_hour.resets_at verbatim.',
    '',
    'If the page redirects to /login the browser is not authenticated: return ok=false,',
    'utilization=-1, resets_at_iso="", and say so in `method`. Same for any other failure.',
    'NEVER invent a number, and never report a high utilization you did not read — a failed',
    'measurement must not halt work.',
    '',
    'Checkpoint: ' + where,
  ].join('\n')
}

let paused = false
let pauseInfo = null

async function guard(where) {
  if (paused) return true
  const u = await agent(probePrompt(where), {
    label: 'usage-probe:' + where, phase: 'Guard', schema: PROBE_SCHEMA, model: 'sonnet', effort: 'low',
  })
  if (!u || !u.ok || typeof u.utilization !== 'number' || u.utilization < 0) {
    log('Usage probe could not read the session bar at "' + where + '" ('
      + (u ? u.method : 'agent returned nothing') + ') — PROCEEDING. A failed measurement is not evidence of danger.')
    return false
  }
  log('Session limit at "' + where + '": ' + u.utilization + '% used'
    + (u.resets_at_iso ? ', resets ' + u.resets_at_iso : '') + '.')
  if (u.utilization < PAUSE_AT) return false
  paused = true
  pauseInfo = { at: where, utilization: u.utilization, resets_at_iso: u.resets_at_iso, threshold: PAUSE_AT }
  log('PAUSING: session limit ' + u.utilization + '% >= ' + PAUSE_AT + '%. Segments after "' + where
    + '" were not launched. Resume after ' + (u.resets_at_iso || 'the window resets')
    + ' with the same scriptPath + resumeFromRunId; completed agents replay from cache.')
  return true
}


// ---------------------------------------------------------------- run
//
// Drives the guard block above exactly as the review workflow does. Nothing here is a paraphrase:
// everything from the "usage guard" banner down to this line was copied byte-for-byte out of
// workflows/cpp-review.workflow.js, so a pass here is a pass for the real thing.

phase('Guard')

log('Calling the production guard() against the live session-limit bar.')
const first = await guard('validate:first')

// Second call must short-circuit on the sticky `paused` flag WITHOUT spawning another probe.
// If this spawns a second usage-probe agent, the stickiness is broken and a paused run would
// keep paying for probes it has already decided to skip.
const before = paused
const second = await guard('validate:second')

return {
  threshold_pct: PAUSE_AT,
  first_call_returned_paused: first,
  second_call_returned_paused: second,
  sticky_flag_was_already_set: before,
  pause_info: pauseInfo,
  read_a_real_number: pauseInfo ? typeof pauseInfo.utilization === 'number' && pauseInfo.utilization >= 0 : false,
  got_reset_timestamp: pauseInfo ? Boolean(pauseInfo.resets_at_iso) : false,
}
