# Conventional Commits Specification v1.0.0

This is the complete Conventional Commits specification, v1.0.0.

## Summary

The Conventional Commits specification is a lightweight convention on top of commit messages. It provides an easy set of rules for creating an explicit commit history; which makes it easier to write automated tools on top of. This convention dovetails with SemVer, by describing the features, fixes, and breaking changes made in commit messages.

The commit message should be structured as follows:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

The commit consists of the following structural elements, to communicate intent to the consumers of your library:

1. **fix:** a commit of the type `fix` patches a bug in your codebase (this correlates with PATCH in semantic versioning).
2. **feat:** a commit of the type `feat` introduces a new feature to the codebase (this correlates with MINOR in semantic versioning).
3. **BREAKING CHANGE:** a commit that has a footer `BREAKING CHANGE:`, or appends a `!` after the type/scope, introduces a breaking API change (correlating with MAJOR in semantic versioning). A BREAKING CHANGE can be part of commits of any type.
4. **types** other than `fix:` and `feat:` are allowed, for example @commitlint/config-conventional (based on the the Angular convention) recommends `build:`, `chore:`, `ci:`, `docs:`, `style:`, `refactor:`, `perf:`, `test:`, and others.
5. **footers** other than `BREAKING CHANGE: <description>` may be provided and follow a convention similar to git trailer format.

Additional types are not mandated by the Conventional Commits specification, and have no implicit effect in semantic versioning (unless they include a BREAKING CHANGE). A scope may be provided to a commit's type, to provide additional contextual information and is contained within parenthesis, e.g., `feat(parser): add ability to parse arrays`.

## Examples

### Commit message with description and breaking change footer

```
feat: allow provided config object to extend other configs

BREAKING CHANGE: `extends` key in config file is now used for extending other config files
```

### Commit message with `!` to draw attention to breaking change

```
refactor!: drop support for Node 6
```

### Commit message with scope and `!` to draw attention to breaking change

```
refactor(api)!: drop support for Node 6
```

### Commit message with both `!` and BREAKING CHANGE footer

```
refactor(api)!: send an event to all listeners

BREAKING CHANGE: refactor to use JavaScript events instead of custom subscriptions
```

### Commit message with no body

```
docs: correct spelling of CHANGELOG
```

### Commit message with scope

```
feat(lang): add polish language
```

### Commit message with multi-paragraph body and multiple footers

```
fix: prevent racing condition when using takeLatest

Introduce a request id and a reference to latest request. Dismiss
incoming responses other than from the latest request.

Remove strdup() call from notifier fire-and-forget footers to prevent
memory leaks.

Reviewed-by: Z
Refs: #123
```

## Specification

The keywords "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

1. Commits MUST be prefixed with a type, which consists of a noun, `feat`, `fix`, etc., followed by the OPTIONAL scope, OPTIONAL `!`, and REQUIRED terminating colon and space.

2. The type `feat` MUST be used when a commit adds a new feature to your application or library.

3. The type `fix` MUST be used when a commit represents a bug fix for your application.

4. A scope MAY be provided after a type. A scope MUST consist of a noun describing a section of the codebase surrounded by parenthesis, e.g., `fix(parser):`

5. A description MUST immediately follow the colon and space after the type/scope prefix. The description is a short summary of the code changes, e.g., _fix: array parsing issue when multiple spaces were contained in string_.

6. A longer commit body MAY be provided after the short description, providing additional contextual information about the code changes. The body MUST begin one blank line after the description.

7. A commit body is free-form and MAY consist of any number of newline-separated paragraphs.

8. One or more footers MAY be provided one blank line after the body. Each footer MUST consist of a word token, followed by either a `:` or `#` separator, followed by a string value (this is inspired by the git trailer convention).

9. A footer's token MUST use `-` in place of whitespace characters, e.g., `Acked-by` (this helps differentiate the footer section from a multi-paragraph body). An exception is made for `BREAKING CHANGE`, which MAY also be written as a token by itself.

10. A footer's value MAY contain spaces and newlines, and parsing MUST terminate when the next valid footer token/separator pair is observed.

11. Breaking changes MUST be indicated in the type/scope prefix of a commit, or as an entry in the footer.

12. If included as a footer, a breaking change MUST consist of the uppercase text `BREAKING CHANGE`, followed by a colon, space, and description, e.g., _BREAKING CHANGE: environment variables now take precedence over config files_.

13. If included in the type/scope prefix, breaking changes MUST be indicated by a `!` immediately before the `:`. If `!` is used, `BREAKING CHANGE:` MAY be omitted from the footer section, and the commit description SHALL be used to describe the breaking change.

14. Types other than `feat:` and `fix:` MAY be used in your commit messages, and they are not prescribed by the Conventional Commits specification.

15. The unit of information composing conventional commits MUST NOT be treated as opaque by tool implementors, except to ignore the letter case. This allows for tooling based on conventional commits to be predictable.

16. `BREAKING-CHANGE` MUST be synonymous with `BREAKING CHANGE`, when used as a token in a footer.

## Why Use Conventional Commits?

- Automatically generating CHANGELOGs.
- Automatically determining a semantic version bump (based on the types of commits landed).
- Communicating the nature of changes to teammates, the public, and other stakeholders.
- Triggering build and publish processes.
- Making it easier for people to contribute to your projects, by allowing them to explore a more structured commit history.

## FAQ

### How should I deal with commit messages in the initial development phase?

We recommend that you proceed as if you've already released the product. Typically *somebody*, even if it's your fellow software developers, is using your software. They'll want to know what's fixed, what breaks, etc.

### Are the types in commit title case or lower case?

Any casing convention MAY be used, but it is best to be consistent.

### What do I do if the commit conforms to more than one of the commit types?

Go back and make multiple commits whenever possible. Part of the benefit of Conventional Commits is its ability to drive us to make more organized commits and PRs. This provides better history and makes it easier to revert changes if needed.

### Doesn't this discourage rapid development and fast iteration?

It discourages rapid and disorganized development. It does enable you to move fast long-term with multiple contributors on multiple projects.

### Might Conventional Commits lead developers to limit the types of commits they make because they'll be thinking in the constrained types provided?

Conventional Commits encourages us to make more of certain types of commits such as fixes. Additionally, this explicitness enables better communication and facilitates automated tooling.

### How does this relate to SemVer?

`fix` type commits should be translated to `PATCH` releases. `feat` type commits should be translated to `MINOR` releases. Commits with `BREAKING CHANGE` in the commits, regardless of type, should be translated to `MAJOR` releases.

### How should I version my extensions to the Conventional Commits specification?

We recommend using SemVer to version your own extensions to this specification.

### What should I use for a typo in a commit message?

You have a few options:
- Create a new commit that fixes the typo
- Squash it into the previous commit (you'll need to force push)
- Use the `fixup!` convention (see git-rebase documentation)

### Should all my commit messages follow the Conventional Commits pattern?

No. Squashed commits don't need to follow the Conventional Commits convention. However, it is useful to do so as it provides better history and makes it easier for people to scan your commits.

---

**Document Version**: 1.0.0  
**Source**: https://www.conventionalcommits.org/en/v1.0.0/#specification
