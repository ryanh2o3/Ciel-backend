# Unraid + k3s for polyglot Ciel APIs

This repo keeps **one data plane** on Unraid Compose and runs **API pods** (Rust / Spring / .NET) plus the **Rust media worker** in a single-node **k3s** VM.

## Layout

```text
Phones → Cloudflare Tunnel → Traefik (k3s)
                              ├─ weighted: rust | spring | dotnet
                              └─ X-Ciel-Backend: rust|spring|dotnet (pin)
API pods → Unraid LAN → Postgres / Redis / MinIO / ElasticMQ (Compose)
Rust worker → same queue + S3 + Postgres
```

Data services stay in [`docker-compose.unraid.yml`](../docker-compose.unraid.yml). Do **not** publish DB/Redis/queue to the WAN; only expose them on the Unraid LAN (or a Docker network the k3s VM can reach).

## 1. Unraid VM for k3s

1. Unraid → VMs → create Ubuntu 22.04/24.04 (2+ vCPU, 4+ GB RAM, bridged NIC on LAN).
2. SSH in and install k3s:

```bash
curl -sfL https://get.k3s.io | sh -
sudo kubectl get nodes
```

3. Copy kubeconfig to your laptop if desired: `/etc/rancher/k3s/k3s.yaml` (replace `127.0.0.1` with the VM IP).

Unraid’s host OS is a poor place for bare-metal k3s (no systemd). Prefer the VM.

## 2. Expose Compose data plane to the VM

Either:

- Publish Postgres `5432`, Redis `6379`, MinIO `9000`, ElasticMQ `9324` on the Unraid **LAN IP only**, or
- Put Compose on a user-defined network and route to the VM.

Update [`00-namespace-data-plane.yaml`](../k8s/00-namespace-data-plane.yaml): replace `DATA_PLANE_HOST.local` with the Unraid hostname/IP, **or** skip ExternalName and put host IPs directly in the Secret `DATABASE_URL` / `REDIS_URL` / etc.

When migrating API off Compose, point Cloudflare Tunnel’s `home-api` hostname at the k3s Traefik NodePort (see `ciel-api-ingress`) or Traefik’s host port instead of `http://api:8080`.

## 3. Deploy

```bash
# From this repo on a machine with kubectl → the k3s cluster
cp k8s/01-secret.example.yaml /tmp/ciel-secrets.yaml
# edit secrets
kubectl apply -f k8s/00-namespace-data-plane.yaml
kubectl apply -f /tmp/ciel-secrets.yaml
kubectl apply -f k8s/10-rust.yaml
kubectl apply -f k8s/20-spring.yaml   # after building/pushing ciel-api-spring:local
kubectl apply -f k8s/30-dotnet.yaml   # after building/pushing ciel-api-dotnet:local
kubectl apply -f k8s/40-traefik-ingress.yaml
```

Build local images on the k3s node (or push to a registry the node can pull):

```bash
docker build -t ciel-api-spring:local ./spring
docker build -t ciel-api-dotnet:local ./dotnet
# import into k3s if needed: k3s ctr images import …
```

## 4. Routing behaviour

| Request | Behavior |
|---------|----------|
| No special header | Weighted pool (default 50% Rust / 25% Spring / 25% .NET) |
| `X-Ciel-Backend: rust` | Always Rust |
| `X-Ciel-Backend: spring` | Always Spring |
| `X-Ciel-Backend: dotnet` | Always .NET |

Responses include `X-Ciel-Served-By: rust|spring|dotnet` for learning/debugging. Normal client apps ignore both headers.

## 5. Ownership (do not break)

| Concern | Owner |
|---------|--------|
| Migrations | Rust migrate job / Compose migrate |
| Media worker | `ciel-worker-rust` only |
| Hourly cleanup | Rust API only |
| HTTP `/v1` | Any of the three APIs |

See [`CONTRACT.md`](CONTRACT.md).
