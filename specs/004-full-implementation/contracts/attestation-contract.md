# Attestation Contract

## CertificateChainValidator Trait

```
validate_chain(quote: &[u8], certs: &[Vec<u8>]) -> Result<ChainVerification>
root_fingerprint() -> [u8; 32]
```

### Input
- `quote`: Platform-specific attestation quote (TPM2 TPMS_ATTEST, SEV-SNP REPORT, TDX QUOTE)
- `certs`: DER-encoded certificate chain (leaf, intermediates, root)

### Output
- `ChainVerification { valid: bool, trust_tier: TrustTier, platform: Platform, errors: Vec<String> }`

### Behavior
- Verify each signature in chain: leaf → intermediate → root
- Compare root fingerprint against pinned constant
- Check certificate expiry dates
- Check manufacturer OIDs (TPM2: 2.23.133.x)
- Invalid chain → reject (not downgrade to T0)
- Empty attestation → downgrade to T0 (WASM-only)

### Error Codes
- WC-009: InvalidAttestation — chain verification failed
- WC-010: UnsupportedPlatform — unknown attestation format
