// TASKFLOW Analytics & Productivity Visualizer
let analyticsReport = null;
let currentChartMode = "7d"; // "7d" or "30d"

document.addEventListener("DOMContentLoaded", async () => {
  try {
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
    if (adminLink && user.role === "admin") adminLink.style.display = "flex";

    await loadAnalyticsData();
  } catch (err) {
    console.error("Analytics init error:", err);
    window.location.href = "/login";
  }
});

async function loadAnalyticsData() {
  try {
    const res = await api.get("/api/v1/analytics/report");
    analyticsReport = res.data;

    // Render KPI Metrics
    document.getElementById("statTotal").textContent = analyticsReport.total_tasks;
    document.getElementById("statCompleted").textContent = analyticsReport.completed_tasks;
    document.getElementById("statCompletionRate").textContent = `${analyticsReport.completion_rate}%`;
    document.getElementById("statWeeklyVelocity").textContent = analyticsReport.weekly_completed_count;
    document.getElementById("statOverdue").textContent = analyticsReport.overdue_tasks;

    // Render Velocity SVG Chart
    renderVelocityChart();

    // Render Priority Breakdown
    renderPriorityDistribution(analyticsReport.priority_distribution);

    // Render Status Breakdown
    renderStatusDistribution(analyticsReport.status_distribution);

    // Render Project Workload
    renderProjectWorkload(analyticsReport.project_distribution);
  } catch (err) {
    showToast(err.message, "error");
  }
}

function setChartMode(mode) {
  currentChartMode = mode;
  document.getElementById("btnMode7d").classList.toggle("active", mode === "7d");
  document.getElementById("btnMode30d").classList.toggle("active", mode === "30d");
  renderVelocityChart();
}

// Pure Vanilla JS SVG Chart Generator (Zero External Dependencies)
function renderVelocityChart() {
  const container = document.getElementById("velocityChartContainer");
  if (!container || !analyticsReport) return;

  const data = currentChartMode === "7d"
    ? analyticsReport.weekly_velocity
    : analyticsReport.monthly_velocity;

  if (!data || data.length === 0) {
    container.innerHTML = `<p style="text-align: center; color: var(--text-muted); padding: 2rem;">No completion history available.</p>`;
    return;
  }

  const width = container.clientWidth || 700;
  const height = 260;
  const padding = { top: 20, right: 30, bottom: 40, left: 40 };

  const innerWidth = width - padding.left - padding.right;
  const innerHeight = height - padding.top - padding.bottom;

  const maxVal = Math.max(...data.map(d => d.count), 4);

  // Generate coordinates
  const points = data.map((d, i) => {
    const x = padding.left + (i / (data.length - 1 || 1)) * innerWidth;
    const y = padding.top + innerHeight - (d.count / maxVal) * innerHeight;
    return { x, y, ...d };
  });

  // Construct SVG Path
  const linePath = points.map((p, i) => `${i === 0 ? "M" : "L"} ${p.x.toFixed(1)} ${p.y.toFixed(1)}`).join(" ");
  const areaPath = `${linePath} L ${points[points.length - 1].x.toFixed(1)} ${padding.top + innerHeight} L ${points[0].x.toFixed(1)} ${padding.top + innerHeight} Z`;

  // Gridlines & Y-axis labels
  const yTicks = [0, Math.round(maxVal / 2), maxVal];
  const gridHtml = yTicks.map(val => {
    const y = padding.top + innerHeight - (val / maxVal) * innerHeight;
    return `
      <line x1="${padding.left}" y1="${y}" x2="${width - padding.right}" y2="${y}" stroke="var(--border-color)" stroke-dasharray="4,4" stroke-width="1"/>
      <text x="${padding.left - 8}" y="${y + 4}" fill="var(--text-muted)" font-size="11" text-anchor="end">${val}</text>
    `;
  }).join("");

  // X-axis labels (skip some if 30 days)
  const xLabels = points.map((p, i) => {
    if (data.length > 10 && i % 5 !== 0 && i !== data.length - 1) return "";
    return `
      <text x="${p.x}" y="${height - 12}" fill="var(--text-muted)" font-size="11" text-anchor="middle">${p.date}</text>
    `;
  }).join("");

  // Data point dots
  const dotsHtml = points.map(p => `
    <circle
      cx="${p.x}" cy="${p.y}" r="4"
      fill="var(--primary)" stroke="var(--bg-surface)" stroke-width="2"
      style="cursor: pointer; transition: r 150ms ease;"
      onmouseover="this.setAttribute('r', '6')"
      onmouseout="this.setAttribute('r', '4')"
    >
      <title>${p.date}: ${p.count} task${p.count === 1 ? '' : 's'} completed</title>
    </circle>
  `).join("");

  container.innerHTML = `
    <svg width="100%" height="${height}" viewBox="0 0 ${width} ${height}" style="overflow: visible;">
      <defs>
        <linearGradient id="areaGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--primary)" stop-opacity="0.35"/>
          <stop offset="100%" stop-color="var(--primary)" stop-opacity="0.0"/>
        </linearGradient>
      </defs>
      ${gridHtml}
      <path d="${areaPath}" fill="url(#areaGradient)"/>
      <path d="${linePath}" fill="none" stroke="var(--primary)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
      ${dotsHtml}
      ${xLabels}
    </svg>
  `;
}

function renderPriorityDistribution(items) {
  const container = document.getElementById("priorityDistContainer");
  if (!container || !items) return;

  const colors = {
    urgent: "var(--danger)",
    high: "var(--warning)",
    medium: "var(--info)",
    low: "#94a3b8"
  };

  container.innerHTML = items.map(item => `
    <div style="margin-bottom: 1rem;">
      <div style="display: flex; justify-content: space-between; font-size: 0.85rem; margin-bottom: 0.35rem;">
        <span style="font-weight: 600; text-transform: capitalize; color: var(--text-primary);">${item.label}</span>
        <span style="color: var(--text-secondary); font-weight: 500;">${item.count} tasks (${item.percentage}%)</span>
      </div>
      <div style="width: 100%; height: 8px; background-color: var(--border-subtle); border-radius: var(--radius-full); overflow: hidden;">
        <div style="width: ${item.percentage}%; height: 100%; background-color: ${colors[item.label] || 'var(--primary)'}; border-radius: var(--radius-full);"></div>
      </div>
    </div>
  `).join("");
}

function renderStatusDistribution(items) {
  const container = document.getElementById("statusDistContainer");
  if (!container || !items) return;

  const formatStatus = {
    todo: "To Do",
    in_progress: "In Progress",
    review: "Review",
    done: "Completed"
  };

  const colors = {
    todo: "#64748b",
    in_progress: "var(--primary)",
    review: "var(--warning)",
    done: "var(--success)"
  };

  container.innerHTML = items.map(item => `
    <div style="margin-bottom: 1rem;">
      <div style="display: flex; justify-content: space-between; font-size: 0.85rem; margin-bottom: 0.35rem;">
        <span style="font-weight: 600; color: var(--text-primary);">${formatStatus[item.label] || item.label}</span>
        <span style="color: var(--text-secondary); font-weight: 500;">${item.count} tasks (${item.percentage}%)</span>
      </div>
      <div style="width: 100%; height: 8px; background-color: var(--border-subtle); border-radius: var(--radius-full); overflow: hidden;">
        <div style="width: ${item.percentage}%; height: 100%; background-color: ${colors[item.label] || 'var(--primary)'}; border-radius: var(--radius-full);"></div>
      </div>
    </div>
  `).join("");
}

function renderProjectWorkload(items) {
  const container = document.getElementById("projectWorkloadContainer");
  if (!container) return;

  if (!items || items.length === 0) {
    container.innerHTML = `<p style="text-align: center; color: var(--text-muted); padding: 1.5rem;">No projects with assigned tasks.</p>`;
    return;
  }

  container.innerHTML = items.map(item => `
    <div style="margin-bottom: 1.25rem;">
      <div style="display: flex; justify-content: space-between; font-size: 0.85rem; margin-bottom: 0.35rem;">
        <div style="display: flex; align-items: center; gap: 0.5rem;">
          <div style="width: 10px; height: 10px; border-radius: 2px; background-color: ${item.color};"></div>
          <span style="font-weight: 600; color: var(--text-primary);">${escapeHtml(item.project_title)}</span>
        </div>
        <span style="color: var(--text-secondary); font-weight: 500;">${item.task_count} tasks (${item.percentage}%)</span>
      </div>
      <div style="width: 100%; height: 8px; background-color: var(--border-subtle); border-radius: var(--radius-full); overflow: hidden;">
        <div style="width: ${item.percentage}%; height: 100%; background-color: ${item.color}; border-radius: var(--radius-full);"></div>
      </div>
    </div>
  `).join("");
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
