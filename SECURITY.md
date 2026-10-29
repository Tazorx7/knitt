# Security Policy

## Reporting a Vulnerability
Please report security vulnerabilities privately to the maintainers. Do not open public issues for security concerns.

## Security Practices
- Tokens are stored in the Linux Secret Service via keyring; never logged or exposed to the frontend.
- IPC commands are validated on the backend.
- No plaintext secrets in config files.
