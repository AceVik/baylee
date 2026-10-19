These are public **test-only** fixtures, never deployment credentials.

`localhost.der` is a self-signed ECDSA P-256 certificate with localhost and
127.0.0.1 subject alternative names, valid 2020–2100. `localhost-key.der` is its
unencrypted PKCS#8 private key, intentionally committed for deterministic offline
TLS refusal tests. The tests bind only an ephemeral numeric loopback listener.
The certificate is not installed in any trust store; .NET CertificateRequest
generated the files entirely in memory. Never trust or deploy this keypair.
