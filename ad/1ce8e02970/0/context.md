# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# Fix: GitHub Copilot provider — Responses API support for gpt-5.4+

## Context

GitHub Copilot provider (`crates/providers/src/github_copilot.rs`) hardcodes `/chat/completions` for all models. Newer OpenAI models (gpt-5.4, gpt-5.4-pro, gpt-5.2-pro) only support the Responses API (`/responses`), returning HTTP 400 with `unsupported_api_for_model`. See issue #392.

The OpenAI provider already has Responses API support via WebSocket, but that's gated behind `api.o...

### Prompt 2

commit, push, create a PR

### Prompt 3

any remaining comments to fix at https://github.com/moltis-org/moltis/pull/393 ?

