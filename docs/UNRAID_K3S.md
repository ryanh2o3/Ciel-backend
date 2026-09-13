# Unraid + k3s/k3d for polyglot Ciel APIs

Preferred host path: the [Unraid Kubernetes plugin](https://github.com/DonnieDice/unraid-kubernetes) (k3d in Docker). Alternative: Ubuntu VM with bare k3s.

**Images are built by GitHub Actions and pulled from GHCR** — do not compile Spring/.NET on Unraid.

| Image | Workflow | Tags |
|-------|----------|------|
| `ghcr.io/ryanh2o3/ciel-backend` | [`docker-publish.yml`](../.github/workflows/docker-publish.yml) | `:main`, `:latest`, `:sha-*` |
| `ghcr.io/ryanh2o3/ciel-api-spring` | [`docker-publish-spring.yml`](../.github/workflows/docker-publish-spring.yml) | same |
| `ghcr.io/ryanh2o3/ciel-api-dotnet` | [`docker-publish-dotnet.yml`](../.github/workflows/docker-publish-dotnet.yml) | same |

## Layout

```text
Phones → Cloudflare Tunnel → Traefik (k3d/k3s)
                              ├─ weighted: rust | spring | dotnet
                              └─ X-Ciel-Backend: rust|spring|dotnet (pin)
API pods → Docker/LAN → Postgres / Redis / MinIO / ElasticMQ (Compose)
Rust worker → same queue + S3 + Postgres
```

Data services stay in [`docker-compose.unraid.yml`](../docker-compose.unraid.yml). Do **not** publish DB/Redis/queue to the WAN.

## 1. Cluster on Unraid (plugin)

1. Install the plugin `.plg` from [DonnieDice/unraid-kubernetes](https://github.com/DonnieDice/unraid-kubernetes).
2. Create/start the k3d cluster from the Unraid **Kubernetes** page.
3. Do not edit k3d runtime containers via Docker → Edit.
4. Export kubeconfig from the plugin/appdata path (or use the host `kubectl` the plugin provides).

Alternative: Ubuntu VM + `curl -sfL https://get.k3s.io | sh -` (see older notes below).

## 2. GHCR access on the cluster

If packages are **public**, no pull secret is needed.

If **private**, on Unraid create a PAT with `read:packages`, then:

```bash
kubectl -n ciel create secret docker-registry ghcr-creds \
  --docker-server=ghcr.io \
  --docker-username=YOUR_GITHUB_USER \
  --docker-password=YOUR_PAT
```

Add to each Deployment (or a shared ServiceAccount):

```yaml
imagePullSecrets:
  - name: ghcr-creds
```

## 3. Data plane

Keep Compose for Postgres / Redis / MinIO / ElasticMQ / cloudflared. Put k3d and Compose on a **shared Docker network**, or publish those ports on the Unraid LAN IP. Point Secret URLs at hostnames reachable from pods.

## 4. Deploy (images already on GHCR)

Push to `main` (or **Actions → Run workflow**) so Spring/.NET images exist, then:

```bash
cp k8s/01-secret.example.yaml /tmp/ciel-secrets.yaml
# edit secrets to match Unraid .env (same PASETO keys)

kubectl apply -f k8s/00-namespace-data-plane.yaml
kubectl apply -f /tmp/ciel-secrets.yaml
kubectl apply -f k8s/10-rust.yaml
kubectl apply -f k8s/20-spring.yaml
kubectl apply -f k8s/30-dotnet.yaml
kubectl apply -f k8s/40-traefik-ingress.yaml
```

Update after a new `:main` publish:

```bash
kubectl -n ciel rollout restart deploy/ciel-api-rust deploy/ciel-api-spring deploy/ciel-api-dotnet deploy/ciel-worker-rust
```

## 5. Cloudflare

Point the API public hostname at the k3d **load-balancer** host port (see plugin Docker/Kubernetes UI), not compose `api:8080`. Media hostname stays on MinIO.

Stop Compose `api` / `worker`; keep one Rust worker in the cluster only.

## 6. Routing

| Request | Behavior |
|---------|----------|
| No special header | Weighted pool (50% Rust / 25% Spring / 25% .NET) |
| `X-Ciel-Backend: rust\|spring\|dotnet` | Pin that implementation |

Responses include `X-Ciel-Served-By`.

## Ownership

| Concern | Owner |
|---------|--------|
| Image builds | GitHub Actions → GHCR |
| Migrations | Compose migrate / Rust |
| Media worker | `ciel-worker-rust` only |
| Hourly cleanup | Rust API only |
| HTTP `/v1` | Any of the three APIs |

See [`CONTRACT.md`](CONTRACT.md).
