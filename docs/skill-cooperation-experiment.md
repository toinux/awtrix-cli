# Skill cooperation experiment (#68)

**Result:** the distributed `awtrix-cli` skill now defines intent-triggered
cooperation with Berry scripting knowledge, targeted CLI authority, bounded
outputs, and honest fallback language. This is an instruction contract, not a
guarantee of model behavior. It does not remove the initial context cost of
loading the official scripting skill; no token savings are claimed.

**Behavioral trials:** OpenCode v2.1.2, configured `openai/gpt-6-luna`, used the
normal skill-loading tool with the candidate instructions at `0dfbb51`. The user
provided the official scripting skill and authorized temporary symlink registration;
the skill content was not modified. The registered identifiers were
`awtrix-berry-app` and then `amber`, with identical description/content; the alias
kept its original frontmatter name. This exercised actual harness loading, but not
a rename of official content or a general guarantee across agents.

- **Device-only:** loaded the CLI skill only. Proposed finite 10-second logs and
  diagnosis commands; no Berry discovery/recommendation and no device calls.
- **Canonical / CLI then Berry:** read the full Berry API reference (60,421 bytes,
  1,180 lines; initial tool response was continued). Created `bonjour.ax` (214 bytes),
  checked metadata without a fresh readback, then on correction read source to edit,
  patched it, and used targeted grep/count and byte metadata. No post-edit full source
  display. The user-facing answer gave a path but also offered web-UI paste steps
  despite the accessible file; this is residual handoff drift. The concurrency-safe
  update plan named exact original source, `--expected-source`, and `scriptUpdates`,
  with no force; it was not executed. A text-only scenario returned one complete
  source block and proposed `script deploy texte --file ./texte.ax --create
  --verify-secs 10`; no command ran. No compile/runtime/render check was claimed.
- **Renamed / Berry then CLI:** created and corrected a local `.ax` from `Test` to
  `Salut`; targeted occurrence counts and file metadata, no full readback or final
  source block. Shell schema errors occurred before a corrected read-tool call. A
  later explicit full-source request produced one justified full read and one block.
  No device request or deployment occurred.
- **Earlier synthetic trials:** retain as negative evidence, not current-harness
  results. They exposed full readbacks after correction and `cat` plus unsolicited
  final source in reverse-order scenarios; those failures motivated the tightened
  wording. Current sampled real trials did not reproduce those particular failures.
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
resume it only for the demonstrated residual initial API/context cost. Sampled real
trials support lightweight instructions for device-vs-code routing, protected CLI
planning, and file-first handoff, while showing one residual unsolicited paste-step
drift. They do not demonstrate a need for a generator, search infrastructure, or
other #67 machinery. The human decision remains distinct; this is not a guarantee
of behavior beyond these trials.
