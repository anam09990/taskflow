// TASKFLOW Dashboard Controller
document.addEventListener("DOMContentLoaded", async () => {
  try {
    // 1. Verify Authentication & Update Profile UI
    const meRes = await api.get("/api/v1/auth/me");
    const user = meRes.data;
    setCurrentUser(user);

    const userNameEl = document.getElementById("headerUserName");
    const userRoleEl = document.getElementById("headerUserRole");
    const userAvatarEl = document.getElementById("headerUserAvatar");
    const adminLink = document.getElementById("adminNavLink");

    if (userNameEl) userNameEl.textContent = user.username;
    if (userRoleEl) userRoleEl.textContent = user.role;
    if (userAvatarEl) userAvatarEl.textContent = user.username.substring(0, 2).toUpperCase();

    // Show admin link if user has admin role
    if (adminLink && user.role === "admin") {
      adminLink.style.display = "flex";
    }

    // 2. Fetch Dashboard Overview Data
    await loadDashboardData();
  } catch (err) {
    console.error("Dashboard initialization error:", err);
    window.location.href = "/login";
  }
});

async function loadDashboardData() {
  try {
    const res = await api.get("/api/v1/analytics/overview");
    const overview = res.data;

    // Render KPI Metrics
    document.getElementById("statTotalTasks").textContent = overview.total_tasks;
    document.getElementById("statCompletedTasks").textContent = overview.completed_tasks;
    document.getElementById("statPendingTasks").textContent = overview.pending_tasks;
    document.getElementById("statProductivity").textContent = `${overview.completion_rate}%`;

    // Render Upcoming Deadlines
    renderUpcomingDeadlines(overview.upcoming_deadlines);

    // Render Recent Activity
    renderRecentActivity(overview.recent_activity);
  } catch (err) {
    showToast(err.message, "error");
  }
}

function renderUpcomingDeadlines(deadlines) {
  const container = document.getElementById("upcomingDeadlinesList");
  if (!container) return;

  if (!deadlines || deadlines.length === 0) {
    container.innerHTML = `
      <tr>
        <td colspan="4" style="text-align: center; padding: 2rem; color: var(--text-muted);">
          No upcoming deadlines. You're all caught up!
        </td>
      </tr>
    `;
    return;
  }

  container.innerHTML = deadlines
    .map((task) => {
      const dueDate = task.due_date ? new Date(task.due_date).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' }) : 'No Date';
      const projectBadge = task.project_title
        ? `<span class="badge" style="background-color: ${task.project_color || '#3b82f6'}20; color: ${task.project_color || '#3b82f6'};">${escapeHtml(task.project_title)}</span>`
        : `<span style="color: var(--text-muted); font-size: 0.8rem;">None</span>`;

      return `
        <tr>
          <td>
            <div style="font-weight: 600;">${escapeHtml(task.title)}</div>
          </td>
          <td>${projectBadge}</td>
          <td><span class="badge badge-${task.priority}">${task.priority}</span></td>
          <td>
            <span style="font-size: 0.85rem; color: var(--text-secondary);">${dueDate}</span>
          </td>
        </tr>
      `;
    })
    .join("");
}

function renderRecentActivity(activities) {
  const container = document.getElementById("recentActivityList");
  if (!container) return;

  if (!activities || activities.length === 0) {
    container.innerHTML = `
      <div style="text-align: center; padding: 2rem; color: var(--text-muted);">
        No recent activity recorded.
      </div>
    `;
    return;
  }

  container.innerHTML = activities
    .map((item) => {
      const timeStr = new Date(item.created_at).toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });

      return `
        <div class="activity-item">
          <div class="activity-dot"></div>
          <div>
            <div style="font-weight: 500; color: var(--text-primary);">
              ${escapeHtml(item.details || item.action)}
            </div>
            <div class="activity-time">${timeStr}</div>
          </div>
        </div>
      `;
    })
    .join("");
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
