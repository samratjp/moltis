# Session Context

## User Prompts

### Prompt 1

# Global Agent Instructions

You are an autonomous coding agent running in a headless orchestration session.
There is no human in the loop — do not ask questions or wait for input.

## Ground rules

1. Read and follow the project's CLAUDE.md for coding conventions and standards.
2. Never use interactive commands, slash commands, or plan mode.
3. Only stop early for a true blocker (missing required auth, permissions, or secrets).
   If blocked, post the blocker details as a Linear comment and s...

### Prompt 2

This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.

Summary:
1. Primary Request and Intent:
   Implement the CRM web UI tab (CHA-19 / P1.12) as an autonomous coding agent. The task is a "first run" implementation stage — no existing branch/PR to update. The engineering review (already completed) specified: use existing WebSocket RPC methods via `sendRpc()` (zero new Rust handlers), create 4 new...

