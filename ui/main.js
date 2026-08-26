// Dew HUD Frontend Controller (High-Performance In-Place Renderer)
const { invoke, event } = window.__TAURI__ ? window.__TAURI__.core : {
  invoke: async () => [],
  event: { listen: () => {} }
};

const hudPill = document.getElementById("hud-pill");
const widgetsContainer = document.getElementById("widgets-container");
const paletteModal = document.getElementById("palette-modal");
const paletteInput = document.getElementById("palette-input");

const widgetNodes = new Map();

function renderWidgets(widgets) {
  if (!widgets) return;

  let hasRunningGlow = false;

  widgets.forEach((widget, index) => {
    if (widget.glow) {
      hasRunningGlow = true;
    }

    let nodeData = widgetNodes.get(widget.id);

    if (!nodeData) {
      if (widgetNodes.size > 0) {
        const divider = document.createElement("div");
        divider.className = "dew-widget-divider";
        widgetsContainer.appendChild(divider);
      }

      const widgetEl = document.createElement("div");
      widgetEl.className = "dew-widget-item";
      widgetEl.dataset.id = widget.id;

      const dotEl = document.createElement("div");
      dotEl.className = "dew-brand-dot";

      const textGroup = document.createElement("div");
      textGroup.className = "dew-widget-text-group";

      const timerDisplay = document.createElement("div");
      timerDisplay.className = "dew-timer-display";

      const subtextDisplay = document.createElement("div");
      subtextDisplay.className = "dew-subtext-display";

      textGroup.appendChild(timerDisplay);
      textGroup.appendChild(subtextDisplay);

      const badgeEl = document.createElement("span");
      badgeEl.className = "dew-badge";

      widgetEl.appendChild(dotEl);
      widgetEl.appendChild(textGroup);
      widgetEl.appendChild(badgeEl);

      // Primary click
      widgetEl.addEventListener("click", async (e) => {
        e.stopPropagation();
        try {
          await invoke("click_widget", { widgetId: widget.id, secondary: false });
        } catch (err) {
          console.error("Widget click failed:", err);
        }
      });

      // Secondary click
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

      nodeData = {
        element: widgetEl,
        dot: dotEl,
        timer: timerDisplay,
        subtext: subtextDisplay,
        badge: badgeEl,
      };
      widgetNodes.set(widget.id, nodeData);
    }

    // Update text & colors in place (Zero DOM recreation!)
    nodeData.timer.textContent = widget.text || "";
    if (widget.color) {
      nodeData.timer.style.color = widget.color;
    }

    if (widget.subtext) {
      nodeData.subtext.textContent = widget.subtext;
      nodeData.subtext.style.display = "block";
    } else {
      nodeData.subtext.style.display = "none";
    }

    if (widget.badge) {
      nodeData.badge.textContent = widget.badge;
      nodeData.badge.style.display = "inline-block";
    } else {
      nodeData.badge.style.display = "none";
    }

    if (widget.glow !== undefined) {
      nodeData.dot.className = `dew-brand-dot ${widget.glow ? "glow-active" : ""}`;
      nodeData.dot.style.display = "block";
    } else {
      nodeData.dot.style.display = "none";
    }
  });

  if (hasRunningGlow) {
    hudPill.classList.add("is-running");
  } else {
    hudPill.classList.remove("is-running");
  }
}

// Command Palette Logic
function openPalette(placeholder) {
  paletteInput.placeholder = placeholder || "Type a query...";
  paletteInput.value = "";
  paletteModal.classList.remove("hidden");
  paletteInput.focus();
}

async function closePalette() {
  paletteModal.classList.add("hidden");
  paletteInput.blur();
  try {
    await invoke("set_hud_expanded", { expanded: false });
  } catch (err) {}
}

paletteInput.addEventListener("keydown", async (e) => {
  if (e.key === "Enter") {
    const val = paletteInput.value.trim();
    if (val.length > 0) {
      try {
        await invoke("submit_palette", { query: val });
      } catch (err) {
        console.error("Failed to submit palette query:", err);
      }
    }
    await closePalette();
  } else if (e.key === "Escape") {
    await closePalette();
  }
});

// Setup Tauri Event Listeners
if (window.__TAURI__) {
  window.__TAURI__.event.listen("dew:update-widgets", (evt) => {
    renderWidgets(evt.payload);
  });

  window.__TAURI__.event.listen("dew:open-palette", (evt) => {
    openPalette(evt.payload);
  });

  // Initial fetch
  invoke("get_widgets").then(renderWidgets).catch(console.error);
}
