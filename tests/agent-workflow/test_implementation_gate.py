#!/usr/bin/env python3
"""Exercise publication policy against real Git history and GitHub-shaped responses."""

import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch
from typing import Any


sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / ".opencode/scripts/implementation_gate.py"
SPEC = importlib.util.spec_from_file_location("implementation_gate", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


def reviews(base, head):
    return [{"axis": axis, "base": base, "head": head, "approved": True, "evidence": f"{axis} report"}
            for axis in ("Standards", "Spec")]


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.env = dict(os.environ, GIT_AUTHOR_NAME="Workflow test", GIT_AUTHOR_EMAIL="test@example.invalid",
                        GIT_COMMITTER_NAME="Workflow test", GIT_COMMITTER_EMAIL="test@example.invalid",
                        GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        # Ignore caller Git environment so CI/local worktree settings cannot redirect fixtures.
        for key in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"):
            self.env.pop(key, None)
        self.git("init", "-b", "main")
        self.base = self.commit("baseline")
        self.git("update-ref", "refs/remotes/origin/main", self.base)
        self.git("checkout", "-b", "integration/spec-34")
        self.head = self.commit("ticket implementation")
        self.ticket: dict[str, Any] = {
            "number": 35, "in_scope": True, "status": "integrated", "dependencies": [],
            "issue_state": "OPEN", "dispatch_base": self.base,
            "candidate_head": self.head, "integrated_sha": self.head,
            "acceptance": [{"status": "passed", "evidence": "CLI behavior verified"}],
            "reviews": reviews(self.base, self.head),
            "checks": {"head": self.head, "passed": True, "evidence": "full integrated checks"},
        }
        self.checkpoint: dict[str, Any] = {
            "run_id": "fixture-run", "spec": 34, "base": self.base, "head": self.head, "pr": 123,
            "tickets": [self.ticket], "required_checks": sorted(gate.HOST_CHECKS),
            "final_reviews": reviews(self.base, self.head),
            "final_checks": {"head": self.head, "passed": True, "evidence": "end-to-end checks"},
        }
        self.pr: dict[str, Any] = {
            "state": "OPEN", "isDraft": True, "baseRefName": "main",
            "headRefName": "integration/spec-34", "headRefOid": self.head, "mergeCommit": None,
            "closingIssuesReferences": [{"number": 34}, {"number": 35}],
            "statusCheckRollup": [{"name": name, "status": "COMPLETED", "conclusion": "SUCCESS"}
                                  for name in self.checkpoint["required_checks"]],
        }
        self.previous = Path.cwd()
        os.chdir(self.repo)
        self.addCleanup(os.chdir, self.previous)
        self.environment = patch.dict(os.environ, self.env, clear=True)
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, env=self.env, check=True,
                              capture_output=True, text=True).stdout.strip()

    def commit(self, content):
        (self.repo / "behavior.txt").write_text(content, encoding="utf-8")
        self.git("add", "behavior.txt")
        self.git("-c", "commit.gpgsign=false", "commit", "-m", content)
        return self.git("rev-parse", "HEAD")

    def dependent(self):
        self.checkpoint["tickets"].append({"number": 36, "status": "pending", "dependencies": [35]})

    def blocked(self, pattern, merged=False):
        with self.assertRaisesRegex(gate.Blocked, pattern):
            gate.publication(self.checkpoint, self.pr, merged=merged)

    def test_integrated_open_prerequisite_unlocks_dependent(self):
        self.dependent()
        self.assertEqual(gate.frontier(self.checkpoint), [36])
        self.assertEqual(self.ticket["issue_state"], "OPEN")

    def test_closed_issue_without_integration_evidence_does_not_unlock(self):
        self.dependent()
        self.ticket.update(status="pending", issue_state="CLOSED", state_reason="completed")
        self.assertEqual(gate.frontier(self.checkpoint), [35])

    def test_blocked_prerequisite_and_active_tickets_are_not_dispatched(self):
        self.dependent()
        self.ticket["status"] = "blocked"
        self.assertEqual(gate.frontier(self.checkpoint), [])
        self.ticket["status"] = "active"
        self.assertEqual(gate.frontier(self.checkpoint), [])

    def test_missing_prerequisite_and_dependency_cycle_fail(self):
        self.ticket["dependencies"] = [999]
        self.blocked("Missing prerequisite")
        self.ticket["dependencies"] = [35]
        self.blocked("Dependency cycle")

    def test_duplicate_ticket_and_missing_graph_fail(self):
        self.checkpoint["tickets"].append(copy.deepcopy(self.ticket))
        self.blocked("duplicate issue")
        self.checkpoint["tickets"] = []
        self.blocked("Missing ticket graph")

    def test_unmerged_worker_commit_is_not_integration_evidence(self):
        self.git("checkout", "-b", "unmerged-worker", self.base)
        unmerged = self.commit("worker-only behavior")
        self.git("checkout", "integration/spec-34")
        self.ticket.update(candidate_head=unmerged, integrated_sha=unmerged,
                           reviews=reviews(self.base, unmerged),
                           checks={"head": unmerged, "passed": True, "evidence": "worker checks"})
        self.blocked("not integrated")

    def test_not_planned_closure_is_not_evidence(self):
        self.dependent()
        self.ticket.update(status="blocked", issue_state="CLOSED", state_reason="not_planned")
        self.assertEqual(gate.frontier(self.checkpoint), [])

    def test_integrated_ticket_with_unresolved_prerequisite_fails(self):
        self.checkpoint["tickets"].append({"number": 36, "status": "blocked", "dependencies": []})
        self.ticket["dependencies"] = [36]
        self.blocked("unresolved prerequisites")

    def test_external_prerequisite_must_be_delivered_on_main(self):
        self.ticket["in_scope"] = False
        self.dependent()
        self.blocked("not delivered")
        self.ticket["status"] = "delivered"
        self.ticket["delivery"] = {"pr": 100, "merge_sha": self.head}
        delivered = dict(self.pr, state="MERGED", mergeCommit={"oid": self.head})
        with patch.object(gate, "pr_data", return_value=delivered):
            self.blocked("external delivery not verified")
        self.git("update-ref", "refs/remotes/origin/main", self.head)
        with patch.object(gate, "pr_data", return_value=delivered):
            self.assertEqual(gate.frontier(self.checkpoint), [36])

    def test_stale_integration_head_and_symbolic_evidence_fail(self):
        self.commit("later change")
        self.blocked("Stale integration HEAD")
        self.checkpoint["head"] = "HEAD"
        self.blocked("pinned base/head")

    def test_wrong_worktree_branch_fails(self):
        self.git("checkout", "main")
        self.blocked("assigned integration worktree")

    def test_failed_missing_or_stale_review_blocks_ready(self):
        self.checkpoint["final_reviews"][0]["approved"] = False
        self.blocked("review")
        self.checkpoint["final_reviews"] = reviews(self.base, self.base)
        self.blocked("stale")
        self.checkpoint["final_reviews"] = [reviews(self.base, self.head)[0]]
        self.blocked("exactly one")

    def test_review_of_wrong_baseline_does_not_cover_the_spec(self):
        self.checkpoint["final_reviews"] = reviews(self.head, self.head)
        self.blocked("stale")

    def test_resume_after_marking_pr_ready_is_idempotent_but_rechecks_evidence(self):
        self.pr["isDraft"] = False
        gate.publication(self.checkpoint, self.pr)
        self.pr["statusCheckRollup"] = []
        self.blocked("Required CI not successful")

    def test_external_candidate_after_delivered_head_is_rejected(self):
        later = self.commit("not in delivered PR")
        self.checkpoint["head"] = later
        self.ticket.update(in_scope=False, status="delivered", candidate_head=later,
                           reviews=reviews(self.base, later),
                           delivery={"pr": 100, "merge_sha": self.head})
        self.dependent()
        delivered = dict(self.pr, state="MERGED", mergeCommit={"oid": self.head})
        with patch.object(gate, "pr_data", return_value=delivered):
            self.blocked("candidate is not integrated")

    def test_cli_live_tracker_failure_is_blocked(self):
        checkpoint = self.repo / "checkpoint.json"
        checkpoint.write_text(json.dumps(self.checkpoint), encoding="utf-8")
        with patch.object(gate, "pr_data", side_effect=gate.Blocked("GitHub unavailable")), \
                patch.object(sys, "argv", [str(SCRIPT), "ready", str(checkpoint)]), \
                redirect_stderr(io.StringIO()) as error:
            self.assertEqual(gate.main(), 1)
            self.assertIn("GitHub unavailable", error.getvalue())

    def test_failed_or_stale_local_checks_block_ready(self):
        self.checkpoint["final_checks"]["passed"] = False
        self.blocked("local checks")
        self.checkpoint["final_checks"].update(passed=True, head=self.base)
        self.blocked("stale local checks")

    def test_unavailable_mandatory_acceptance_blocks_ready(self):
        self.ticket["acceptance"][0]["status"] = "unavailable"
        self.blocked("unresolved acceptance")
        self.ticket["acceptance"][0]["status"] = "waived"
        self.blocked("unresolved acceptance")
        self.ticket["acceptance"][0]["waiver"] = "Explicit spec permission"
        gate.publication(self.checkpoint, self.pr)

    def test_partial_implementation_stays_draft(self):
        self.dependent()
        self.blocked("Partial implementation")

    def test_required_ci_missing_pending_failed_skipped_or_stale_blocks_ready(self):
        for conclusion, status in (("FAILURE", "COMPLETED"), ("SKIPPED", "COMPLETED"),
                                   ("SUCCESS", "IN_PROGRESS"), ("CANCELLED", "COMPLETED")):
            with self.subTest(conclusion=conclusion, status=status):
                self.pr["statusCheckRollup"][0].update(conclusion=conclusion, status=status)
                self.blocked("Required CI not successful")
        self.pr["statusCheckRollup"] = []
        self.blocked("Required CI not successful")
        self.pr["headRefOid"] = self.base
        self.blocked("Published PR HEAD")

    def test_empty_or_incomplete_required_matrix_blocks_ready(self):
        self.checkpoint["required_checks"] = []
        self.blocked("three-host CI matrix")
        self.checkpoint["required_checks"] = [next(iter(gate.HOST_CHECKS))]
        self.blocked("three-host CI matrix")

    def test_additional_failed_or_pending_ci_blocks_ready(self):
        self.pr["statusCheckRollup"].append({"name": "ticket-specific", "status": "QUEUED"})
        self.blocked("Additional CI failed/pending")
        self.pr["statusCheckRollup"][-1].update(status="COMPLETED", conclusion="FAILURE")
        self.blocked("Additional CI failed/pending")

    def test_optional_skipped_release_does_not_block_successful_matrix(self):
        self.pr["statusCheckRollup"].append({"name": "release", "status": "COMPLETED",
                                           "conclusion": "SKIPPED"})
        gate.publication(self.checkpoint, self.pr)
        self.assertEqual(self.pr["state"], "OPEN")
        self.assertEqual(self.ticket["issue_state"], "OPEN")

    def test_wrong_base_branch_missing_closing_reference_and_closed_pr_fail(self):
        original = copy.deepcopy(self.pr)
        for update, message in (({"baseRefName": "develop"}, "Wrong PR base"),
                                ({"headRefName": "integration/awtrix-cli"}, "Wrong PR base"),
                                ({"closingIssuesReferences": [{"number": 34}]}, "PR must close"),
                                ({"state": "CLOSED"}, "PR is not open")):
            with self.subTest(update=update):
                self.pr = dict(original, **update)
                self.blocked(message)

    def test_open_pr_cannot_complete_issues(self):
        self.blocked("PR is not merged", merged=True)

    def test_merged_pr_must_be_reachable_from_main(self):
        self.pr.update(state="MERGED", mergeCommit={"oid": self.head})
        self.blocked("not reachable from origin/main", merged=True)
        self.git("update-ref", "refs/remotes/origin/main", self.head)
        gate.publication(self.checkpoint, self.pr, merged=True)

    def test_squash_merge_is_delivery_without_worker_sha_on_main(self):
        self.git("checkout", "main")
        self.git("merge", "--squash", "integration/spec-34")
        self.git("-c", "commit.gpgsign=false", "commit", "-m", "squash delivery")
        squashed = self.git("rev-parse", "HEAD")
        self.git("update-ref", "refs/remotes/origin/main", squashed)
        self.git("checkout", "integration/spec-34")
        self.assertFalse(gate.ancestor(self.head, "origin/main"))
        self.pr.update(state="MERGED", mergeCommit={"oid": squashed})
        gate.publication(self.checkpoint, self.pr, merged=True)

    def test_merging_partial_work_does_not_make_it_complete(self):
        self.git("update-ref", "refs/remotes/origin/main", self.head)
        self.pr.update(state="MERGED", mergeCommit={"oid": self.head})
        self.ticket["acceptance"][0]["status"] = "unavailable"
        self.blocked("unresolved acceptance", merged=True)

    def test_cli_checkpoint_resume_recomputes_frontier(self):
        self.dependent()
        checkpoint = self.repo / "checkpoint.json"
        checkpoint.write_text(json.dumps(self.checkpoint), encoding="utf-8")
        result = subprocess.run([sys.executable, str(SCRIPT), "frontier", str(checkpoint)],
                                capture_output=True, text=True, env=self.env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"runnable": [36]})
        self.commit("change after checkpoint")
        result = subprocess.run([sys.executable, str(SCRIPT), "frontier", str(checkpoint)],
                                capture_output=True, text=True, env=self.env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Stale integration HEAD", result.stderr)

    def test_cli_uses_live_pr_and_never_mutates_tracker(self):
        checkpoint = self.repo / "checkpoint.json"
        checkpoint.write_text(json.dumps(self.checkpoint), encoding="utf-8")
        commands = []
        original = gate.command

        def live(*args):
            commands.append(args)
            if args[0] == "gh":
                return json.dumps(self.pr)
            return original(*args)

        with patch.object(gate, "command", side_effect=live), \
                patch.object(sys, "argv", [str(SCRIPT), "ready", str(checkpoint)]), \
                redirect_stdout(io.StringIO()) as output:
            self.assertEqual(gate.main(), 0)
            self.assertTrue(json.loads(output.getvalue())["passed"])
        gh_commands = [args for args in commands if args[0] == "gh"]
        self.assertEqual(len(gh_commands), 1)
        self.assertEqual(gh_commands[0][:6], ("gh", "pr", "view", "123", "--repo", "toinux/awtrix-cli"))
        self.assertIn("statusCheckRollup", gh_commands[0][-1])

    def test_cli_missing_or_malformed_checkpoint_is_blocked(self):
        checkpoint = self.repo / "checkpoint.json"
        for content in (None, "invalid JSON", "[]", '{"spec": 34}'):
            with self.subTest(content=content):
                if content is not None:
                    checkpoint.write_text(content, encoding="utf-8")
                with patch.object(sys, "argv", [str(SCRIPT), "frontier", str(checkpoint)]), \
                        redirect_stderr(io.StringIO()) as error:
                    self.assertEqual(gate.main(), 1)
                    self.assertIn("BLOCKED:", error.getvalue())


if __name__ == "__main__":
    unittest.main()
