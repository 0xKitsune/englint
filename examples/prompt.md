# Draft a proposal with englint

Use this prompt with an authoring agent. Supply the concrete change, source files, and decisions under “Input” before running it. Do not treat the fictional example as an existing Tempo feature.

## Prompt

Write a reviewable Tempo Improvement Proposal for the change described below. Read the supplied implementation and requirements before drafting. Save the result as `proposal.md`.

Use the sections Abstract, Motivation, Assumptions, Specification, Tooling, Observability, Threat Model, and Invariants. Include a title and the metadata requested by the target repository's current TIP template. Englint does not validate metadata, so check it separately.

State the problem and resulting behavior first. Name the actor for each action. Define inputs, outputs, bounds, units, state transitions, failure behavior, and authorization checks. Describe behavior before activation, after activation, and when activation is unscheduled. Identify compatibility constraints and deployment ordering. State important behavior that remains unchanged.

Explain tooling changes and production signals. Name the actors and trust boundaries. For each invariant, give a test case that can falsify it. Include boundary values and failure cases. If a section does not apply, explain why.

Use concrete claims and familiar words. Avoid promotional claims, unnamed evidence, stock introductions, repeated hedges, and vague requirements. Cite supplied evidence for claims. Do not invent measurements, protocol facts, references, or decisions to satisfy a lint.

Run:

```sh
englint proposal.md --profile tip --format json --fail-on advice
```

If the checkout's vocabulary policy applies, run this as well:

```sh
englint proposal.md --config profiles/tip.toml --format json --fail-on advice
```

Read each diagnostic in context and revise the draft. Repeat until the enabled checks pass or a remaining finding requires a human decision. Never change technical meaning just to remove a finding. Report any justified policy exceptions explicitly. Do not suppress a finding silently.

Return the proposal, the command used, its exit code, and a short list of unresolved factual or design questions. A passing lint report does not replace protocol or security review. If information is missing, list the questions rather than presenting the draft as complete.

## Input

- Proposed change and motivation:
- Relevant source files and public references:
- Decisions already made:
- Compatibility and activation requirements:
- Known constraints and open questions:

## Other document types

For a technical specification, use `--profile technical-spec` and the sections Summary, Motivation, Specification, Compatibility, Security, and Test plan. Keep the same evidence and verification requirements.

For ordinary prose, use `--profile plain --input-format markdown`. No section structure is required.

## Examples

The `prose`, `technical-spec`, and `tip` folders each contain `before.md`, `after.md`, and corresponding JSON reports. These are authored fixtures, not a benchmark of model success. The technical cache spec and TIP are fictional. Run `python3 scripts/examples.py` to regenerate reports, and `./scripts/check.sh` to verify that the outputs still match.
