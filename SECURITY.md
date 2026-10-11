# Security Policy

## Supported versions

Aether is an early-stage project. Only the latest `main` is supported.

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report privately either by:

1. Opening a [private security advisory](https://github.com/jeganxthan/aether/security/advisories/new)
   on GitHub (preferred), or
2. Emailing **jegan4044@gmail.com** with the subject `[aether] security`.

Please include:

- a description of the issue and its impact,
- steps to reproduce (compositor, distro, Aether version),
- any suggested fix.

You can expect an acknowledgement within a few days. Please give a reasonable
window to ship a fix before disclosing publicly.

## Scope

Aether runs as your user, reads local system state (clipboard, `.desktop`
files, D-Bus) and draws a Wayland surface. The most relevant classes of issue
are command injection (e.g. via `.desktop` `Exec` lines), unsafe handling of
D-Bus / notification input, and path or IPC abuse. Vulnerabilities in
third-party dependencies should be reported upstream.
