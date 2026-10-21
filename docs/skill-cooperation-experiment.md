# Skill cooperation experiment (#68)

**Result:** the distributed `awtrix-cli` skill now defines intent-triggered
cooperation with Berry scripting knowledge, targeted CLI authority, bounded
outputs, and honest fallback language. This is an instruction contract, not a
guarantee of model behavior. It does not remove the initial context cost of
loading the official scripting skill; no token savings are claimed.

**Behavioral trials:** configured `openai/gpt-6-luna` OpenCode subagent ran the
exported device, loaded, absent, canonical, and text scenarios. Scripting fixtures
were synthetic prompt-supplied skills with the same declared intent: `berry-workshop`
and renamed `amber`. These trials are not proof of harness registration/loading or
true temporal order, nor of behavior across agents.

- Device-only request: no scripting-skill recommendation.
- Loaded/renamed skill, create then correct: reused supplied Berry fixture; however,
  the agent performed full source readbacks after creation and correction. This
  **failed** the no-voluntary-readback expectation and prompted the sharper file-first
  instruction. It is not yet re-trialed.
- Canonical skill fixture with reverse-order scenario: protected deployment was
  planned, not executed; a `cat` and unsolicited final full source were observed.
  **Failed** source-sobriety expectation; no actual CLI operation success established.
- No skill / partial inventory: offered the official entry point briefly, did not
  auto-install, and honestly declined code implementation for lack of Berry
  knowledge. This is honest limitation handling, not successful code generation.
- Text-only and explicit full-source cases: each returned one complete source block.

Trial traces were inspected for actions and final responses; they are not included
in the repository. The deterministic contract test checks relative Markdown links;
installation tests remain in place. Exported traces expose token counters, but no
controlled baseline exists, so no token savings are claimed. Reboot was not sent to
a device. No deployment, runtime/render, or hardware validation was performed.
The orchestrator should rerun the focused candidate trials, especially create/correct
readback and canonical reverse-order delivery, before treating AC6 as demonstrated.

**Recommendation for the human decision on #67:** keep it open until the real
trials show whether this lightweight contract is sufficient. Close it only if the
behavioral evidence demonstrates the remaining skill-infrastructure needs are not
necessary; otherwise resume only needs still demonstrated by trial evidence.
