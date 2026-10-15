---
id: TIP-XXXX
title: Reject oversized request batches
description: A fictional example of a bounded request batch
authors: Example author
status: Draft
related: []
protocolVersion: Unscheduled
---

# TIP-XXXX: Reject oversized request batches

## Abstract

This fictional proposal limits a request batch to 64 entries. A validator rejects larger batches before executing any entry.

## Motivation

A count limit bounds the number of dispatch operations per batch. This example demonstrates specification writing; it proposes no change to the Tempo protocol.

## Assumptions

The decoder provides the complete entry count before execution. If decoding fails, the validator rejects the batch without changing state.

Activation requires all validators to use the same configured protocol version. An unscheduled version retains the previous behavior.

## Specification

After activation, the validator MUST reject a batch containing more than 64 entries with the error `BatchTooLarge`.

The validator MUST check the count before executing entries or charging execution fees. Rejection MUST preserve all account balances and nonces.

A batch containing 1 through 64 entries follows the existing execution rules. Empty batches retain their existing rejection behavior.

Before activation, all batches follow the previous protocol rules. This proposal changes neither entry encoding nor signature validation.

## Tooling

The SDK exposes `BatchTooLarge` as a typed error. Its batch builder rejects a list of more than 64 entries before signing.

The CLI prints the supplied count and the limit. Clients must split a larger request into independent batches and review the loss of atomicity.

## Observability

The node MUST increment `batch_rejections_total` with reason `count_limit` when it rejects an oversized batch. This counter measures rejected requests at the node.

The counter contains no account addresses or request payloads. The node emits no consensus event because the rejected batch changes no state.

## Threat Model

An untrusted sender controls the batch size. Validators MUST enforce the bound even when a client bypasses SDK validation.

This count limit does not bound the encoded byte size of an entry. Existing decoder limits remain necessary to bound memory use.

## Invariants

An oversized batch MUST leave balances and nonces unchanged. All validators at the same activated version MUST agree on rejection.

Test batches containing 0, 1, 64, and 65 entries before and after activation. Include malformed encodings and an unscheduled activation.

For every rejected batch, compare the state root before and after validation. Verify that no execution fee has changed any balance.
