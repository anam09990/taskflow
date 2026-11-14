// TASKFLOW Admin Panel Controller
let currentUser = null;
let allUsers = [];

document.addEventListener("DOMContentLoaded", async () => {
  try {
    const meRes = await api.get("/api/v1/auth/me");
    currentUser = meRes.data;
    setCurrentUser(currentUser);

    if (currentUser.role !== "admin") {
      showToast("Access denied. Admin privileges required.", "error");
      setTimeout(() => {
        window.location.href = "/dashboard";
      }, 500);
      return;
    }

    const userNameEl = document.getElementById("headerUserName");
    const userRoleEl = document.getElementById("headerUserRole");
    const userAvatarEl = document.getElementById("headerUserAvatar");
    const adminLink = document.getElementById("adminNavLink");

    if (userNameEl) userNameEl.textContent = currentUser.username;
    if (userRoleEl) userRoleEl.textContent = currentUser.role;
    if (userAvatarEl) userAvatarEl.textContent = currentUser.username.substring(0, 2).toUpperCase();
    if (adminLink) adminLink.style.display = "flex";

    await loadAdminData();
  } catch (err) {
    console.error("Admin init error:", err);
    window.location.href = "/login";
  }
});

async function loadAdminData() {
  try {
    // 1. Load Platform Stats
    const statsRes = await api.get("/api/v1/admin/stats");
    const stats = statsRes.data;

    document.getElementById("statAdminUsers").textContent = stats.total_users;
    document.getElementById("statAdminActiveUsers").textContent = stats.active_users;
    document.getElementById("statAdminSessions").textContent = stats.active_sessions;
    document.getElementById("statAdminTasks").textContent = stats.total_tasks;

    // 2. Load Users
    const usersRes = await api.get("/api/v1/admin/users");
    allUsers = usersRes.data || [];
    renderUsersTable(allUsers);
  } catch (err) {
    showToast(err.message, "error");
  }
}

function renderUsersTable(users) {
  const tbody = document.getElementById("adminUsersTableBody");
  const countBadge = document.getElementById("adminUserCountBadge");
  if (countBadge) countBadge.textContent = `${users.length} accounts`;

  if (!tbody) return;

  if (users.length === 0) {
    tbody.innerHTML = `
      <tr>
        <td colspan="6" style="text-align: center; padding: 2rem; color: var(--text-muted);">
          No users registered.
        </td>
      </tr>
    `;
    return;
  }

  tbody.innerHTML = users.map(user => {
    const isSelf = user.id === currentUser.id;
    const createdAt = new Date(user.created_at).toLocaleDateString(undefined, {
      month: 'short', day: 'numeric', year: 'numeric'
    });

    const statusBadge = user.is_active
      ? `<span class="badge" style="background-color: var(--success-light); color: var(--success);">Active</span>`
      : `<span class="badge" style="background-color: var(--danger-light); color: var(--danger);">Deactivated</span>`;

    const roleBadge = user.role === "admin"
      ? `<span class="badge" style="background-color: rgba(168, 85, 247, 0.15); color: #a855f7;">Admin</span>`
      : `<span class="badge" style="background-color: var(--primary-light); color: var(--primary);">User</span>`;

    const nextRole = user.role === "admin" ? "user" : "admin";
    const nextStatus = !user.is_active;

    const actionButtons = isSelf
      ? `<span style="font-size: 0.8rem; color: var(--text-muted); font-style: italic;">Current Session</span>`
      : `
        <div style="display: flex; gap: 0.5rem; justify-content: flex-end;">
          <button
            class="btn btn-secondary"
            style="padding: 0.3rem 0.65rem; font-size: 0.75rem;"
            onclick="toggleUserStatusById(${user.id}, ${nextStatus})"
          >
            ${user.is_active ? 'Lock' : 'Unlock'}
          </button>
          <button
            class="btn btn-secondary"
            style="padding: 0.3rem 0.65rem; font-size: 0.75rem;"
            onclick="changeUserRoleById(${user.id}, '${nextRole}')"
          >
            Make ${nextRole}
          </button>
        </div>
      `;

    return `
      <tr>
        <td>
          <div style="font-weight: 600; color: var(--text-primary);">${escapeHtml(user.username)}</div>
          <div style="font-size: 0.8rem; color: var(--text-muted);">${escapeHtml(user.email)}</div>
        </td>
        <td>${roleBadge}</td>
        <td>${statusBadge}</td>
        <td style="font-size: 0.85rem; color: var(--text-secondary);">
          ${user.task_count} tasks · ${user.project_count} projects
        </td>
        <td style="font-size: 0.85rem; color: var(--text-muted);">${createdAt}</td>
        <td style="text-align: right;">${actionButtons}</td>
      </tr>
    `;
  }).join("");
}

async function toggleUserStatusById(userId, is_active) {
  const user = allUsers.find(u => u.id === userId);
  const username = user ? user.username : "this user";
  const actionName = is_active ? "reactivate" : "deactivate and lock";
  if (!confirm(`Are you sure you want to ${actionName} account "${username}"?`)) {
    return;
  }

  try {
    await api.patch(`/api/v1/admin/users/${userId}/status`, { is_active });
    showToast(`Account "${username}" status updated.`, "success");
    loadAdminData();
  } catch (err) {
    showToast(err.message, "error");
  }
}

async function changeUserRoleById(userId, role) {
  const user = allUsers.find(u => u.id === userId);
  const username = user ? user.username : "this user";
  if (!confirm(`Are you sure you want to change "${username}" to role ${role}?`)) {
    return;
  }

  try {
    await api.patch(`/api/v1/admin/users/${userId}/role`, { role });
    showToast(`Role for "${username}" updated to ${role}.`, "success");
    loadAdminData();
  } catch (err) {
    showToast(err.message, "error");
  }
}

function escapeHtml(str) {
  if (!str) return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
