# BRIEFING — 2026-09-05T05:28:37Z

## Mission
Orchestrate the development of RestPHP: a high-performance persistent PHP application server in Rust embedding Zend Engine via zero-cost C FFI.

## 🔒 My Identity
- Archetype: orchestrator
- Roles: orchestrator, user_liaison, human_reporter, successor
- Working directory: /home/cads/restphp/.agents/orchestrator_1
- Original parent: parent
- Original parent conversation ID: 77524e5a-38c1-445d-ab92-2c74e44138d1

## 🔒 My Workflow
- **Pattern**: Project
- **Scope document**: /home/cads/restphp/PROJECT.md
1. **Decompose**: Survey codebase and requirements, build Feature Inventory, establish module boundaries and interface contracts, assign milestones in PROJECT.md.
2. **Dispatch & Execute**:
   - **Delegate (sub-orchestrator)**: Spawn sub-orchestrators for milestones and E2E testing track.
   - **Direct (iteration loop)**: For milestones executed directly: Explorer(3) -> Worker(1) -> Reviewer(2) -> Challenger(2) -> Auditor(1) -> Gate.
3. **On failure** (in this order):
   - Retry: nudge stuck agent or re-send task
   - Replace: spawn fresh agent with partial progress
   - Skip: proceed without (only if non-critical)
   - Redistribute: split stuck agent's remaining work
   - Redesign: re-partition decomposition
   - Escalate: report to parent (sub-orchestrators only, last resort)
4. **Succession**: Self-succeed at 16 spawns, write handoff.md, spawn successor
- **Work items**:
  1. Survey & Codebase Exploration [in-progress]
  2. PROJECT.md Decomposition & Test Track Planning [pending]
  3. Milestone Execution & Tracking [pending]
  4. Acceptance Criteria Verification & Final Report [pending]
- **Current phase**: Phase 0 (Survey)
- **Current focus**: Survey phase to map full scope, existing repository files, and system PHP environment

## 🔒 Key Constraints
- DISPATCH-ONLY orchestrator: MUST delegate ALL work to subagents via invoke_subagent.
- NEVER write, modify, or create source code files directly.
- NEVER run build/test commands yourself — require workers to do so.
- NEVER investigate or explore the problem at the code level — dispatch Explorers for technical investigation.
- Use file-editing tools ONLY for metadata/state files (.md) in .agents/ folder.
- Binary veto on Forensic Auditor INTEGRITY VIOLATION.
- Never reuse a subagent after it has delivered its handoff — always spawn fresh.

## Current Parent
- Conversation ID: 77524e5a-38c1-445d-ab92-2c74e44138d1
- Updated: 2026-09-05T05:28:37Z

## Key Decisions Made
- Initiated Project Orchestration pattern with Survey phase.
- Dispatched 3 parallel survey subagents (spec miner + 2 explorers).

## Team Roster
| Agent | Type | Work Item | Status | Conv ID |
|-------|------|-----------|--------|---------|
| spec_miner_survey_1 | teamwork_preview_spec_miner | Specifications & Requirements Mining | in-progress | b81537ef-4e4e-471c-97c5-822aa519179c |
| explorer_survey_2 | teamwork_preview_explorer | Codebase & Environment Survey | in-progress | 90438166-6538-40ed-9afb-4eda88f6b11f |
| explorer_survey_3 | teamwork_preview_explorer | Zend FFI, SAPI & Concurrency Model | in-progress | a4558ccb-428e-4c68-946b-f7225a5997fb |

## Succession Status
- Succession required: no
- Spawn count: 3 / 16
- Pending subagents: b81537ef-4e4e-471c-97c5-822aa519179c, 90438166-6538-40ed-9afb-4eda88f6b11f, a4558ccb-428e-4c68-946b-f7225a5997fb
- Predecessor: none
- Successor: not yet spawned

## Active Timers
- Heartbeat cron: task-18
- Safety timer: pending
- On succession: kill all timers before spawning successor
- On context truncation: run manage_task(Action="list") — re-create if missing

## Artifact Index
- /home/cads/restphp/.agents/ORIGINAL_REQUEST.md — Original user request
- /home/cads/restphp/.agents/orchestrator_1/DISPATCH.md — Orchestrator dispatch log
- /home/cads/restphp/.agents/orchestrator_1/progress.md — Progress checkpoint
