import { Page, Locator } from '@playwright/test';

export class AppFixture {
  readonly page: Page;
  readonly terminalInput: Locator;
  readonly terminalOutput: Locator;
  readonly sidebar: Locator;
  readonly aiPanel: Locator;
  readonly smartSuggestions: Locator;
  readonly fileExplorer: Locator;

  constructor(page: Page) {
    this.page = page;
    this.terminalInput = page.locator('[data-testid="terminal-input"]');
    this.terminalOutput = page.locator('[data-testid="terminal-output"]');
    this.sidebar = page.locator('[data-testid="sidebar"]');
    this.aiPanel = page.locator('[data-testid="ai-panel"]');
    this.smartSuggestions = page.locator('[data-testid="smart-suggestions"]');
    this.fileExplorer = page.locator('[data-testid="file-explorer"]');
  }

  async goto() {
    await this.page.goto('http://localhost:5173');
    await this.page.waitForLoadState('networkidle');
  }

  async executeCommand(command: string) {
    await this.terminalInput.fill(command);
    await this.terminalInput.press('Enter');
    // Wait for command execution to complete
    await this.page.waitForTimeout(1000);
  }

  async waitForSuggestions() {
    await this.smartSuggestions.waitFor({ state: 'visible', timeout: 5000 });
  }

  async getSuggestions() {
    await this.waitForSuggestions();
    return await this.smartSuggestions.locator('[data-testid="suggestion-item"]').allTextContents();
  }

  async clickSuggestion(suggestionText: string) {
    const suggestion = this.smartSuggestions.locator('[data-testid="suggestion-item"]', { hasText: suggestionText });
    await suggestion.click();
  }

  async getTerminalOutput() {
    return await this.terminalOutput.textContent();
  }

  async createNewTerminal() {
    const newTerminalButton = this.sidebar.locator('[data-testid="new-terminal-button"]');
    await newTerminalButton.click();
  }

  async switchToTerminal(terminalId: string) {
    const terminalTab = this.sidebar.locator(`[data-testid="terminal-tab-${terminalId}"]`);
    await terminalTab.click();
  }

  async toggleAIPanel() {
    const aiToggle = this.page.locator('[data-testid="ai-panel-toggle"]');
    await aiToggle.click();
  }

  async openFileExplorer() {
    const fileExplorerToggle = this.page.locator('[data-testid="file-explorer-toggle"]');
    await fileExplorerToggle.click();
  }

  async navigateToDirectory(directory: string) {
    await this.executeCommand(`cd ${directory}`);
  }

  async typeInTerminal(text: string) {
    await this.terminalInput.fill(text);
  }

  async waitForCommandCompletion() {
    // Wait for the terminal to show a new prompt
    await this.page.waitForTimeout(2000);
  }

  async getCommandHistory() {
    // Access command history (usually with up arrow)
    await this.terminalInput.press('ArrowUp');
    return await this.terminalInput.inputValue();
  }

  async testNaturalLanguageCommand(nlCommand: string) {
    await this.executeCommand(nlCommand);
    await this.waitForCommandCompletion();
    return await this.getTerminalOutput();
  }

  async testCommandValidation(command: string) {
    await this.typeInTerminal(command);
    await this.waitForSuggestions();
    return await this.getSuggestions();
  }
}
