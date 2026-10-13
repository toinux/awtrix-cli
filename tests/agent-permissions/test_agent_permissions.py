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
    top_level_keys = {
        match.group(1)
        for match in re.finditer(r"^([A-Za-z][\w-]*):", frontmatter, re.MULTILINE)
    }
    rules = [match.groupdict() for match in RULE.finditer(frontmatter)]
    return text, top_level_keys, rules


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
                text, keys, rules = agent(path.stem)
                self.assertNotIn("permission", keys, f"{path.name} uses legacy permission:")
                self.assertNotIn("bash", keys, f"{path.name} uses legacy bash:")
                self.assertNotIn("task", keys, f"{path.name} uses legacy task:")
                self.assertIn("permissions", keys)
                self.assertTrue(rules, f"{path.name} has no V2 permission rules")

    def test_reviewer_merge_base_exception_does_not_allow_git_merge(self):
        _, _, rules = agent("awtrix-reviewer")
        self.assertTrue(allowed(rules, "git merge-base --is-ancestor base head"))
        self.assertFalse(allowed(rules, "git merge feature"))

    def test_implementer_can_check_ancestry_but_cannot_merge_push_or_mutate_github(self):
        _, _, rules = agent("awtrix-implementer")
        self.assertTrue(allowed(rules, "git merge-base --is-ancestor base head"))
        self.assertFalse(allowed(rules, "git merge feature"))
        self.assertFalse(allowed(rules, "git push origin feature"))
        self.assertFalse(allowed(rules, "gh issue edit 47 --add-label ready-for-human"))

    def test_reviewer_uses_dedicated_file_tools_and_restricted_shell(self):
        text, _, rules = agent("awtrix-reviewer")
        for tool in ("glob", "grep", "read"):
            self.assertIn(f"`{tool}`", text)
        self.assertFalse(allowed(rules, "find . -name '*.md'"))
        self.assertFalse(allowed(rules, "cat AGENTS.md"))
        self.assertFalse(allowed(rules, "gh issue comment 47 --body update"))

    def test_project_agents_load_with_expected_models(self):
        expected = {
            "awtrix-implementer": "openai/gpt-6-luna",
            "awtrix-reviewer": "openai/gpt-6-luna",
            "awtrix-orchestrator": "openai/gpt-6.1-sol",
        }
        for name, model in expected.items():
            with self.subTest(agent=name):
                _, keys, _ = agent(name)
                self.assertIn("model", keys)
                text = (AGENTS / f"{name}.md").read_text(encoding="utf-8")
                self.assertRegex(text, rf"(?m)^model: {re.escape(model)}$")


if __name__ == "__main__":
    unittest.main()
