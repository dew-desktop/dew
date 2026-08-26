// Dew HUD Frontend Controller
const { invoke, event } = window.__TAURI__ ? window.__TAURI__.core : {
  invoke: async () => [],
  event: { listen: () => {} }
};

const hudPill = document.getElementById("hud-pill");
const timerDisplay = document.getElementById("timer-display");
const subtextDisplay = document.getElementById("subtext-display");
const widgetBadge = document.getElementById("widget-badge");
const btnToggle = document.getElementById("btn-toggle");
const iconPlayPause = document.getElementById("icon-play-pause");

let activeWidgetId = "timetracker-main";

function updateHUD(widgets) {
  if (!widgets || widgets.length === 0) return;
  const primary = widgets[0];
  activeWidgetId = primary.id;

  // Text & Timer
  timerDisplay.textContent = primary.text || "00:00";
  subtextDisplay.textContent = primary.subtext || primary.title || "";
  widgetBadge.textContent = primary.badge || "DEW";

  // Glow & State
  if (primary.glow) {
    hudPill.classList.add("is-running");
    iconPlayPause.innerHTML = '<rect x="6" y="4" width="4" height="16"></rect><rect x="14" y="4" width="4" height="16"></rect>';
  } else {
    hudPill.classList.remove("is-running");
    iconPlayPause.innerHTML = '<polygon points="5 3 19 12 5 21 5 3"></polygon>';
  }

  if (primary.color) {
    timerDisplay.style.color = primary.color;
  }
}

// Click Handlers
btnToggle.addEventListener("click", async (e) => {
  e.stopPropagation();
  try {
    await invoke("click_widget", { widgetId: activeWidgetId, secondary: false });
  } catch (err) {
    console.error("Failed to click widget:", err);
  }
});

hudPill.addEventListener("contextmenu", async (e) => {
  e.preventDefault();
  try {
    await invoke("click_widget", { widgetId: activeWidgetId, secondary: true });
  } catch (err) {
    console.error("Secondary click failed:", err);
  }
});

// Setup Tauri Event Listener
if (window.__TAURI__) {
  window.__TAURI__.event.listen("dew:update-widgets", (evt) => {
    updateHUD(evt.payload);
  });

  // Initial fetch
  invoke("get_widgets").then(updateHUD).catch(console.error);
}
