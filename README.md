# Faith

## Overview

Faith is a Windows-focused security research and malware-analysis repository. Its current source tree is an incomplete proof of concept containing modules associated with remote administration, system discovery, credential collection, persistence, screen capture, keylogging, propagation, evasion, and data exfiltration.

This repository is for authorized analysis and defensive research only. Do not run it on personal, production, or third-party systems.

## Architecture

```text
Core controller
|-- Configuration and startup
|-- Discovery and privilege enumeration
|-- Collection modules
|-- Command and control transport
|-- Persistence and propagation modules
|-- Staging, encryption, and exfiltration
`-- Windows-specific support code
```

## Repository Areas

- `core/`: startup flow, configuration, and controller logic
- `modules/c2-comm]/`: beaconing, transport, and message protection experiments
- `modules/discovery/`: system, user, privilege, and network discovery
- `modules/cred-dump/`: credential-collection research code
- `modules/exfil/`: file harvesting, staging, compression, and transport experiments
- `modules/persistence/`: Windows persistence research
- `modules/propagation/`: Outlook and SMTP propagation research
- `utils/`: cryptography, encoding, and evasion-related helpers
- `implants/`: stager and configuration artifacts

## Current Status

This is not a finished or supported application. The codebase currently contains missing module wiring, placeholder implementations, inconsistent file and directory names, and code that does not compile as a complete project. Several components also depend on Windows-specific APIs and elevated privileges.

## Safe Analysis Guidance

- Review the code statically before executing anything.
- Use an isolated, disposable analysis VM with networking disabled or tightly controlled.
- Never use real credentials, personal files, or production infrastructure.
- Preserve samples and logs for analysis, and remove any persistence created during testing.
- Treat binaries, scripts, and remote endpoints in this repository as untrusted.

## Defensive Research Goals

Appropriate follow-up work includes documenting indicators of compromise, mapping behaviors to MITRE ATT&CK techniques, writing detection rules, and replacing offensive modules with harmless simulations for training.