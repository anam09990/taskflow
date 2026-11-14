# TASKFLOW — Modern Full-Stack Productivity & Task Management Platform

TASKFLOW is a high-performance, single-binary productivity and task management platform engineered with **Rust**, **Axum**, **SQLx**, **SQLite**, and a zero-dependency **HTML5/CSS3/Vanilla JavaScript** frontend.

Designed from the ground up to run effortlessly on resource-constrained hardware (including 8GB RAM Windows machines), TASKFLOW compiles into a single, highly optimized native executable (~4.4 MB) with minimal memory footprint (< 15 MB RSS) and zero external Node.js/npm dependencies.

---

## ⚡ Key Highlights & Architecture

- **Ultra-Fast & Lean Native Engine:** Built on Tokio and Axum 0.8 with Link-Time Optimization (LTO) and symbol stripping.
- **Embedded Database with WAL:** SQLite with Write-Ahead Logging (`PRAGMA journal_mode=WAL`), busy timeouts, foreign key enforcement, and automated compile-time migrations.
- **Enterprise-Grade Security:**
  - Argon2id password hashing with constant-time dummy verification to eliminate user enumeration.
  - Session tokens stored with 64-character UUIDs and `HttpOnly`, `SameSite=Lax`, and conditional `Secure` cookie policies.
  - Strict Content-Security-Policy (CSP), Permissions-Policy, X-Frame-Options (`DENY`), and X-Content-Type-Options (`nosniff`).
  - 100% parameterized SQLx queries (`?` bindings) with zero string concatenation.
  - Front-end XSS prevention using safe DOM ID lookups and HTML entity escaping.
- **Zero-Dependency SaaS UI:**
  - Semantic HTML5 with modern CSS variables (Light and Dark mode support).
  - SVG-based charts rendered client-side without heavy charting libraries.
  - Mobile-responsive sidebar drawer, modals, notifications, and real-time client state.
- **First-User Superadmin Bootstrap:**
  - The first user to register on the platform is automatically designated as `admin`.
  - Subsequent users default to `user`.
  - Self-lockout and self-demotion protection guards in place.

---

## 📁 System Architecture & Directory Structure

```text
J:\RUST\
├── .env.example                # Template for environment configurations
├── .env                        # Local runtime environment configuration
├── Cargo.toml                  # Dependencies & LTO release optimization profile
├── migrations/                 # Compile-time SQLite database migrations
│   └── 20261113000001_initial_schema.sql
├── src/
│   ├── main.rs                 # Server entry point & graceful shutdown listener
│   ├── lib.rs                  # Library entry point exporting modules for tests
│   ├── config/                 # Environment and application configuration loader
│   ├── database/               # SQLite pool management & automated migration runner
│   ├── errors/                 # Unified AppError enum & Axum IntoResponse mapper
│   ├── middleware/             # Auth extractor, RBAC guards, and security headers
│   ├── models/                 # Database entity structs (User, Task, Project, etc.)
│   ├── routes/                 # Sub-routers for Web, REST API, static files, 404
│   ├── handlers/               # HTTP request handlers
│   └── services/               # Business logic, analytics calculations, password hashing
├── static/
│   ├── css/                    # Variables (themes), layout, and component stylesheets
│   └── js/                     # API client, theme manager, toasts, and page controllers
├── templates/                  # Semantic HTML5 views (dashboard, tasks, projects, etc.)
└── tests/                      # 11 integration and security test suites (32 total tests)
```

---

## 🛠️ Prerequisites & Installation

### Prerequisites
- **Operating System:** Windows 10/11 (64-bit), Linux, or macOS.
- **Rust Toolchain:** Rust 1.85+ (`rustc` and `cargo`).
- **C Compiler (Windows):** Visual Studio 2022 C++ Build Tools or Community Edition (for `libsqlite3-sys`).

### Configuration
Copy `.env.example` to `.env` and configure your settings:

```bash
cp .env.example .env
```

| Variable | Default | Description |
| :--- | :--- | :--- |
| `SERVER_HOST` | `127.0.0.1` | Network interface IP to bind |
| `SERVER_PORT` | `3000` | Port to listen for incoming HTTP traffic |
| `DATABASE_URL` | `sqlite://taskflow.db` | SQLite connection string (WAL mode) |
| `SESSION_SECRET` | `32+ character random string` | Secret used for session signing/integrity |
| `SESSION_MAX_AGE_SECS` | `86400` | Session lifetime in seconds (default: 24h) |
| `ENVIRONMENT` | `development` | Set to `production` to enforce `Secure` cookies |
| `RUST_LOG` | `taskflow=info,tower_http=info` | Tracing log filter levels |

---

## 🚀 Running the Application

### 1. Development Mode
Run with auto-recompilation and debug logging:
```powershell
cargo run
```
Open your browser to `http://localhost:3000`.

### 2. Optimized Production Release
Build the release binary with Link-Time Optimization (LTO):
```powershell
cargo build --release
```
The resulting executable is generated at:
```text
target/release/taskflow.exe
```

Run the production executable:
```powershell
.\target\release\taskflow.exe
```

---

## 🧪 Automated Test Suites

TASKFLOW features 11 integration test suites covering unit logic, database migrations, security barriers, and end-to-end HTTP route handling:

```powershell
cargo test
```

### Test Coverage Summary (32 Tests):
1. `src/lib.rs` (1 test): Configuration defaults and env parsing.
2. `tests/server_routing_tests.rs` (4 tests): Health check, static asset serving, web template rendering, global 404 fallback.
3. `tests/database_tests.rs` (3 tests): WAL schema migrations, unique constraints, foreign key cascades.
4. `tests/auth_tests.rs` (2 tests): User registration, Argon2id hashing, session token cookies & Bearer extraction.
5. `tests/dashboard_tests.rs` (2 tests): Overview metrics aggregation, unauthorized route protection.
6. `tests/task_tests.rs` (3 tests): Task CRUD lifecycle, priority/status filtering, user authorization isolation.
7. `tests/project_tests.rs` (3 tests): Project CRUD, completion progress calculation, task retention upon project deletion.
8. `tests/calendar_tests.rs` (2 tests): Monthly event filtering, cross-user privacy boundaries.
9. `tests/analytics_tests.rs` (2 tests): Productivity velocity series, workload distribution, user isolation.
10. `tests/admin_tests.rs` (4 tests): RBAC protection, platform statistics, account locking, session invalidation, self-protection guards.
11. `tests/security_tests.rs` (6 tests): SQL injection defense, XSS payload handling & boundary checks, account enumeration defense, privilege escalation defense, session tampering & expiration enforcement, security headers verification.

---

## 🌐 API Reference

All REST endpoints are prefixed with `/api/v1` and accept/return JSON.

### Authentication
- `POST /api/v1/auth/register` — Register a new account (`username`, `email`, `password`).
- `POST /api/v1/auth/login` — Authenticate and receive session cookie/token (`username_or_email`, `password`).
- `POST /api/v1/auth/logout` — Invalidate current session and clear session cookie.
- `GET /api/v1/auth/me` — Get details of current authenticated user.

### Tasks
- `GET /api/v1/tasks` — List tasks with filters (`status`, `priority`, `project_id`, `search`).
- `POST /api/v1/tasks` — Create task (`title`, `description`, `project_id`, `priority`, `status`, `due_date`).
- `GET /api/v1/tasks/:id` — Get single task by ID.
- `PUT /api/v1/tasks/:id` — Update full task record.
- `PATCH /api/v1/tasks/:id/status` — Quick-toggle task status (`todo`, `in_progress`, `review`, `done`).
- `DELETE /api/v1/tasks/:id` — Delete task.

### Projects
- `GET /api/v1/projects` — List user projects with aggregated progress and task counts.
- `POST /api/v1/projects` — Create project (`title`, `description`, `color`, `status`, `due_date`).
- `GET /api/v1/projects/:id` — Get project details with associated tasks.
- `PUT /api/v1/projects/:id` — Update project.
- `DELETE /api/v1/projects/:id` — Delete project (unlinks associated tasks safely).

### Calendar
- `GET /api/v1/calendar/events?year=YYYY&month=MM` — Fetch all deadline markers for month.

### Analytics
- `GET /api/v1/analytics/overview` — Dashboard summary metrics, upcoming deadlines, activity logs.
- `GET /api/v1/analytics/report` — 7-day & 30-day velocity charts, priority distribution, workload percentages.

### Admin Panel (Admin Role Required)
- `GET /api/v1/admin/stats` — Global platform metrics (users, active sessions, tasks, projects).
- `GET /api/v1/admin/users` — Detailed accounts list with task counts and statuses.
- `PATCH /api/v1/admin/users/:id/status` — Lock or unlock account (wipes sessions upon lockout).
- `PATCH /api/v1/admin/users/:id/role` — Promote/demote role (`admin` or `user`).

---

## 🪟 Windows 11 Production Deployment Guide

To run TASKFLOW as a persistent background service on Windows 11 that automatically starts on system boot and recovers from crashes:

### Option A: Using NSSM (Non-Sucking Service Manager) — Recommended
1. Download **NSSM** from `https://nssm.cc/download`.
2. Extract `nssm.exe` (64-bit) to a permanent folder (e.g. `C:\Tools\nssm\`).
3. Open PowerShell as Administrator and run:
   ```powershell
   nssm install TaskflowService "J:\RUST\target\release\taskflow.exe"
   nssm set TaskflowService AppDirectory "J:\RUST"
   nssm set TaskflowService Start SERVICE_AUTO_START
   nssm set TaskflowService AppStdout "J:\RUST\service_stdout.log"
   nssm set TaskflowService AppStderr "J:\RUST\service_stderr.log"
   nssm start TaskflowService
   ```
4. Verify service status:
   ```powershell
   Get-Service TaskflowService
   ```

### Option B: Using Windows Task Scheduler
If third-party service managers are not permitted:
1. Open PowerShell as Administrator.
2. Register a startup task that runs hidden in the background:
   ```powershell
   $Action = New-ScheduledTaskAction -Execute "J:\RUST\target\release\taskflow.exe" -WorkingDirectory "J:\RUST"
   $Trigger = New-ScheduledTaskTrigger -AtStartup
   $Principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
   $Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
   Register-ScheduledTask -TaskName "TaskflowBackgroundService" -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings
   ```
3. Start the task immediately:
   ```powershell
   Start-ScheduledTask -TaskName "TaskflowBackgroundService"
   ```

---

## 🔒 Security Best Practices for Production

1. **Change Default Secrets:** Always generate a strong, unique `SESSION_SECRET` (at least 32 random alphanumeric characters) in your production `.env`.
2. **Reverse Proxy & TLS/HTTPS:** Place TASKFLOW behind a reverse proxy (e.g. Caddy, Nginx, or Cloudflare Tunnel) to terminate HTTPS. When `ENVIRONMENT=production` is set, TASKFLOW automatically enforces `Secure` cookies over HTTPS.
3. **Database Backups:** Because SQLite runs in WAL mode, create live backups without stopping the service using SQLite's online backup API or command line:
   ```powershell
   sqlite3 taskflow.db ".backup 'taskflow_backup_$(Get-Date -Format yyyyMMdd).db'"
   ```

---

## 📄 License
This project is licensed under the MIT License.
