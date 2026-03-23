---
title: iOS app
---

Ciel Social **iOS** is a native **SwiftUI** application structured in layers.

---

## Layout (typical)

```text
Ciel-ios/Ciel-ios/
├── App/           # AppContainer (DI), root views, tabs
├── Features/      # Per-feature Views + ViewModels
├── Domain/        # Entities and use cases
├── Data/          # Repositories, DTOs, FeedCache
├── Core/          # APIClient, Keychain token store, safety helpers
└── UI/            # Shared components and design system
```

---

## Data flow

- **Repositories** implement interfaces consumed by use cases.
- **APIClient** performs HTTP calls against the Ciel base URL.
- **KeychainTokenStore** (or equivalent) persists access/refresh tokens; interceptors refresh on 401 when supported.

---

## Design choices

- **Zero third-party Swift packages** — UIKit/SwiftUI, Combine, Foundation, Security only.
- **Async/await** with `@MainActor` on view models where appropriate.

---

## Related docs

- [Auth API](/docs/api-auth/) — login, refresh, revoke.
- [Media API](/docs/api-media/) — upload and completion flow.
