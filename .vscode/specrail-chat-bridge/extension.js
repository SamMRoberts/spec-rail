const vscode = require("vscode");

async function delay(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

function extractPrompt(payload) {
  if (!payload) return "";
  if (Array.isArray(payload) && payload.length > 0) {
    const first = payload[0];
    if (first && typeof first.prompt === "string") return first.prompt;
    return "";
  }
  if (typeof payload.prompt === "string") return payload.prompt;
  return "";
}

function extractMode(payload) {
  if (!payload) return "agent";
  if (Array.isArray(payload) && payload.length > 0) {
    const first = payload[0];
    if (first && typeof first.mode === "string") return first.mode;
    return "agent";
  }
  if (typeof payload.mode === "string") return payload.mode;
  return "agent";
}

function extractAutoSubmit(payload) {
  if (!payload) return true;
  if (Array.isArray(payload) && payload.length > 0) {
    const first = payload[0];
    if (first && typeof first.autoSubmit === "boolean") return first.autoSubmit;
    return true;
  }
  if (typeof payload.autoSubmit === "boolean") return payload.autoSubmit;
  return true;
}

async function submitChatIfPossible() {
  const available = new Set(await vscode.commands.getCommands(true));
  const candidates = ["workbench.action.chat.submit", "chat.action.submit"];

  for (const command of candidates) {
    if (!available.has(command)) {
      continue;
    }

    try {
      await vscode.commands.executeCommand(command);
      return command;
    } catch {
      // Ignore and continue trying other candidates.
    }
  }

  return undefined;
}

function activate(context) {
  const disposable = vscode.commands.registerCommand("specrail.runMcpAction", async (payload) => {
    const prompt = extractPrompt(payload);
    const mode = extractMode(payload);
    const autoSubmit = extractAutoSubmit(payload);
    await vscode.commands.executeCommand("workbench.action.chat.open", {
      query: prompt,
      isPartialQuery: false,
      mode,
    });

    if (autoSubmit) {
      await delay(75);
      await submitChatIfPossible();
    }
  });

  context.subscriptions.push(disposable);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
