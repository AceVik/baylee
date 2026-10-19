# Native client transport security

The gateway HTTP/WebSocket transport in `baylee-client/src/transport.rs` verifies
TLS certificates in **every** build profile. Debug builds no longer accept
arbitrary self-signed certificates. HTTPS and WSS use the same public root set;
the separate artwork HTTP policy can use the operating system's trust store,
but certificate verification remains enabled there as well.

Cleartext HTTP/WS is accepted only for numeric loopback addresses such as
`127.0.0.1` and `[::1]`. DNS names, including `localhost`, are not a cleartext
exception. Central's development-mode origin policy still controls whether a
loopback gateway may be admitted. No custom-certificate bypass or trust-store
modification is provided. Development should use the explicit loopback service
addresses; a future private-CA feature would need its own bounded trust design.

The gateway HTTP agent follows no redirects. This protects both Authorization
headers and credentials in request bodies, including 307/308 redirects that
would otherwise repeat a password-bearing POST. Callers receive the original
3xx response as a non-success. WebSocket upgrades likewise do not follow
redirects. URLs with embedded user information are rejected before connection.

HTTP has a five-second connect limit and a 60-second default total request
deadline. Explicit request budgets are capped at 120 seconds; `None`/zero cannot
disable the deadline. A hosted replay's existing 75-second budget remains valid.
WebSocket DNS/connect/TLS/upgrade has a 15-second total admission budget (each
TCP attempt at most five seconds), enforced before each handshake read/write
so trickling bytes cannot reset it. Established socket writes have a ten-second
I/O timeout. DNS uses the existing HTTP library's timeout-aware resolver; an OS
resolver call may finish later internally, but the caller stops waiting at its
deadline. No custom DNS or cryptography dependency is added.

Native WebSocket queues are independently limited to 256 pending events/messages
and 64 MiB of payload in each direction. The existing 64-MiB reassembled game
message allowance is preserved. A receive overflow discards the pending batch,
publishes one terminal failure and stops the producer; a send overflow also
publishes a terminal failure and prevents further queued writes. Queue admission
never waits for network progress. The outgoing byte charge includes the message
currently being written and is released on write completion or discard.
The UI detaches one bounded receive batch at a time, so a continuously sending
peer cannot refill an endless drain loop. Dropping the receiver stops subsequent
publication. A detached batch and parser buffers are separate from the queue
budget, so 64 MiB is not a whole-process memory limit.

Browser builds retain the browser/ewebsock receive storage, whose byte allocation
is not controlled by this native adapter. Their UI pump processes at most 256
events per call; this limits work per pump without claiming a browser queue cap.

Native HTTP response reads are bounded while streaming, after HTTP transfer and
content decoding. The general catalog/asset-compatible bound is 256 MiB;
hosted replay responses have a 32-MiB bound matching the gateway's maximum tape
response. The reader consumes at most the bound plus one detection byte and
does not trust Content-Length. A proxy-inflated JSON replay response is subject
to the same 32-MiB network limit.

Gzip record/tape decoding has a separate 256-MiB total decoded bound, shared by
all concatenated gzip members. Exceeding it rejects the entire prefix, even
when the last member is also truncated. Direct JSON tape parsing applies the
same bound. The existing local crash-tail recovery within this budget remains
supported; CRC/trailing errors are not cryptographic authentication. An
oversized local record receives `ReadBack::TooLarge` and is left unchanged,
not deleted or rewritten as a recovered prefix. Hosted tape cache publication
occurs only after successful tape parsing.

Kept, archived and explicitly opened local replay files have a separate
256-MiB encoded-file bound. The shared file reader checks opened-file metadata
before allocation and still streams at most limit + 1 bytes to catch concurrent
growth. Crash recovery preserves both encoded-file and decoded-data oversize
failures unchanged. Existing path/symlink and live-file-lock handling is retained.

Regression tests use owned ephemeral loopback listeners and join every server
thread. They cover real HTTPS/WSS refusal of an untrusted test certificate,
301/302/303/307/308 refusal for bearer and password-body requests, HTTP body
framing and content-decoding limits, interrupted bodies, concatenated gzip,
oversized damaged tails, valid records larger than 2 MiB, unchanged oversized
local files, silent/trickling HTTP and WebSocket peers, reflected-ticket refusal
diagnostics, and queue byte/count/closure/order boundaries. No live gateway,
GPU client, user credential store or installed certificate is used.
