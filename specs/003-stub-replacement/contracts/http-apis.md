# External HTTP API Contracts

**Branch**: `003-stub-replacement` | **Date**: 2026-04-16

Documents the external HTTP APIs consumed by stub replacements. These are third-party APIs — we consume them, not define them.

## BrightID Verification API

- **Endpoint**: GET /node/v6/verifications/{context}/{contextId}
- **Base URL**: https://app.brightid.org (or configured node)
- **Response**: `{"data": {"unique": bool, "contextIds": [string], ...}}`
- **Error**: `{"error": true, "errorMessage": string, "errorNum": int}`
- **Auth**: None (public API)

## Sigstore Rekor API

- **Endpoint**: POST /api/v1/log/entries
- **Base URL**: https://rekor.sigstore.dev (public) or private instance
- **Request body**: hashedrekord entry (JSON)
- **Response**: Log entry with UUID, log index, inclusion proof
- **Auth**: None (public instance)

## Apple DeviceCheck / App Attest

- **Endpoint**: POST /v1/attestation/verify
- **Base URL**: https://data.appattest.apple.com (production)
- **Request body**: CBOR attestation object
- **Response**: Verification result
- **Auth**: Apple Developer credentials (JWT)

## Twilio Verify API

- **Send code**: POST /v2/Services/{ServiceSid}/Verifications
- **Check code**: POST /v2/Services/{ServiceSid}/VerificationCheck
- **Base URL**: https://verify.twilio.com
- **Auth**: Basic (AccountSid:AuthToken)

## OAuth2 Provider Endpoints

| Provider | Auth URL | Token URL |
|-|-|-|
| GitHub | https://github.com/login/oauth/authorize | https://github.com/login/oauth/access_token |
| Google | https://accounts.google.com/o/oauth2/v2/auth | https://oauth2.googleapis.com/token |
| Twitter | https://twitter.com/i/oauth2/authorize | https://api.twitter.com/2/oauth2/token |

## Firecracker API Socket (local)

- **Transport**: HTTP over Unix domain socket
- **Endpoints**: PUT /machine-config, /boot-source, /drives/{id}, /network-interfaces/{id}, /actions, /snapshot/create
- **Auth**: None (local socket, process-level access control)
