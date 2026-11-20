# Remaining parity work, ranked

Generated from docs/parity-matrix.tsv. Open means not present and not not-applicable: present-candidate, blocked and missing. A row needs a witness before it can be present; a row can only get one if bevy has a counterpart that does the same thing. Groups follow the spoint areas named in the request; rows outside them are in other.

| Rank | Group | Open | Present | Candidate (no witness) | Blocked | Missing | Not applicable | Witnessable now | Blocker |
|---|---|---|---|---|---|---|---|---|---|
| 1 | planet | 107 | 1 | 0 | 9 | 98 | 4 | 0 | globe: different tangent basis, no surface solve, no streamer budgets, no sampler parity |
| 2 | client | 102 | 0 | 0 | 6 | 96 | 129 | 0 | browser app vs native Lightyear client; no app-module loader; projection contract differs |
| 3 | mapspinner | 34 | 0 | 0 | 2 | 32 | 37 | 0 | no planet renderer in bevy; defaults differ (radius 64 m against 6360000 m) |
| 4 | netcode | 39 | 0 | 0 | 1 | 38 | 1 | 0 | no lockstep, consensus, desync, cheat or replay code in bevy |
| 5 | protocol | 12 | 0 | 0 | 1 | 11 | 0 | 0 | no msgpack wire or input codec in bevy |
| 6 | static | 9 | 0 | 0 | 3 | 6 | 0 | 0 | partial counterpart: GLB decode and basis transcode; rows 561 to 568 need a transform stage, an encoder crate or a consumer that bevy does not have |
| 7 | perf-run | 2 | 0 | 0 | 0 | 2 | 0 | 0 | partial counterpart: bench and render stats, no percentile runner |
| 8 | animation | 11 | 0 | 0 | 0 | 11 | 0 | 0 | no counterpart |
| 9 | sharding | 11 | 0 | 0 | 0 | 11 | 0 | 0 | no counterpart |
| 10 | grass | 16 | 0 | 0 | 0 | 16 | 1 | 0 | no counterpart |
| 11 | occlusion | 7 | 0 | 0 | 0 | 7 | 5 | 0 | no counterpart |
| 12 | weather | 7 | 0 | 0 | 0 | 7 | 2 | 0 | no counterpart |
| 13 | groundy | 1 | 0 | 0 | 0 | 1 | 0 | 0 | no counterpart |
| - | other (outside the list) | 441 | 0 | 14 | 0 | 427 | 90 | not re-read in this change | not ranked |

Totals: open 799, present 1, candidate-only 14, blocked 22.

Witnessable now: 0 in every group. The one present row (280, vegetation placement) is already witnessed. No candidate in the requested groups survives reading against spoint, so Phase A writes no new witness.
