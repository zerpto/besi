# HTTP API

`app api` starts an HTTP server. The server itself lives in the core crate
(`besi`, feature `api`); the app only supplies its routes.

| Part | Owned by | Code |
|---|---|---|
| Server, `/health`, request logging, graceful shutdown, `api` config | core | [`crates/core/src/api/`](../crates/core/src/api/) |
| Endpoints (`/api/hello/{name}`, ...) | app | [`crates/app/src/api/`](../crates/app/src/api/): `handlers/`, `requests/`, `responses/`, `routes.rs` |

Stack: [axum](https://crates.io/crates/axum) 0.8 on
[tokio](https://crates.io/crates/tokio), with
[tower-http](https://crates.io/crates/tower-http) `TraceLayer` and
[tracing](https://crates.io/crates/tracing) for logs.

## Running

```sh
app api                         # 127.0.0.1:8080, or api.host / api.port from config
app api --port 9000             # flags override config
app api --host 0.0.0.0          # listen on all interfaces (e.g. in Docker)
RUST_LOG=debug app api          # more verbose logs
```

```sh
curl localhost:8080/health          # {"status":"ok"}
curl localhost:8080/api/hello/Ferris    # {"message":"Hello, Ferris!"}
```

Stop it with Ctrl+C or SIGTERM. The server stops accepting connections, lets
in-flight requests finish, then exits with code 0.

## Configuration

Section `api` (see [config.md](config.md) for layering):

| Key | Env | Flag | Default |
|---|---|---|---|
| `api.host` | `API__HOST` | `--host` | `127.0.0.1` |
| `api.port` | `API__PORT` | `--port` | `8080` |

Priority: flag > env var > `.env` > `config.yaml` > default.

## Health check

`GET /health` belongs to the core: the route, response format, status codes and
a timeout. The app only decides the outcome by overriding `ApiRoutes::health`.

| Outcome | Status | Body |
|---|---|---|
| `Health::Healthy` | 200 | `{"status":"ok"}` |
| `Health::unhealthy("reason")` | 503 | `{"status":"unhealthy","reason":"reason"}` |
| check takes longer than 2 s | 503 | `{"status":"unhealthy","reason":"health check timed out"}` |

The default implementation always returns healthy ("the process is up"). This
app overrides it: `AppRoutes::health` calls `handlers::health::check`
([`crates/app/src/api/handlers/health.rs`](../crates/app/src/api/handlers/health.rs)),
which reports unhealthy when the `hello` config section fails to load or
`hello.greeting` is empty, the two ways its endpoints break at runtime. Add
checks there as the app gains dependencies. The pattern, for a database:

```rust
impl ApiRoutes for AppRoutes {
    fn routes() -> Router<ApiState> { /* ... */ }

    async fn health(state: ApiState) -> Health {
        match ping_database(state.config()).await {
            Ok(()) => Health::Healthy,
            Err(err) => Health::unhealthy(format!("database: {err}")),
        }
    }
}
```

Notes:

- The reason goes into the response body and the log (`warn`). `/health` is
  usually reachable without auth, so don't put secrets, hostnames or stack
  traces in it.
- The 2 s timeout is applied by the core; a hung dependency can't hang the
  probe. Keep checks fast anyway: probes call this often.
- A failed check doesn't stop the server; it just answers 503 until the check
  passes again.
- For a separate "ready to serve traffic" check (Kubernetes readiness vs
  liveness), add an app route such as `/api/ready`; `/health` stays the liveness
  probe.

## Enabling and disabling

The API is a Cargo feature of `besi`, off by default. An app opts in by name:

```toml
# crates/app/Cargo.toml
besi = { workspace = true, features = ["api"] }   # API available
besi = { workspace = true }                        # no API; axum/tokio not compiled
```

With the feature on, the app still chooses whether to expose the command by
registering it in `crates/app/src/cmd/registry.rs`:

```rust
pub enum Commands {
    /// Start the HTTP API server
    Api(besi::ApiCommand<AppRoutes>),
}

// in the dispatch match, same as every other command
Self::Api(cmd) => cmd.run(ctx),
```

`AppRoutes` is the app's marker type implementing `besi::ApiRoutes`
(`crates/app/src/api/routes.rs`); it is how the app hands its routes to the
core.

To turn the API off: remove the variant and match arm, the `mod api;` line in
`main.rs` and the `crates/app/src/api/` folder, then drop `features = ["api"]`.

## Adding an endpoint

The app's `api/` folder is split by role, with one file per endpoint group in
each folder (named the same, e.g. `hello.rs`):

| Folder | Holds | Example |
|---|---|---|
| `handlers/` | async functions that do the work | `hello()` |
| `requests/` | types extracted from the request: path params, query, JSON body | `HelloPath` |
| `responses/` | types serialized into the response body | `Greeting` |
| `routes.rs` | the route table, all nested under `/api` | `/api/hello/{name}` -> `hello` |

To add `GET /api/items/{id}`:

1. `requests/items.rs`: the input type (skip if the handler needs none).

   ```rust
   #[derive(Debug, Deserialize)]
   pub struct ItemPath {
       pub id: u64,
   }
   ```

2. `responses/items.rs`: the output type.

   ```rust
   #[derive(Debug, Serialize)]
   pub struct Item {
       id: u64,
       name: String,
   }
   ```

3. `handlers/items.rs`: the handler.

   ```rust
   pub async fn show(
       State(state): State<ApiState>,
       Path(ItemPath { id }): Path<ItemPath>,
   ) -> Json<Item> {
       // state.config().section::<MyConfig>() for config
   }
   ```

4. Declare each new module and re-export its type in the folder's `mod.rs`:
   `handlers/mod.rs` gets `pub mod items;`, `requests/mod.rs` gets
   `mod items; pub use items::ItemPath;`, and `responses/mod.rs` the same for
   `Item`.

5. `routes.rs`: add the route to `api_routes()`. Paths there are relative to
   the `/api` prefix, so don't repeat it.

   ```rust
   Router::new()
       .route("/hello/{name}", get(handlers::hello::hello))
       .route("/items/{id}", get(handlers::items::show))
   ```

A JSON request body works the same way: a `#[derive(Deserialize)]` struct in
`requests/`, taken in the handler as `Json(body): Json<CreateItem>`.

Notes:

- Use axum through the core's re-export, `besi::axum::...`, so the app always
  matches the core's axum version. The app has no direct axum dependency.
- Path parameters use axum 0.8 syntax, `/{name}` (not `/:name`).
- Every app route is served under `/api` (set in `AppRoutes::routes()`). The
  core's `/health` stays at the root, outside the prefix.
- Don't define `/health` as a route; the core owns it, and overlapping routes
  panic at startup. To change what "healthy" means, override
  `ApiRoutes::health` (see [Health check](#health-check)).
- Handlers reach the database through `State<ApiState>`: `state.db()?.pool()`
  (feature `db`; see [database.md](database.md)). Don't return database error
  text to clients.
- Handlers read config through `State<ApiState>`:
  `state.config().section::<T>()`.
- There's no shared error type for handlers yet. Return
  `Result<T, (StatusCode, String)>` or anything else that implements
  `IntoResponse`.

## Testing endpoints

Build the app's router with an empty config and call it in memory, with no
network and no port:

```rust
use besi::axum::body::{Body, to_bytes};
use besi::axum::http::Request;
use tower::ServiceExt; // dev-dependency

#[tokio::test]
async fn greets_by_name() {
    let app = AppRoutes::routes().with_state(besi::ApiState::new(besi::Config::default()));
    let response = app
        .oneshot(Request::get("/api/hello/Ferris").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], br#"{"message":"Hello, Ferris!"}"#);
}
```

The core tests `/health` (default, overridden, timeout), route mounting and
404s the same way in `crates/core/src/api/server.rs`. The router builder is
private to the core, so an app tests its health override by calling it
directly; it is a plain async function (see the tests in `handlers/health.rs`):

```rust
#[tokio::test]
async fn unhealthy_without_database() {
    let state = besi::ApiState::new(besi::Config::default());
    assert!(matches!(AppRoutes::health(state).await, besi::Health::Unhealthy(_)));
}
```

## Logging

One line per request, to **stderr** (stdout stays free for command output):

```
INFO request{method=GET uri=/api/hello/Ferris version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=0 ms status=200
```

The level comes from `RUST_LOG` (default `info`). Use it to filter, e.g.
`RUST_LOG=warn` or `RUST_LOG=besi=debug,info`. Logging is set up only when
`app api` starts; other commands don't install a logger.

## Core internals

| File | Contains |
|---|---|
| `mod.rs` | re-exports `ApiCommand`, `ApiConfig`, `ApiRoutes`, `ApiState`, `Health` |
| `command.rs` | `ApiCommand<R>`: `--host` / `--port` flags; `Command::run` builds `ApiState` (config + db pool) and awaits the server |
| `routes.rs` | `ApiRoutes` trait: what the app supplies (`routes()`, optional `health()`) |
| `settings.rs` | `ApiConfig` section (`api`) |
| `state.rs` | `ApiState`: `Arc<Config>`, cloned into every handler |
| `health.rs` | `Health` result type and the `/health` handler (status codes, timeout) |
| `server.rs` | `router::<R>()`: `/health` + `R::routes()` + TraceLayer; `serve()`: bind, serve, graceful shutdown |

Logging setup and the Ctrl+C / SIGTERM signal live in
[`crates/core/src/daemon/`](../crates/core/src/daemon/), shared with `cron`.

Design notes:

- **`ApiCommand<R>` is a plain `Command`.** It needs the app's routes, which
  `run(ctx)` can't receive as an argument, so they arrive as a type parameter:
  the core defines the `ApiRoutes` trait, the app implements it on a marker
  type (`AppRoutes`), and `R::routes()` is called inside `run`. This keeps the
  core route-agnostic and the dispatch uniform. See "Command contract" in
  `CLAUDE.md`.
- **One runtime.** `Command::run` is async; `besi::run` owns the process's
  only tokio runtime and `ApiCommand::run` just awaits the server on it.
- **stdout lock.** The runner passes `io::stdout()` to `Context`, not
  `io::stdout().lock()`. Holding the lock for the whole run deadlocked the
  server: tokio worker threads block forever on any stdout write. Don't
  reintroduce a long-held stdout lock.
- `make lint` also runs clippy on `besi` with `--no-default-features`, so code
  that only compiles with `api` on is caught.
