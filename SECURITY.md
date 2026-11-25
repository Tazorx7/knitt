# Security policy

## Reporting a vulnerability
Report security vulnerabilities privately to the maintainers. Do not open a public issue.

## Security practices
- Tokens are stored in the Linux Secret Service via keyring; never logged or exposed to the frontend.
- IPC commands are validated on the backend.
- No plaintext secrets in config files.
