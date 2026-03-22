---
title: Android app
---

PicShare **Android** uses **Jetpack Compose** and **Clean Architecture**-style modules.

---

## Layout (typical)

```text
app/src/main/java/com/picshare/android/
├── core/          # Hilt modules, Retrofit/OkHttp, Room, auth
├── data/          # Repository implementations, API models, mappers
├── domain/        # Use cases, models, repository interfaces
├── presentation/  # Screens, ViewModels, navigation, theme
└── util/
```

---

## Networking

- **Retrofit + Moshi** for REST DTOs.
- **AuthInterceptor** and **TokenRefreshAuthenticator** attach Bearer tokens and recover from expiration.

---

## Local data

- **Room** for caching where product requirements need offline or fast repeat reads.

---

## Related docs

- [Auth API](/docs/api-auth/)
- [Media API](/docs/api-media/)
