# @fiducial/realtime

## 0.2.0

### Minor Changes

- 489a2ee: Durable Objects backend, its client, and a local relay that speaks the same protocol.
  
  `RealtimeDO` is the server half: one Durable Object per channel, holding
  WebSocket sessions and fanning out broadcast and presence frames.
  `connectChannel` is the client half. Both were written months ago on a branch
  and never merged.
  
  `createRelay({ port, host })` is the reason to care. It speaks the *same* JSON
  frame protocol as the Durable Object — broadcast and presence:track/untrack in,
  broadcast and presence:join/leave/sync out — over a plain WebSocket server, with
  channels multiplexed on one port by `?channel=`. So the same client code runs
  against Cloudflare in production and against `npx @fiducial/realtime` on a
  laptop, a LAN, or a field gateway with no internet.
  
  That last case is the point for products with hardware in the field: an edge
  runtime is not available at a mesh gateway, and a realtime layer that only
  exists in someone else's cloud is one that stops at the network boundary.
