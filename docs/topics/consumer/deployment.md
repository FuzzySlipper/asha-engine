---
status: current
audience: consumer
tags: [consumer, deployment, setup, repos]
supersedes: []
see-also: [consumer-compatibility.md, repo-family-deployment.md]
---

# Repo Family Deployment

Cold-start map for agents or users setting up the ASHA repo family from scratch.

## Repositories

| Repo | Local path | Owns | Does not own |
|---|---|---|---|
| `FuzzySlipper/asha-engine` | `/home/dev/asha-engine` | Rust authority, generated contracts, public TypeScript package surfaces, runtime bridge, render projection, governance, local guardrails, and provider regressions | Product/demo content or downstream acceptance |
| `FuzzySlipper/asha-demo` | `/home/dev/asha-demo` | Human-facing playable/demo content and visible acceptance built through public ASHA surfaces | Synthetic conformance identity or private engine imports |
| `FuzzySlipper/asha-studio` | `/home/dev/asha-studio` | Studio/editor UI, command composition, authoring workflows, and visual/debug read models over public ASHA surfaces | Rust authority, raw runtime/native transports, private ASHA internals |
| `FuzzySlipper/asha-testing` | `/home/dev/asha-testing` | Focused synthetic public-surface regressions and strict package/path boundary negatives | Product/demo identity, visible acceptance, or engine feature implementation |

## Fresh Checkout

```sh
mkdir -p /home/dev
cd /home/dev
git clone git@github.com:FuzzySlipper/asha-engine.git asha-engine
git clone git@github.com:FuzzySlipper/asha-demo.git asha-demo
git clone git@github.com:FuzzySlipper/asha-studio.git asha-studio
git clone git@github.com:FuzzySlipper/asha-testing.git asha-testing
```

See `topics/consumer/repo-family-deployment.md` for full setup and check commands.
