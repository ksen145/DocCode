# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

This document describes what DocCode signs, how a release is signed, who approves it and how to verify the result.

## Scope

DocCode publishes Windows x64 builds on the [Releases](https://github.com/ksen145/DocCode/releases) page:

| File | Signed |
|---|---|
| `DocCode-<version>-x64-setup.exe` — NSIS installer | Yes, Authenticode (SHA-256 + RFC 3161 timestamp) |
| `doccode.exe` installed by the setup | Yes — signed before packaging, the signature survives the NSIS archive |
| `SHA256SUMS.txt` | No — contains the installer hash |

Linux packages (`.deb`, AppImage) are built from the same source but are not Authenticode-signed.

## How releases are signed

1. A maintainer bumps `version` in `Cargo.toml` (`packaging/bump-version.ps1`) and pushes a `v*` tag.
2. GitHub Actions builds `doccode.exe` from that tag on GitHub-hosted runners (`windows-latest`).
3. The unsigned binary is uploaded as a workflow artifact and submitted to SignPath; an approver confirms the request manually.
4. The signed `doccode.exe` is packed into the NSIS installer, which is submitted and approved in a second request.
5. The workflow verifies both signatures (`Get-AuthenticodeSignature` → `Valid`) before publishing.

The private key never leaves SignPath, no signing secrets are stored in this repository, and every signed file is traceable to the workflow run and commit it was built from.

- Workflow: [`.github/workflows/release.yml`](.github/workflows/release.yml)
- SignPath artifact configurations: [`packaging/windows/signpath/`](packaging/windows/signpath/)
- Local builds can be signed with your own certificate via `packaging/windows/sign.ps1`.

## Roles

- Committers and reviewers: [@ksen145](https://github.com/ksen145)
- Approvers: [@ksen145](https://github.com/ksen145)

Changes from contributors are proposed as pull requests and reviewed by a maintainer. Signing requests are never approved automatically.

## Privacy policy

DocCode runs entirely on the local machine. It sends no data anywhere except the Ollama endpoint configured by the user: source code, prompts and generated documentation never leave the machine. See [Data locations](README.md#data-locations) for the exact paths.

## Verifying a release

```powershell
Get-AuthenticodeSignature .\DocCode-<version>-x64-setup.exe |
  Format-List Status, StatusMessage, SignerCertificate, TimeStamperCertificate
```

`Status` must be `Valid` and the signer must be `SignPath Foundation`. Compare the SHA-256 with `SHA256SUMS.txt` from the same release.

> A valid signature proves that the file was built from this repository and signed by SignPath Foundation. It does not guarantee that SmartScreen stays silent for the first downloads: publisher reputation is built over releases.
