# Vision

*Synced 2026-11-15 from the maintainer's project notes. A faithful summary;
the full text, with every worked example, is synced here once the automated
sync lands.*

**Outcome.** Fiducial is a portable, modular engineering system and playbook:
projects start minimal and progressively adopt only useful structure, while
remaining agent-, repo- and stack-agnostic.

**Core rule.** One system for the engineer; complexity is optional per
project. A client project can live inside Fiducial and use its templates,
standards and checks without exposing Fiducial as a product.

## Principles

1. **Progressive structure**: start with the smallest useful setup; add
   declarations, checks and generated artifacts only where they prevent a real
   class of error or save repeated work.
2. **Modular at every level**: project, domain, package, capability, adapter,
   template and check are independently usable where practical.
3. **Portable by default**: ordinary repositories, monorepos or external
   platforms; no single host, editor, agent or topology required.
4. **Agent-agnostic**: Claude Code is preferred today; the same instructions
   work for Codex, Antigravity, Lovable, v0 and people.
5. **Token-efficient**: concise entry points, progressive disclosure,
   machine-readable manifests.
6. **Real code first**: generated output is usable, testable, maintainable.
7. **Own the standards**: design systems, decisions and conventions live in
   versioned portable files, not in tool memory or dashboards.
8. **Truth and consistency**: a reused authoritative fact is declared once
   and derived; ordinary application code is not pretended to be generated.
9. **Escape hatches are features**: prototypes and bypasses have an explicit,
   scoped path.
10. **Evidence-led evolution**: track what helps and what it costs; remove
    what does not earn its place.

## Adoption ladder

Level 0 plain project → 1 guided (brief, standards, agent entry point) →
2 structured (typed declarations and derivations) → 3 verified (invariants,
checks, CI) → 4 system workspace (software, firmware, electronics, mechanical,
content, deployment, composed). Adoption is per capability or domain, never
one global level.

## Non-goals

Requiring a manifest or thesis everywhere; forcing a monorepo, framework,
cloud, editor or agent; making every file generated; capabilities as giant
documents; a proprietary design-system runtime; delaying client work to
perfect the platform.
