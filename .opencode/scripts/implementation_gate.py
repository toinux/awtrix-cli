#!/usr/bin/env python3
"""Read-only evidence gates for /implement-spec (no tracker mutations)."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys


HOST_CHECKS = {
    "host-test (x86_64-unknown-linux-gnu)",
    "host-test (aarch64-apple-darwin)",
    "host-test (x86_64-pc-windows-msvc)",
}


def sha(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{40}", value) is not None


class Blocked(ValueError):
    """The run does not have sufficient evidence to advance."""


def require(condition, message):
    if not condition:
        raise Blocked(message)


def command(*args):
    result = subprocess.run(args, capture_output=True, text=True, check=False)
    require(result.returncode == 0, f"{' '.join(args)}: {result.stderr.strip()}")
    return result.stdout.strip()


def ancestor(commit, tip):
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", commit, tip],
        capture_output=True, text=True, check=False,
    )
    require(result.returncode in (0, 1), f"Cannot resolve ancestry: {commit} -> {tip}")
    return result.returncode == 0


def reviews_pass(reviews, base, head):
    require(isinstance(reviews, list), "Missing independent review reports")
    require(sorted(r.get("axis", "") for r in reviews) == ["Spec", "Standards"],
            "Require exactly one Standards and one Spec review")
    for review in reviews:
        require(review.get("base") == base and review.get("head") == head and review.get("approved") is True
                and review.get("evidence"), f"Missing/failed/stale {review['axis']} review")


def checks_pass(checks, head):
    require(isinstance(checks, dict) and checks.get("head") == head
            and checks.get("passed") is True and checks.get("evidence"),
            "Missing/failed/stale local checks")


def acceptance_pass(ticket):
    entries = ticket.get("acceptance")
    require(isinstance(entries, list) and entries, f"#{ticket['number']}: missing acceptance")
    for entry in entries:
        require(entry.get("evidence") and (
            entry.get("status") == "passed" or (
                entry.get("status") == "waived" and entry.get("waiver")
            )), f"#{ticket['number']}: unresolved acceptance criterion")


def validate_graph(run):
    require(type(run.get("spec")) is int and run["spec"] > 0 and run.get("run_id"),
            "Missing spec/run identity")
    tickets = run.get("tickets")
    require(isinstance(tickets, list) and tickets, "Missing ticket graph")
    by_number = {}
    for ticket in tickets:
        number = ticket.get("number")
        require(type(number) is int and number > 0 and number not in by_number,
                "Invalid or duplicate issue number")
        require(type(ticket.get("in_scope", True)) is bool, f"#{number}: invalid scope flag")
        require(ticket.get("status") in ("pending", "active", "blocked", "integrated", "delivered"),
                f"#{number}: invalid ticket status")
        require(isinstance(ticket.get("dependencies"), list), f"#{number}: missing dependencies")
        by_number[number] = ticket
    visiting, visited = set(), set()

    def visit(number):
        require(number in by_number, f"Missing prerequisite #{number}")
        require(number not in visiting, f"Dependency cycle at #{number}")
        if number in visited:
            return
        visiting.add(number)
        for dependency in by_number[number]["dependencies"]:
            require(type(dependency) is int, f"#{number}: invalid dependency")
            visit(dependency)
        visiting.remove(number)
        visited.add(number)

    for number in by_number:
        visit(number)
    require(any(t.get("in_scope", True) for t in tickets), "No scoped implementation tickets")
    return by_number


def integrated(ticket, head):
    require(ticket.get("status") in ("integrated", "delivered"),
            f"#{ticket['number']}: not integrated")
    candidate, merge = ticket.get("candidate_head"), ticket.get("integrated_sha")
    base = ticket.get("dispatch_base")
    require(sha(base) and sha(candidate) and sha(merge),
            f"#{ticket['number']}: missing pinned commit evidence")
    acceptance_pass(ticket)
    reviews_pass(ticket.get("reviews"), base, candidate)
    checks_pass(ticket.get("checks"), merge)
    require(ancestor(base, candidate) and ancestor(candidate, merge),
            f"#{ticket['number']}: candidate is not integrated")
    if ticket.get("in_scope", True):
        require(ancestor(merge, head), f"#{ticket['number']}: commits are not integrated into {head}")
    else:
        delivery = ticket.get("delivery") or {}
        pr = pr_data(delivery)
        delivered_sha = delivery.get("merge_sha")
        require(pr.get("state") == "MERGED" and pr.get("baseRefName") == "main"
                and pr.get("headRefOid") == merge and sha(delivered_sha)
                and (pr.get("mergeCommit") or {}).get("oid") == delivered_sha
                and ancestor(delivered_sha, "origin/main"),
                f"#{ticket['number']}: external delivery not verified on main")


def frontier(run):
    tickets = validate_graph(run)
    require(sha(run.get("head")) and sha(run.get("base")), "Require full pinned base/head SHAs")
    require(command("git", "branch", "--show-current") == f"integration/spec-{run['spec']}",
            "Run gate in the assigned integration worktree")
    require(run.get("head") == command("git", "rev-parse", "HEAD"), "Stale integration HEAD")
    require(run.get("base") and ancestor(run["base"], run["head"]), "Invalid original baseline")
    verified = set()
    for number, ticket in tickets.items():
        if ticket["status"] in ("integrated", "delivered"):
            if not ticket.get("in_scope", True):
                require(ticket["status"] == "delivered", f"External prerequisite #{number} not delivered")
            integrated(ticket, run["head"])
            verified.add(number)
    # A satisfied ticket cannot have an unresolved prerequisite in the same graph.
    for number in verified:
        require(all(d in verified for d in tickets[number]["dependencies"]),
                f"#{number}: integrated claim has unresolved prerequisites")
    return sorted(number for number, ticket in tickets.items()
                  if ticket.get("in_scope", True) and ticket["status"] == "pending"
                  and all(d in verified for d in ticket["dependencies"]))


def pr_data(run):
    require(type(run.get("pr")) is int and run["pr"] > 0, "Missing PR number")
    return json.loads(command(
        "gh", "pr", "view", str(run["pr"]), "--repo", "toinux/awtrix-cli", "--json",
        "state,isDraft,baseRefName,headRefName,headRefOid,mergeCommit,"
        "statusCheckRollup,closingIssuesReferences",
    ))


def publication(run, pr, merged=False):
    frontier(run)
    scoped = [t for t in run["tickets"] if t.get("in_scope", True)]
    require(all(t["status"] in ("integrated", "delivered") for t in scoped),
            "Partial implementation cannot be published as ready/completed")
    reviews_pass(run.get("final_reviews"), run["base"], run["head"])
    checks_pass(run.get("final_checks"), run["head"])
    require(pr.get("baseRefName") == "main" and
            pr.get("headRefName") == f"integration/spec-{run['spec']}", "Wrong PR base/branch")
    require(pr.get("headRefOid") == run["head"], "Published PR HEAD differs from verified HEAD")
    closing = {issue["number"] for issue in pr.get("closingIssuesReferences", [])}
    require({run["spec"], *(t["number"] for t in scoped)} <= closing,
            "PR must close the spec and every scoped ticket")
    require(pr.get("state") == ("MERGED" if merged else "OPEN"),
            "PR is not merged" if merged else "PR is not open")
    required = run.get("required_checks")
    require(isinstance(required, list) and all(isinstance(n, str) and n for n in required)
            and HOST_CHECKS <= set(required), "Require the complete three-host CI matrix")
    checks = pr.get("statusCheckRollup") or []
    for name in required:
        matches = [c for c in checks if (c.get("name") or c.get("context")) == name]
        require(matches and all(ci_success(c) for c in matches), f"Required CI not successful: {name}")
    for check in checks:
        require(ci_success(check) or (
            check.get("status") == "COMPLETED" and check.get("conclusion") in ("SKIPPED", "NEUTRAL")
        ), f"Additional CI failed/pending: {check.get('name') or check.get('context')}")
    if merged:
        merge = (pr.get("mergeCommit") or {}).get("oid")
        require(sha(merge) and ancestor(merge, "origin/main"),
                "PR merge is not reachable from origin/main")


def ci_success(check):
    if "conclusion" in check:
        return check.get("status") == "COMPLETED" and check["conclusion"] == "SUCCESS"
    return check.get("state") == "SUCCESS"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("gate", choices=("frontier", "ready", "merged"))
    parser.add_argument("checkpoint", type=Path)
    args = parser.parse_args()
    try:
        run = json.loads(args.checkpoint.read_text(encoding="utf-8"))
        if args.gate == "frontier":
            print(json.dumps({"runnable": frontier(run)}))
        else:
            publication(run, pr_data(run), merged=args.gate == "merged")
            print(json.dumps({"gate": args.gate, "head": run["head"], "passed": True}))
    except (Blocked, OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        print(f"BLOCKED: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
