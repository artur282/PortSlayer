# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| `main` (latest commit) | Yes |
| Latest tagged release | Yes |
| Older releases | No |

## Reporting a vulnerability

Please do not report security vulnerabilities in public issues, discussions, or
pull requests.

Use GitHub private vulnerability reporting:

https://github.com/artur282/PortSlayer/security/advisories/new

Include:

- A description of the issue and the affected version or commit.
- Reproduction steps or a proof of concept.
- The impact you believe it has.
- Any suggested fix or mitigation.

If private reporting is unavailable, open a public issue that only asks for a
private contact channel. Do not include exploit details in that issue.

## Scope

In scope:

- Port and PID parsing from `ss` and `/proc/net`.
- Command construction for `kill`, `pkexec`, and `fuser`, including any
  user-controlled port, protocol, or PID.
- The optional `sudo -n ss` sudoers rule and its install/uninstall path.
- The desktop entry and autostart file.
- Any path that lets an unprivileged local user kill a process they should not
  be able to kill, or read information they should not be able to read.

Out of scope:

- Vulnerabilities in `ksni`, `nix`, `dbus`, or other dependencies that are
  already fixed upstream.
- Attacks that require a compromised local account, physical access, or an
  already-root shell, unless the application is supposed to cross that
  privilege boundary.
- The inherent ability of a local administrator to kill processes.
- Social engineering, spam, and denial-of-service against GitHub.

## Response

This is a personal open-source project maintained on a best-effort basis. We aim
to acknowledge reports within 7 days and to publish a fix or mitigation as soon
as practical. We will credit reporters in the advisory unless you ask us not to.

## Safe harbor

We will not pursue legal action for good-faith security research that follows
this policy, stays within the scope above, and avoids privacy violations, data
destruction, and service disruption.
