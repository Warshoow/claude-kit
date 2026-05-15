# Security Policy

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Open a [GitHub Security Advisory](https://github.com/Warshoow/claude-kit/security/advisories/new) (private by default) with the subject `[claude-kit] Security vulnerability`. Include:

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof of concept
- The claude-kit version affected

You'll receive a response within 72 hours. If the issue is confirmed, a patch will be released as soon as possible and you'll be credited in the release notes (unless you prefer to stay anonymous).

## Scope

Things worth reporting:

- Arbitrary code execution via crafted bundle share codes or marketplace imports
- Path traversal when applying bundles to a project
- Privilege escalation via the symlink mechanism

Out of scope:

- The fact that unsigned binaries trigger SmartScreen / Gatekeeper warnings (by design, see the [release notes](https://github.com/Warshoow/claude-kit/releases))
- Vulnerabilities in third-party dependencies that don't have a known exploit path in this app
