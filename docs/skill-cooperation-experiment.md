# Skill cooperation experiment (#68)

**Result:** the distributed `awtrix-cli` skill now defines intent-triggered
cooperation with Berry scripting knowledge, targeted CLI authority, bounded
outputs, and honest fallback language. This is an instruction contract, not a
guarantee of model behavior. It does not remove the initial context cost of
loading the official scripting skill; no token savings are claimed.

**Behavioral trials:** OpenCode v2.1.2, configured `openai/gpt-6-luna`, used the
normal skill-loading tool with the candidate instructions at `7028154`. The user
provided the official scripting skill and authorized temporary symlink registration;
the skill content was not modified. The registered identifiers were
`awtrix-berry-app` and then `amber`, with identical description/content; the alias
kept its original frontmatter name. This exercised actual harness loading, but not
a rename of official content or a general guarantee across agents.

- **Device-only:** loaded the CLI skill only. Proposed finite 10-second logs and
  diagnosis commands; no Berry discovery/recommendation and no device calls.
- **Canonical / CLI then Berry:** read the full Berry API reference (60,421 bytes,
  1,180 lines; initial tool response was continued). Created and corrected
  `bonjour.ax`, updating the visible name and description and the internal class and
  return as well as the displayed text. Used targeted grep after writing; no full
  source readback or block was returned. The final answer nevertheless suggested
  web-UI paste steps rather than CLI handoff despite the accessible file. This
  handoff drift **persisted** under the stronger wording and this load order. The
  concurrency-safe update plan in an earlier turn named exact original source,
  `--expected-source`, and `scriptUpdates`, with no force; it was not executed. No
  compile/runtime/render check was claimed.
- **Renamed / Berry then CLI:** an initial implementation-agent ancestry check
  interrupted the scratch-only evaluation; after clarification that this was not a
  source dispatch, the trial continued with the already-loaded skills. Created and
  corrected `test.ax` to `Salut`, updating its visible name/description and internal
  class/return; used targeted metadata and occurrence checks, without full source
  redisplay. The final response gave a CLI `script deploy Salut --file ... --create`
  plan, not a web-UI paste instruction. No command or deployment ran. The earlier
  display-only `Test` class concern is not evidence of a bug: that task requested
  changing only displayed text. The explicit full-source and text-only trials were
  from the prior workflow run, not repeated in v2.
- **Earlier synthetic trials:** retain as negative evidence, not current-harness
  results. They exposed full readbacks after correction and `cat` plus unsolicited
  final source in reverse-order scenarios; those failures motivated the tightened
  wording. Neither focused v2 trial produced a post-write full-source redisplay.
  However, canonical CLI handoff drift persists while the renamed/reverse-order trial
  used the requested CLI plan. This mixed outcome does not establish uniform behavior
  or warrant an all-criteria acceptance claim.
- **Absent/partial inventory:** the earlier synthetic trial offered the official
  entry point once and honestly declined code work. In the completed real trial,
  the CLI skill was normally loaded with no Berry skill exposed. For a code request,
  it stated that it could not write reliable Berry, linked the official entry point
  once, and created no file, project, or device request. In the partial-inventory
  case after the user refused installation, it kept CLI operations available without
  repeating the offer or claiming disk-wide absence. No scan, install, or download
  occurred, and it made no Berry validation claim. [Bounded trial evidence](https://github.com/toinux/awtrix-cli/issues/68#issuecomment-6329135073).

The traces are not committed. All temporary scripting-skill symlinks were removed;
the original external skill and API checksums were unchanged. No device/runtime/render
checks were performed. Trace token counters exist, but there is no controlled
baseline, so no savings are claimed. Initial API-context cost remains (the reference
is 60,421 bytes / 1,180 lines).

**Recommendation on #67:** keep it open and unchanged; ask the human whether to
resume it only for the demonstrated residual full API/context loading cost and the
conflicting no-device handoff instructions/load-order drift: canonical still
suggested web-UI paste guidance, while the renamed/reverse-order case proposed CLI
deployment. The trials do not demonstrate a need for new search or generator
infrastructure. The human decision remains distinct; this is not a guarantee of
behavior beyond these trials.
