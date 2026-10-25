# AI usage

I used AI tools to build this challenge. This page says what I did and what the tools did.

## What I did

- I wrote the specs: the requirements, the error contract, the rules for clients, money and the cut-off, and the task list for each slice.
- I made the design decisions and chose between the trade-offs the tools found.
- I reviewed and merged every pull request.

## What the AI did

- Analysed trade-offs before each decision, for example the lock scope, the money type and how to write the cut-off file.
- Implemented the code from my specs, test first, one slice at a time, as stacked pull requests.

## Tools

| Tool | Used for |
| --- | --- |
| Claude Code with Claude Opus 5.5 | Design and orchestration: planning the slices, writing the briefs, checking each pull request. |
| Claude Code with Claude Sonnet 5.5 | Implementation, through subagents. |
| ChatGPT | A second review of the plan, to get another point of view. |

## Skills and other tools

- **My own skills.** I wrote `orquestador` (coordinates the work across sessions and tracks the pull requests) and `socratic-task-coach` (I design first, then the AI critiques the design).
- **Third-party tools.** Engram, for memory that persists between sessions.
