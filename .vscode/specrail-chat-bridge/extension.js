const vscode = require("vscode");

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

function activate(context) {
  const disposable = vscode.commands.registerCommand("specrail.runMcpAction", async (payload) => {
    const prompt = extractPrompt(payload);
    await vscode.commands.executeCommand("workbench.action.chat.open", {
      query: prompt,
      isPartialQuery: false,
    });
  });

  context.subscriptions.push(disposable);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
