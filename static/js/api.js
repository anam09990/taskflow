// Universal API Client Wrapper
const api = {
  async request(endpoint, options = {}) {
    const url = endpoint.startsWith("/") ? endpoint : `/api/v1/${endpoint}`;

    const defaultHeaders = {
      "Content-Type": "application/json",
      "Accept": "application/json",
    };

    const token = localStorage.getItem("taskflow_token");
    if (token) {
      defaultHeaders["Authorization"] = `Bearer ${token}`;
    }

    const config = {
      ...options,
      headers: {
        ...defaultHeaders,
        ...options.headers,
      },
    };

    if (config.body && typeof config.body === "object" && !(config.body instanceof FormData)) {
      config.body = JSON.stringify(config.body);
    }

    try {
      const response = await fetch(url, config);
      const data = await response.json().catch(() => null);

      if (!response.ok) {
        if (response.status === 401 && !window.location.pathname.startsWith("/login") && !window.location.pathname.startsWith("/register")) {
          localStorage.removeItem("taskflow_token");
          localStorage.removeItem("taskflow_user");
          window.location.href = "/login?expired=1";
          return Promise.reject(new Error("Session expired. Please log in again."));
        }

        const errorMessage = data?.error?.message || response.statusText || "An unexpected error occurred.";
        return Promise.reject(new Error(errorMessage));
      }

      return data;
    } catch (err) {
      return Promise.reject(err);
    }
  },

  get(endpoint, options = {}) {
    return this.request(endpoint, { ...options, method: "GET" });
  },

  post(endpoint, body, options = {}) {
    return this.request(endpoint, { ...options, method: "POST", body });
  },

  put(endpoint, body, options = {}) {
    return this.request(endpoint, { ...options, method: "PUT", body });
  },

  patch(endpoint, body, options = {}) {
    return this.request(endpoint, { ...options, method: "PATCH", body });
  },

  delete(endpoint, options = {}) {
    return this.request(endpoint, { ...options, method: "DELETE" });
  },
};

// Global User State Helpers
window.getCurrentUser = function () {
  try {
    const userStr = localStorage.getItem("taskflow_user");
    return userStr ? JSON.parse(userStr) : null;
  } catch (e) {
    return null;
  }
};

window.setCurrentUser = function (user, token) {
  if (user) localStorage.setItem("taskflow_user", JSON.stringify(user));
  if (token) localStorage.setItem("taskflow_token", token);
};

window.logout = async function () {
  try {
    await api.post("/api/v1/auth/logout");
  } catch (e) {
    // Ignore error on logout
  }
  localStorage.removeItem("taskflow_token");
  localStorage.removeItem("taskflow_user");
  window.location.href = "/login";
};
