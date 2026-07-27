# animation-sampling

`animation-sampling` is the small, source-neutral animation math kernel. It
owns checked signed sample time, finite repeated timing, fill behavior, exact
keyframe offsets, easing, and deterministic scalar-curve sampling.

Scalar keyframes are checked finite values. Their constructors reject NaN and
positive or negative infinity before a curve can exist, so exact interpolation
never receives a non-finite endpoint.

It deliberately does not own authored animation programs, source formats,
property targets, typed color/path/transform curves, effect composition,
playback, clocks, I/O, layout, or rendering. Producers keep those concerns and
pass only checked timing contributions into this crate.

The public types make invalid timing positions unrepresentable. A sample is
either absent (`Timing::contribution` returns `None`) or a checked opaque
`Contribution`; curve sampling cannot manufacture an out-of-range progress
fraction.

The `internal` module is a temporary, hidden bridge for producer-owned typed
curves that still share the exact arithmetic kernel. It is breakable and is
not an SDK surface. It should shrink as typed vocabulary finds its final seat.
