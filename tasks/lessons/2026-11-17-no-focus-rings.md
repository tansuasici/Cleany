---
title: The owner does not want focus rings anywhere
created: 2026-11-17
updated: 2026-11-17
tags: [ui, focus, a11y]
problem_type: process
source: correction
confidence: high
top_rule: true
status: active
related: [2026-11-15-no-side-stripe-accents]
supersedes: []
applies_to: [ui-design]
contradicts: []
related_decisions: []
---

## Issue

On first open the scanning screen showed a 2px accent outline on the focused "İptal et" button. The user said they dislike focus rings and want none anywhere.

## Root Cause

The kit default (and ui/CLAUDE.md verification step 3) asks for a visible `:focus-visible` ring, so `index.css` had a global accent outline and panels/rows added accent outlines or borders on focus. The owner's taste overrides that default.

## Rule

No focus rings in Cleany: no `outline`, no `ring-*`, no accent border on focus. `index.css` sets `:focus, :focus-visible { outline: none }`. Show keyboard position only with the surface step (`bg-raised`) that rows and menu items already use. Don't re-add a ring when following the kit's interface-check advice; mention the trade-off instead.

## Verification

`grep -rnE "focus(-visible)?:(ring|outline-[0-9a-z]|border-accent)|group-focus-visible:outline" ui/src` returns nothing.

## References

ui/DESIGN_MAP.md → Don'ts. Memory: no-focus-rings.
