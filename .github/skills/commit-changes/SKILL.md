---
name: commit-changes
description: 'Generate a Conventional Commits message based on staged changes and commit them. Use when: making a git commit, need automated commit messages following conventional commits spec.'
argument-hint: 'Optional commit scope or type (e.g., "feat", "docs", "fix")'
---

# Conventional Commits Generator

Generate and commit staged changes with automatic Conventional Commits messages.

## What It Does

1. Analyzes your staged changes using `git diff --cached`
2. Determines the commit type (feat, fix, chore, docs, refactor, perf, etc.)
3. Generates a proper Conventional Commits message with scope and description
4. Commits the staged changes with the generated message

## When to Use

- Making commits and want automatic conventional message generation
- Need consistent commit message formatting
- Want to comply with Conventional Commits specification
- Using commitlint or automated changelog tools

## How It Works

The skill performs the workflow directly from staged git changes:

1. **Validates staged changes** exist in git (`git diff --cached`)
2. **Analyzes file types** to auto-detect commit type:
   - Test files -> `test`
   - Documentation -> `docs`
   - Config files -> `chore`
   - Source code -> `feat` (default) or `refactor`
3. **Infers scope** from file paths (e.g., `src/runtime/` -> `runtime` scope)
4. **Generates description** based on detected type and changed files
5. **Validates message** with commitlint (if installed)
6. **Commits** with the generated Conventional Commits message

### Usage

1. Stage your changes:
```powershell
git add .
```

2. Inspect the staged diff:
```powershell
git diff --cached --name-status
git diff --cached --stat
```

3. Generate a Conventional Commit message from the staged changes and commit:
```powershell
git commit -m "<type>[optional scope]: <description>"
```

You can optionally verify the commit message with commitlint if your repository enforces it.

## Conventional Commits Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Valid types**: `feat`, `fix`, `docs`, `chore`, `refactor`, `perf`, `style`, `test`, `ci`

**Examples**:
- `feat(runtime): add workflow step validation`
- `fix(llm): handle provider timeout gracefully`
- `docs: update configuration examples`
- `chore(deps): update dependencies`

## Specification

See [Conventional Commits Specification](./SPECIFICATION.md) for the complete specification.
