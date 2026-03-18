# Session Context

## User Prompts

### Prompt 1

## Skill Invocation

You MUST run the `/review` skill to complete this task.
Use the Skill tool: `skill: "review"`

The skill handles the methodology. The context below tells you what issue you are working on and where you are in the pipeline.

# Global Agent Instructions

You are an autonomous coding agent running in a headless orchestration session.
There is no human in the loop — do not ask questions or wait for input.

## Ground rules

1. Read and follow the project's CLAUDE.md for coding ...

### Prompt 2

Base directory for this skill: /Users/samjp/.claude/skills/review

<!-- AUTO-GENERATED from SKILL.md.tmpl — do not edit directly -->
<!-- Regenerate: bun run gen:skill-docs -->

## Preamble (run first)

```bash
_UPD=$(~/.claude/skills/gstack/bin/gstack-update-check 2>/dev/null || .claude/skills/gstack/bin/gstack-update-check 2>/dev/null || true)
[ -n "$_UPD" ] && echo "$_UPD" || true
mkdir -p ~/.gstack/sessions
touch ~/.gstack/sessions/"$PPID"
_SESSIONS=$(find ~/.gstack/sessions -mmin -120 -ty...

