# Security policy

deskwright controls your computer as you: with the accessibility permission it can read what apps
show and click, type and send on your behalf. Treat any client you connect to it, including
AI assistants, as having that power.

## Built-in safeguards

- Pointer input is refused when another app's window covers the target element.
- Key presses are refused unless the named app is frontmost.
- `desk_script` (AppleScript/JXA, which can run any command) is not registered unless you pass
  `--enable-script`.
- Typed and clipboard text is never echoed in results.
- Nothing is sent over the network; deskwright talks only to the local client over stdio.

## Reporting a vulnerability

Please report security problems privately through GitHub's
[security advisories](https://github.com/Sabari-1234/deskwright/security/advisories/new)
rather than in a public issue. Include the version or commit, the steps to reproduce, and the
impact. You'll get an answer as soon as possible, and a fix will be released before details
are made public.
