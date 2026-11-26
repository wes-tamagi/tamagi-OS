// Tamagi-SKL guard: max 1 normal browser window and max 2 tabs per window.
const MAX_TABS = 2;

async function enforceWindows() {
  try {
    const wins = await chrome.windows.getAll({ windowTypes: ["normal"] });
    if (wins.length <= 1) return;
    const primary = wins[0];
    for (let i = 1; i < wins.length; i++) {
      await chrome.windows.remove(wins[i].id).catch(() => {});
    }
    if (primary && primary.id !== undefined) {
      await chrome.windows.update(primary.id, { focused: true }).catch(() => {});
    }
  } catch (_) {}
}

async function enforceTabs(windowId) {
  try {
    const tabs = await chrome.tabs.query({ windowId });
    if (tabs.length <= MAX_TABS) return;
    const excess = tabs.slice(MAX_TABS);
    for (const tab of excess) {
      if (tab.id !== undefined) {
        await chrome.tabs.remove(tab.id).catch(() => {});
      }
    }
  } catch (_) {}
}

chrome.windows.onCreated.addListener((win) => {
  if (win.type === "normal") {
    enforceWindows();
  }
});

chrome.tabs.onCreated.addListener((tab) => {
  if (tab.windowId !== undefined) {
    enforceTabs(tab.windowId);
  }
});

chrome.runtime.onStartup.addListener(enforceWindows);
chrome.runtime.onInstalled.addListener(enforceWindows);
