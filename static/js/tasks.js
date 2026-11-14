// TASKFLOW Tasks Controller
let currentTasks = [];
let userProjects = [];
let activeStatus = "all";
let activePriority = "all";
let activeProjectId = "all";
let currentSearch = "";
let editingTaskId = null;

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
    if (adminLink && user.role === "admin") adminLink.style.display = "flex";

    // 2. Fetch Projects to populate dropdowns
    await loadProjects();

    // 3. Fetch Tasks
    await loadTasks();

    // 4. Attach Event Listeners
    setupEventListeners();
  } catch (err) {
    console.error("Tasks init error:", err);
    window.location.href = "/login";
  }
});

async function loadProjects() {
  try {
    const res = await api.get("/api/v1/projects");
    userProjects = res.data || [];
    populateProjectDropdowns();
  } catch (e) {
    userProjects = [];
  }
}

function populateProjectDropdowns() {
  const filterSelect = document.getElementById("filterProjectSelect");
  const modalSelect = document.getElementById("taskProject");

  if (filterSelect) {
    filterSelect.innerHTML = `<option value="all">All Projects</option>` +
      userProjects.map(p => `<option value="${p.id}">${escapeHtml(p.title)}</option>`).join("");
  }

  if (modalSelect) {
    modalSelect.innerHTML = `<option value="">No Project (General)</option>` +
      userProjects.map(p => `<option value="${p.id}">${escapeHtml(p.title)}</option>`).join("");
  }
}

async function loadTasks() {
  try {
    const params = new URLSearchParams();
    if (activeStatus !== "all") params.append("status", activeStatus);
    if (activePriority !== "all") params.append("priority", activePriority);
    if (activeProjectId !== "all") params.append("project_id", activeProjectId);
    if (currentSearch.trim()) params.append("search", currentSearch.trim());

    const url = `/api/v1/tasks?${params.toString()}`;
    const res = await api.get(url);
    currentTasks = res.data || [];
    renderTasksList(currentTasks);
  } catch (err) {
    showToast(err.message, "error");
  }
}

function renderTasksList(tasks) {
  const tbody = document.getElementById("tasksTableBody");
  const countBadge = document.getElementById("taskCountBadge");
  if (countBadge) countBadge.textContent = `${tasks.length} task${tasks.length === 1 ? '' : 's'}`;

  if (!tbody) return;

  if (tasks.length === 0) {
    tbody.innerHTML = `
      <tr>
        <td colspan="7" style="text-align: center; padding: 3rem 1rem; color: var(--text-muted);">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="margin-bottom: 0.5rem; opacity: 0.5;"><polyline points="9 11 12 14 22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/></svg>
          <p style="font-weight: 500;">No tasks found.</p>
          <p style="font-size: 0.85rem; margin-top: 0.25rem;">Create a new task or adjust your search filters.</p>
        </td>
      </tr>
    `;
    return;
  }

  tbody.innerHTML = tasks.map(task => {
    const dueDate = task.due_date
      ? new Date(task.due_date).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
      : '<span style="color: var(--text-muted);">No date</span>';

    const projectTag = task.project_title
      ? `<span class="badge" style="background-color: ${task.project_color || '#3b82f6'}20; color: ${task.project_color || '#3b82f6'};">${escapeHtml(task.project_title)}</span>`
      : `<span style="color: var(--text-muted); font-size: 0.8rem;">—</span>`;

    const isDone = task.status === "done";
    const titleStyle = isDone ? "text-decoration: line-through; opacity: 0.65;" : "";

    return `
      <tr>
        <td style="width: 40px; text-align: center;">
          <input
            type="checkbox"
            ${isDone ? "checked" : ""}
            onchange="toggleTaskDone(${task.id}, this.checked)"
            style="cursor: pointer; width: 16px; height: 16px; accent-color: var(--primary);"
            title="Mark as done"
          />
        </td>
        <td>
          <div style="font-weight: 600; font-size: 0.95rem; ${titleStyle}">${escapeHtml(task.title)}</div>
          ${task.description ? `<div style="font-size: 0.8rem; color: var(--text-secondary); margin-top: 0.2rem;">${escapeHtml(task.description)}</div>` : ""}
        </td>
        <td>${projectTag}</td>
        <td>
          <span class="badge badge-${task.priority}">${task.priority}</span>
        </td>
        <td>
          <select
            class="form-control"
            style="padding: 0.25rem 0.5rem; font-size: 0.8rem; width: auto; min-width: 110px;"
            onchange="updateTaskStatus(${task.id}, this.value)"
          >
            <option value="todo" ${task.status === "todo" ? "selected" : ""}>To Do</option>
            <option value="in_progress" ${task.status === "in_progress" ? "selected" : ""}>In Progress</option>
            <option value="review" ${task.status === "review" ? "selected" : ""}>Review</option>
            <option value="done" ${task.status === "done" ? "selected" : ""}>Done</option>
          </select>
        </td>
        <td style="font-size: 0.85rem; color: var(--text-secondary);">${dueDate}</td>
        <td style="text-align: right; white-space: nowrap;">
          <button class="btn-icon" onclick="openEditTaskModal(${task.id})" title="Edit Task" style="margin-right: 0.25rem;">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          <button class="btn-icon" onclick="deleteTaskById(${task.id})" title="Delete Task" style="color: var(--danger);">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>
          </button>
        </td>
      </tr>
    `;
  }).join("");
}

function setupEventListeners() {
  // Search input with debounce
  const searchInput = document.getElementById("taskSearchInput");
  let debounceTimeout = null;
  if (searchInput) {
    searchInput.addEventListener("input", (e) => {
      clearTimeout(debounceTimeout);
      debounceTimeout = setTimeout(() => {
        currentSearch = e.target.value;
        loadTasks();
      }, 250);
    });
  }

  // Priority filter select
  const prioritySelect = document.getElementById("filterPrioritySelect");
  if (prioritySelect) {
    prioritySelect.addEventListener("change", (e) => {
      activePriority = e.target.value;
      loadTasks();
    });
  }

  // Project filter select
  const projectSelect = document.getElementById("filterProjectSelect");
  if (projectSelect) {
    projectSelect.addEventListener("change", (e) => {
      activeProjectId = e.target.value;
      loadTasks();
    });
  }

  // Status Tab Pills
  const statusTabs = document.querySelectorAll(".status-tab-btn");
  statusTabs.forEach(tab => {
    tab.addEventListener("click", () => {
      statusTabs.forEach(t => t.classList.remove("active"));
      tab.classList.add("active");
      activeStatus = tab.getAttribute("data-status");
      loadTasks();
    });
  });

  // Task Form Submit
  const taskForm = document.getElementById("taskForm");
  if (taskForm) {
    taskForm.addEventListener("submit", handleTaskFormSubmit);
  }
}

async function updateTaskStatus(id, newStatus) {
  try {
    await api.patch(`/api/v1/tasks/${id}/status`, { status: newStatus });
    showToast("Status updated.", "success");
    loadTasks();
  } catch (err) {
    showToast(err.message, "error");
    loadTasks();
  }
}

async function toggleTaskDone(id, isChecked) {
  const newStatus = isChecked ? "done" : "todo";
  await updateTaskStatus(id, newStatus);
}

function openCreateTaskModal() {
  editingTaskId = null;
  document.getElementById("taskModalTitle").textContent = "Create New Task";
  document.getElementById("taskForm").reset();
  document.getElementById("taskModalSubmitBtn").textContent = "Create Task";
  document.getElementById("taskModalBackdrop").classList.add("active");
}

async function openEditTaskModal(id) {
  try {
    const res = await api.get(`/api/v1/tasks/${id}`);
    const task = res.data;
    editingTaskId = task.id;

    document.getElementById("taskModalTitle").textContent = "Edit Task";
    document.getElementById("taskTitle").value = task.title;
    document.getElementById("taskDescription").value = task.description || "";
    document.getElementById("taskProject").value = task.project_id || "";
    document.getElementById("taskPriority").value = task.priority;
    document.getElementById("taskStatus").value = task.status;

    if (task.due_date) {
      const d = new Date(task.due_date);
      document.getElementById("taskDueDate").value = d.toISOString().split("T")[0];
    } else {
      document.getElementById("taskDueDate").value = "";
    }

    document.getElementById("taskModalSubmitBtn").textContent = "Save Changes";
    document.getElementById("taskModalBackdrop").classList.add("active");
  } catch (err) {
    showToast(err.message, "error");
  }
}

function closeTaskModal() {
  document.getElementById("taskModalBackdrop").classList.remove("active");
}

async function handleTaskFormSubmit(e) {
  e.preventDefault();
  const submitBtn = document.getElementById("taskModalSubmitBtn");
  submitBtn.disabled = true;

  const title = document.getElementById("taskTitle").value.trim();
  const description = document.getElementById("taskDescription").value.trim() || null;
  const projectVal = document.getElementById("taskProject").value;
  const project_id = projectVal ? parseInt(projectVal, 10) : null;
  const priority = document.getElementById("taskPriority").value;
  const status = document.getElementById("taskStatus").value;
  const dueDateVal = document.getElementById("taskDueDate").value;
  const due_date = dueDateVal ? new Date(dueDateVal).toISOString() : null;

  const payload = {
    title,
    description,
    project_id,
    priority,
    status,
    due_date,
  };

  try {
    if (editingTaskId) {
      await api.put(`/api/v1/tasks/${editingTaskId}`, payload);
      showToast("Task updated successfully.", "success");
    } else {
      await api.post("/api/v1/tasks", payload);
      showToast("Task created successfully.", "success");
    }
    closeTaskModal();
    loadTasks();
  } catch (err) {
    showToast(err.message, "error");
  } finally {
    submitBtn.disabled = false;
  }
}

async function deleteTaskById(id) {
  const task = currentTasks.find(t => t.id === id);
  const title = task ? task.title : "this task";
  if (!confirm(`Are you sure you want to delete "${title}"? This action cannot be undone.`)) {
    return;
  }

  try {
    await api.delete(`/api/v1/tasks/${id}`);
    showToast("Task deleted.", "success");
    loadTasks();
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
