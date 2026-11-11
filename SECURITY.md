# Security policy

panosync exists to move clipboard contents (which often include passwords and tokens) between machines,
so security reports are taken seriously and are welcome.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub: open the repository's **Security** tab and click
**Report a vulnerability** ([direct link](https://github.com/msbayindir/panosync/security/advisories/new)).
Only the maintainers can see the report.

Please include:

- what an attacker can do, and from which position (the relay operator, someone on the network,
  another local user, someone holding a stolen device, …)
- the affected version (`panosync --version`) and platform
- steps or a proof of concept to reproduce it

You should get an answer within 7 days. Once a fix is ready we will agree on a disclosure date
together, and you will be credited in the advisory unless you prefer not to be.

## Scope

In scope, for example:

- reading or modifying clipboard contents without the key
- getting the key or the clipboard contents from the relay, the logs or the config file
- bypassing replay protection or the relay's limits
- making a client crash or misbehave with crafted messages

Already documented and not considered vulnerabilities (see the *Known limitations* section of the
[README](README.md#security-model)): the metadata the relay can see (IP addresses, timing, sizes,
channel id), the single shared key without forward secrecy, the relay being able to drop, delay or
replay messages within the 5-minute window to a restarted client, and passwords being synced like
any other text (there is no password-manager detection yet).

## Supported versions

Only the latest release receives security fixes.
