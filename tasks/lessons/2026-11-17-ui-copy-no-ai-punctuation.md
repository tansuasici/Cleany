---
title: UI copy read as AI-written (semicolons, stiff passive phrasing)
created: 2026-11-17
updated: 2026-11-17
tags: [ui, copy, i18n]
problem_type: process
source: correction
confidence: high
top_rule: false
status: active
related: []
supersedes: []
applies_to: [ui-copy]
contradicts: []
related_decisions: []
---

## Issue

The user flagged "Tarama yalnızca okur; hiçbir dosyayı değiştirmez veya silmez." and similar strings as obviously AI-written: semicolons, dashes, and stiff translated phrasing ("dahil edilmez", "bölünmüştür", "Hata ayrıntısı:").

## Root Cause

Strings were written to be precise and compact, packing two clauses into one with ";" and using passive, documentation-style Turkish instead of how a person would say it.

## Rule

In UI copy (ui/src/lib/i18n.ts, TR and EN): no semicolons, no em/en dashes in sentences. Split into two short sentences or join with a comma/"ama"/"bu yüzden". Prefer active, everyday verbs ("toplama girmez", "durdu", "artık yerinde değil") over passive or bureaucratic forms. Keep formal "siz".

## Verification

`grep -nE "(—|–|; )" ui/src/lib/i18n.ts` shows no hits inside string literals (type signatures don't count). Read each new string aloud once.

## References

Memory: ui-copy-production-tone.
