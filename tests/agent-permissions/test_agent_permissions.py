#!/usr/bin/env python3
"""Guard the repository's OpenCode V2 custom-agent permission contracts."""

import fnmatch
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
AGENTS = ROOT / ".opencode/agents"
RULE = re.compile(
    r"^\s+- action: (?P<action>\w+)\n"
    r"\s+resource: \"?(?P<resource>[^\"\n]+)\"?\n"
    r"\s+effect: (?P<effect>allow|deny)\s*$",
    re.MULTILINE,
)


def agent(name):
    text = (AGENTS / f"{name}.md").read_text(encoding="utf-8")
    frontmatter = text.split("---", 2)[1]
    keys, rules = parse_agent(name, frontmatter)
    return keys, rules, text


def parse_agent(name, frontmatter):
    top_level_keys = {
        match.group(1)
        for match in re.finditer(r"^([A-Za-z][\w-]*):", frontmatter, re.MULTILINE)
    }
    rules = [match.groupdict() for match in RULE.finditer(frontmatter)]
    legacy = top_level_keys & {"permission", "bash", "task"}
    if legacy:
        raise AssertionError(f"{name} uses legacy agent fields: {', '.join(sorted(legacy))}")
    if "permissions" not in top_level_keys or not rules:
        raise AssertionError(f"{name} is missing V2 permission rules")
    return top_level_keys, rules


def allowed(rules, command):
    """OpenCode uses the last matching shell rule when rules overlap."""
    result = None
    for rule in rules:
        if rule["action"] == "shell" and fnmatch.fnmatchcase(command, rule["resource"]):
            result = rule["effect"] == "allow"
    return result


class AgentPermissionTests(unittest.TestCase):
    def test_custom_agents_use_v2_permission_frontmatter(self):
        files = sorted(AGENTS.glob("*.md"))
        self.assertTrue(files, "no custom agent files found")
        for path in files:
            with self.subTest(agent=path.name):
                keys, rules, _ = agent(path.stem)
                self.assertIn("permissions", keys)
                self.assertTrue(rules, f"{path.name} has no V2 permission rules")

    def test_reviewer_merge_base_exception_does_not_allow_git_merge(self):
        _, rules, _ = agent("awtrix-reviewer")
        self.assertTrue(allowed(rules, "git merge-base --is-ancestor base head"))
        self.assertFalse(allowed(rules, "git merge feature"))

    def test_implementer_can_check_ancestry_but_cannot_merge_push_or_mutate_github(self):
        _, rules, _ = agent("awtrix-implementer")
        self.assertTrue(allowed(rules, "git merge-base --is-ancestor base head"))
        self.assertFalse(allowed(rules, "git merge feature"))
        self.assertFalse(allowed(rules, "git push origin feature"))
        self.assertFalse(allowed(rules, "gh issue edit 47 --add-label ready-for-human"))

    def test_reviewer_uses_dedicated_file_tools_and_restricted_shell(self):
        _, rules, text = agent("awtrix-reviewer")
        for tool in ("glob", "grep", "read"):
            self.assertIn(f"`{tool}`", text)
        allowed_rules = {
            rule["resource"] for rule in rules
            if rule["action"] == "shell" and rule["effect"] == "allow"
        }
        self.assertEqual(allowed_rules, {
            "git status*", "git diff*", "git log*", "git show*", "git rev-parse*",
            "git merge-base*", "gh issue view*", "gh pr view*", "gh run view*",
            "gh api --method GET *", "cargo fmt --check*", "cargo clippy*", "cargo test*",
            "python tests/agent-workflow/test_implementation_gate.py*",
            "python3 tests/agent-workflow/test_implementation_gate.py*",
            "python tests/agent-permissions/test_agent_permissions.py*",
            "python3 tests/agent-permissions/test_agent_permissions.py*",
        })
        self.assertFalse(allowed(rules, "find . -name '*.md'"))
        self.assertFalse(allowed(rules, "cat AGENTS.md"))
        self.assertFalse(allowed(rules, "git push origin feature"))
        self.assertFalse(allowed(rules, "gh issue comment 47 --body update"))

    def test_project_agents_load_with_expected_models(self):
        expected = {
            "awtrix-implementer": "openai/gpt-6-luna",
            "awtrix-reviewer": "openai/gpt-6-luna",
            "awtrix-orchestrator": "openai/gpt-6.1-sol",
        }
        for name, model in expected.items():
            with self.subTest(agent=name):
                keys, _, _ = agent(name)
                self.assertIn("model", keys)
                text = (AGENTS / f"{name}.md").read_text(encoding="utf-8")
                self.assertRegex(text, rf"(?m)^model: {re.escape(model)}$")

    def test_legacy_permission_key_is_rejected(self):
        with self.assertRaisesRegex(AssertionError, "legacy agent fields: permission"):
            parse_agent("fixture", "permission:\n  bash: allow\n")


if __name__ == "__main__":
    unittest.main()
