// TASKFLOW Projects Controller
let currentProjects = [];
let editingProjectId = null;

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

    await loadProjects();

    const form = document.getElementById("projectForm");
    if (form) form.addEventListener("submit", handleProjectFormSubmit);
  } catch (err) {
    console.error("Projects init error:", err);
    window.location.href = "/login";
  }
});

async function loadProjects() {
  try {
    const res = await api.get("/api/v1/projects");
    currentProjects = res.data || [];
    renderProjects(currentProjects);
  } catch (err) {
    showToast(err.message, "error");
  }
}

function renderProjects(projects) {
  const container = document.getElementById("projectsGrid");
  const countBadge = document.getElementById("projectCountBadge");
  if (countBadge) countBadge.textContent = `${projects.length} project${projects.length === 1 ? '' : 's'}`;

  if (!container) return;

  if (projects.length === 0) {
    container.innerHTML = `
      <div style="grid-column: 1 / -1; text-align: center; padding: 4rem 1rem; background: var(--bg-surface); border: 1px dashed var(--border-color); border-radius: var(--radius-lg);">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="margin-bottom: 0.75rem; color: var(--text-muted);"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
        <h3 style="font-size: 1.15rem; font-weight: 600; color: var(--text-primary); margin-bottom: 0.25rem;">No projects yet</h3>
        <p style="font-size: 0.875rem; color: var(--text-secondary); margin-bottom: 1.5rem;">Group your tasks into projects to track goals and milestones.</p>
        <button class="btn btn-primary" onclick="openCreateProjectModal()">Create Your First Project</button>
      </div>
    `;
    return;
  }

  container.innerHTML = projects.map(project => {
    const total = project.total_tasks || 0;
    const completed = project.completed_tasks || 0;
    const percent = total > 0 ? Math.round((completed / total) * 100) : 0;

    const dueDate = project.due_date
      ? new Date(project.due_date).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
      : "No deadline";

    const color = project.color || "#3b82f6";

    return `
      <div class="card" style="margin-bottom: 0; display: flex; flex-direction: column; justify-content: space-between; border-top: 4px solid ${color};">
        <div class="card-header" style="border-bottom: none; padding-bottom: 0;">
          <div style="display: flex; align-items: center; gap: 0.5rem;">
            <div style="width: 12px; height: 12px; border-radius: 3px; background-color: ${color};"></div>
            <span class="badge" style="background-color: var(--border-subtle); color: var(--text-secondary); text-transform: uppercase; font-size: 0.7rem;">${project.status}</span>
          </div>
          <div style="display: flex; gap: 0.25rem;">
            <button class="btn-icon" onclick="openEditProjectModal(${project.id})" title="Edit Project">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
            </button>
            <button class="btn-icon" onclick="deleteProjectById(${project.id})" title="Delete Project" style="color: var(--danger);">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>
            </button>
          </div>
        </div>

        <div class="card-body">
          <h2 style="font-size: 1.15rem; font-weight: 700; color: var(--text-primary); margin-bottom: 0.5rem;">
            ${escapeHtml(project.title)}
          </h2>
          <p style="font-size: 0.85rem; color: var(--text-secondary); margin-bottom: 1.25rem; min-height: 2.5rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;">
            ${escapeHtml(project.description || "No description provided.")}
          </p>

          <!-- Progress Bar -->
          <div style="margin-bottom: 0.75rem;">
            <div style="display: flex; justify-content: space-between; font-size: 0.8rem; margin-bottom: 0.35rem;">
              <span style="color: var(--text-muted); font-weight: 500;">Progress</span>
              <span style="font-weight: 700; color: var(--text-primary);">${percent}%</span>
            </div>
            <div style="width: 100%; height: 8px; background-color: var(--border-subtle); border-radius: var(--radius-full); overflow: hidden;">
              <div style="width: ${percent}%; height: 100%; background-color: ${color}; transition: width 300ms ease;"></div>
            </div>
          </div>

          <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.8rem; color: var(--text-muted); padding-top: 0.75rem; border-top: 1px solid var(--border-subtle);">
            <span>${completed} of ${total} tasks</span>
            <span>📅 ${dueDate}</span>
          </div>
        </div>

        <div style="padding: 0.75rem 1.5rem; background-color: var(--border-subtle); border-top: 1px solid var(--border-color); display: flex; justify-content: flex-end;">
          <a href="/tasks" class="btn btn-secondary" style="padding: 0.35rem 0.75rem; font-size: 0.8rem;">
            View Tasks →
          </a>
        </div>
      </div>
    `;
  }).join("");
}

function selectColor(hex) {
  document.getElementById("projectColor").value = hex;
  document.querySelectorAll(".color-swatch").forEach(swatch => {
    swatch.style.outline = swatch.getAttribute("data-color") === hex ? "2px solid var(--text-primary)" : "none";
  });
}

function openCreateProjectModal() {
  editingProjectId = null;
  document.getElementById("projectModalTitle").textContent = "Create New Project";
  document.getElementById("projectForm").reset();
  selectColor("#3b82f6");
  document.getElementById("projectModalSubmitBtn").textContent = "Create Project";
  document.getElementById("projectModalBackdrop").classList.add("active");
}

async function openEditProjectModal(id) {
  try {
    const res = await api.get(`/api/v1/projects/${id}`);
    const project = res.data;
    editingProjectId = project.id;

    document.getElementById("projectModalTitle").textContent = "Edit Project";
    document.getElementById("projectTitle").value = project.title;
    document.getElementById("projectDescription").value = project.description || "";
    document.getElementById("projectStatus").value = project.status;
    selectColor(project.color || "#3b82f6");

    if (project.due_date) {
      const d = new Date(project.due_date);
      document.getElementById("projectDueDate").value = d.toISOString().split("T")[0];
    } else {
      document.getElementById("projectDueDate").value = "";
    }

    document.getElementById("projectModalSubmitBtn").textContent = "Save Changes";
    document.getElementById("projectModalBackdrop").classList.add("active");
  } catch (err) {
    showToast(err.message, "error");
  }
}

function closeProjectModal() {
  document.getElementById("projectModalBackdrop").classList.remove("active");
}

async function handleProjectFormSubmit(e) {
  e.preventDefault();
  const submitBtn = document.getElementById("projectModalSubmitBtn");
  submitBtn.disabled = true;

  const title = document.getElementById("projectTitle").value.trim();
  const description = document.getElementById("projectDescription").value.trim() || null;
  const color = document.getElementById("projectColor").value || "#3b82f6";
  const status = document.getElementById("projectStatus").value;
  const dueDateVal = document.getElementById("projectDueDate").value;
  const due_date = dueDateVal ? new Date(dueDateVal).toISOString() : null;

  const payload = { title, description, color, status, due_date };

  try {
    if (editingProjectId) {
      await api.put(`/api/v1/projects/${editingProjectId}`, payload);
      showToast("Project updated successfully.", "success");
    } else {
      await api.post("/api/v1/projects", payload);
      showToast("Project created successfully.", "success");
    }
    closeProjectModal();
    loadProjects();
  } catch (err) {
    showToast(err.message, "error");
  } finally {
    submitBtn.disabled = false;
  }
}

async function deleteProjectById(id) {
  const project = currentProjects.find(p => p.id === id);
  const title = project ? project.title : "this project";
  if (!confirm(`Are you sure you want to delete "${title}"? All assigned tasks will be preserved as unassigned.`)) {
    return;
  }

  try {
    await api.delete(`/api/v1/projects/${id}`);
    showToast("Project deleted.", "success");
    loadProjects();
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
