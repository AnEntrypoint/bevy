# Vendored lightyear patches

Two crates from lightyear 0.31.0 (the newest release on crates.io) are vendored here and
applied through `[patch.crates-io]` in `Cargo.toml`. Each patch is the smallest change that
fixes the same bug.

## The bug

`lightyear_messages` decodes every payload it reads from a transport channel as a lightyear
message: the first byte is a message net id, and the rest is that message's body. Replicon's
mutation traffic on `RepliconMutationsChannel` is raw replicon data with no net-id framing, and
replicon's own receiver reads it straight from the transport. Both readers consumed the same
queue, so the message plugin decoded replicon data as messages. The first byte `0x03` matched
`SenderMetadata`'s net id, and the next four bytes `00 00 00 01` decoded as the `u32` tick
`0x01000000 = 2^24`. That is the growth of 2^24 per message that the RTS loopback showed,
which then made the interval negative and panicked in `core::time`. Replicon also lost the
mutations the message plugin had already consumed.

## The patch

- `lightyear_messages/src/receive.rs`: a `RawChannels` resource lists channel kinds the message
  plugin must leave alone. `recv` skips those channels, so replicon reads them.
- `lightyear_messages/src/plugin.rs`: the `recv` system takes the optional `RawChannels`
  parameter.
- `lightyear_replication/src/channels.rs`: `RepliconChannelMap` setup in `finish()` adds the
  three replicon channel kinds to `RawChannels`.

The copies differ from upstream only in those files. `diff -r` against
`~/.cargo/registry/src/index.crates.io-*/lightyear_{messages,replication}-0.31.0` shows them.

## Dropping it

Remove the two `[patch.crates-io]` lines in `Cargo.toml` and delete this directory once an
upstream lightyear release skips replicon's raw channels in its message receive loop. Then run
the `rts::loopback` test; it must stay green without the patch.
