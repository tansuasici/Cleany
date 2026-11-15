---
title: Side-stripe accents (border-left / inset bar) read as AI slop
created: 2026-11-15
updated: 2026-11-15
tags: [ui, design-kit, slop]
problem_type: process
source: correction
confidence: high
top_rule: true
status: active
related: []
supersedes: []
applies_to: [design-kit, ui]
contradicts: []
related_decisions: []
---

## Issue

The tree table marked the selected row with a 2px accent bar on its left edge (`shadow-[inset_2px_0_0_var(--c-accent)]`), and the failed-scan message used `border-l-2 border-negative`. The user called this AI slop.

## Root Cause

UI was built from a reflex pattern instead of the design kit: the selection style was not traced to `ui/DESIGN_MAP.md`, and no `/slop-audit` ran on the screen before shipping.

## Rule

Never mark a row, card or message with a colored side stripe. Selection and emphasis come from a surface step (`bg-raised`) and a full hairline border. For UI work, follow the design kit in `ui/` (DESIGN_MAP, then `/slop-audit` before calling it done).

## Verification

`grep -rnE "border-l-[0-9]|inset_[0-9]+px_0_0" ui/src` returns nothing.

## References

- `ui/DESIGN_MAP.md` → Don'ts
