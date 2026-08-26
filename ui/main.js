// Dew HUD Frontend Controller (Multi-Widget & Command Palette)
const { invoke, event } = window.__TAURI__ ? window.__TAURI__.core : {
  invoke: async () => [],
  event: { listen: () => {} }
};

const hudPill = document.getElementById("hud-pill");
const widgetsContainer = document.getElementById("widgets-container");
const paletteModal = document.getElementById("palette-modal");
const paletteInput = document.getElementById("palette-input");

let activeWidgets = [];

function renderWidgets(widgets) {
  if (!widgets) return;
  activeWidgets = widgets;

  let hasRunningGlow = false;
  widgetsContainer.innerHTML = "";

  widgets.forEach((widget, index) => {
    if (index > 0) {
      const divider = document.createElement("div");
      divider.className = "dew-widget-divider";
      widgetsContainer.appendChild(divider);
    }

    const widgetEl = document.createElement("div");
    widgetEl.className = "dew-widget-item";
    widgetEl.dataset.id = widget.id;

    if (widget.glow) {
      hasRunningGlow = true;
    }

    const dotHtml = widget.glow !== undefined
      ? `<div class="dew-brand-dot ${widget.glow ? 'glow-active' : ''}"></div>`
      : '';

    const badgeHtml = widget.badge
      ? `<span class="dew-badge">${widget.badge}</span>`
      : '';

    const colorStyle = widget.color ? `style="color: ${widget.color}"` : '';

    widgetEl.innerHTML = `
      ${dotHtml}
      <div class="dew-widget-text-group">
        <div class="dew-timer-display" ${colorStyle}>${widget.text || ''}</div>
        ${widget.subtext ? `<div class="dew-subtext-display">${widget.subtext}</div>` : ''}
      </div>
      ${badgeHtml}
    `;

    // Primary Click
    widgetEl.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await invoke("click_widget", { widgetId: widget.id, secondary: false });
      } catch (err) {
        console.error("Widget click failed:", err);
      }
    });

    // Secondary Click
    widgetEl.addEventListener("contextmenu", async (e) => {
      e.preventDefault();
      e.stopPropagation();
      try {
        await invoke("click_widget", { widgetId: widget.id, secondary: true });
      } catch (err) {
        console.error("Widget secondary click failed:", err);
      }
    });

    widgetsContainer.appendChild(widgetEl);
  });

  if (hasRunningGlow) {
    hudPill.classList.add("is-running");
  } else {
    hudPill.classList.remove("is-running");
  }
}

// Command Palette Management
function openPalette(placeholder) {
  paletteInput.placeholder = placeholder || "Type a query or task...";
  paletteInput.value = "";
  paletteModal.classList.remove("hidden");
  paletteInput.focus();
}

function closePalette() {
  paletteModal.classList.add("hidden");
  paletteInput.blur();
}

paletteInput.addEventListener("keydown", async (e) => {
  if (e.key === "Enter") {
    const val = paletteInput.value.trim();
    if (val.length > 0) {
      try {
        await invoke("submit_palette", { query: val });
      } catch (err) {
        console.error("Failed to submit palette:", err);
      }
    }
    closePalette();
  } else if (e.key === "Escape") {
    closePalette();
  }
});

// Setup Tauri Event Listener
if (window.__TAURI__) {
  window.__TAURI__.event.listen("dew:update-widgets", (evt) => {
    renderWidgets(evt.payload);
  });

  // Check palette status periodically
  setInterval(async () => {
    try {
      const status = await invoke("get_palette_status");
      if (status && status[0] && paletteModal.classList.contains("hidden")) {
        openPalette(status[1]);
      }
    } catch (err) {}
  }, 200);

  // Initial fetch
  invoke("get_widgets").then(renderWidgets).catch(console.error);
}
