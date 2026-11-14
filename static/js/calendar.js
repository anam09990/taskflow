// TASKFLOW Calendar Controller
let currentDate = new Date();
let viewYear = currentDate.getFullYear();
let viewMonth = currentDate.getMonth(); // 0-11
let userProjects = [];
let currentCalendarEvents = [];

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
    await renderCalendar();

    const taskForm = document.getElementById("calendarTaskForm");
    if (taskForm) taskForm.addEventListener("submit", handleCalendarTaskSubmit);
  } catch (err) {
    console.error("Calendar init error:", err);
    window.location.href = "/login";
  }
});

async function loadProjects() {
  try {
    const res = await api.get("/api/v1/projects");
    userProjects = res.data || [];
    const select = document.getElementById("modalTaskProject");
    if (select) {
      select.innerHTML = `<option value="">No Project (General)</option>` +
        userProjects.map(p => `<option value="${p.id}">${escapeHtml(p.title)}</option>`).join("");
    }
  } catch (e) {
    userProjects = [];
  }
}

async function renderCalendar() {
  const monthNames = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"
  ];

  document.getElementById("calendarMonthTitle").textContent = `${monthNames[viewMonth]} ${viewYear}`;

  const grid = document.getElementById("calendarGrid");
  if (!grid) return;

  // Calculate start day and number of days
  const firstDayIndex = new Date(viewYear, viewMonth, 1).getDay(); // 0 is Sunday
  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const daysInPrevMonth = new Date(viewYear, viewMonth, 0).getDate();

  // Fetch events for this month from API
  let events = [];
  try {
    const res = await api.get(`/api/v1/calendar/events?year=${viewYear}&month=${viewMonth + 1}`);
    events = res.data || [];
  } catch (err) {
    showToast(err.message, "error");
  }
  currentCalendarEvents = events;

  // Group events by day of the month
  const eventsByDay = {};
  events.forEach(evt => {
    const d = new Date(evt.due_date);
    if (d.getFullYear() === viewYear && d.getMonth() === viewMonth) {
      const day = d.getDate();
      if (!eventsByDay[day]) eventsByDay[day] = [];
      eventsByDay[day].push(evt);
    }
  });

  const today = new Date();
  const isCurrentMonthView = today.getFullYear() === viewYear && today.getMonth() === viewMonth;
  const todayDate = today.getDate();

  let html = "";

  // 1. Previous month padding days
  for (let i = firstDayIndex - 1; i >= 0; i--) {
    const prevDay = daysInPrevMonth - i;
    html += `
      <div class="calendar-day other-month">
        <span class="day-number">${prevDay}</span>
      </div>
    `;
  }

  // 2. Current month days
  for (let day = 1; day <= daysInMonth; day++) {
    const isToday = isCurrentMonthView && day === todayDate;
    const dayEvents = eventsByDay[day] || [];
    const dateStr = `${viewYear}-${String(viewMonth + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;

    const eventsHtml = dayEvents.slice(0, 3).map(evt => {
      const color = evt.project_color || (evt.event_type === "project" ? "#10b981" : "#3b82f6");
      const isDone = evt.status === "done";
      const style = isDone ? "opacity: 0.55; text-decoration: line-through;" : "";

      return `
        <div
          class="calendar-event-pill"
          style="border-left: 3px solid ${color}; ${style}"
          title="${escapeHtml(evt.title)} (${evt.status})"
          onclick="event.stopPropagation(); viewEventById(${evt.id})"
        >
          <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(evt.title)}</span>
        </div>
      `;
    }).join("");

    const moreCount = dayEvents.length > 3 ? `<div style="font-size: 0.7rem; color: var(--text-muted); padding-left: 4px;">+${dayEvents.length - 3} more</div>` : "";

    html += `
      <div class="calendar-day current-month ${isToday ? 'today' : ''}" onclick="openCreateForDate('${dateStr}')">
        <div class="day-header">
          <span class="day-number ${isToday ? 'today-pill' : ''}">${day}</span>
          ${isToday ? '<span class="today-label">TODAY</span>' : ''}
        </div>
        <div class="day-events">
          ${eventsHtml}
          ${moreCount}
        </div>
      </div>
    `;
  }

  // 3. Next month padding days to complete grid (total multiple of 7)
  const totalCells = firstDayIndex + daysInMonth;
  const nextMonthDays = (7 - (totalCells % 7)) % 7;
  for (let i = 1; i <= nextMonthDays; i++) {
    html += `
      <div class="calendar-day other-month">
        <span class="day-number">${i}</span>
      </div>
    `;
  }

  grid.innerHTML = html;
}

function prevMonth() {
  viewMonth--;
  if (viewMonth < 0) {
    viewMonth = 11;
    viewYear--;
  }
  renderCalendar();
}

function nextMonth() {
  viewMonth++;
  if (viewMonth > 11) {
    viewMonth = 0;
    viewYear++;
  }
  renderCalendar();
}

function goToToday() {
  const now = new Date();
  viewYear = now.getFullYear();
  viewMonth = now.getMonth();
  renderCalendar();
}

function openCreateForDate(dateStr) {
  document.getElementById("modalTaskDueDate").value = dateStr;
  document.getElementById("calendarTaskForm").reset();
  document.getElementById("modalTaskDueDate").value = dateStr;
  document.getElementById("calendarModalBackdrop").classList.add("active");
}

function closeCalendarModal() {
  document.getElementById("calendarModalBackdrop").classList.remove("active");
}

function viewEventById(id) {
  const evt = currentCalendarEvents.find(e => e.id === id);
  if (!evt) return;
  const dateStr = evt.due_date ? new Date(evt.due_date).toLocaleDateString() : 'N/A';
  alert(`Event: ${evt.title}\nStatus: ${evt.status}\nPriority: ${evt.priority || 'medium'}\nDue: ${dateStr}`);
}

async function handleCalendarTaskSubmit(e) {
  e.preventDefault();
  const submitBtn = document.getElementById("calendarModalSubmitBtn");
  submitBtn.disabled = true;

  const title = document.getElementById("modalTaskTitle").value.trim();
  const projectVal = document.getElementById("modalTaskProject").value;
  const project_id = projectVal ? parseInt(projectVal, 10) : null;
  const priority = document.getElementById("modalTaskPriority").value;
  const dueDateVal = document.getElementById("modalTaskDueDate").value;
  const due_date = dueDateVal ? new Date(dueDateVal).toISOString() : null;

  try {
    await api.post("/api/v1/tasks", {
      title,
      project_id,
      priority,
      status: "todo",
      due_date,
    });
    showToast("Task created on calendar.", "success");
    closeCalendarModal();
    renderCalendar();
  } catch (err) {
    showToast(err.message, "error");
  } finally {
    submitBtn.disabled = false;
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
