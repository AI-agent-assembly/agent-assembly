# aa-runtime docker-compose example

Demonstrates running `aa-runtime` as a sidecar alongside an agent container,
connected via a shared Unix domain socket.

## Prerequisites

- Docker and Docker Compose v2
- An Agent Assembly API key (`AA_API_KEY`)

## Running the example

```bash
AA_API_KEY=<your-key> docker compose up
```

`aa-runtime` will start, expose the IPC socket at `/tmp/aa-runtime-my-agent-001.sock`,
and serve health/metrics at `http://localhost:8080`.

## Agent placeholder

> **Python SDK not yet available.** The `agent-stub` service is an `alpine` placeholder.
> Replace it with your agent image once the Python SDK (`aa-sdk`) is published
> (tracked in AAASM-55). The socket mount and `AA_AGENT_ID` env var must be
> preserved in your replacement.

To swap in your own agent:

1. Replace the `agent-stub` service's `image:` with your agent image
   (or use `build: ./your-agent` to build locally).
2. Keep `AA_AGENT_ID` identical in both `aa-runtime` and your agent service.
3. Keep the `aa-runtime-socket` volume mount at `/tmp` — the IPC socket lives at
   `/tmp/aa-runtime-<AA_AGENT_ID>.sock`.

## Policy enforcement (optional)

Uncomment the policy volume mount in `docker-compose.yml` to load a policy file:

```yaml
volumes:
  - ./policy.toml:/etc/aa/policy.toml:ro
```

See `../policy.toml` for an example policy.

## Health check

```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/metrics
```
