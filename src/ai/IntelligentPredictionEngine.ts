// Enhanced Predictive Intelligence Engine for pH7Console
// This file implements advanced autocompletion with intent detection and context awareness
// Now includes real-time validation of commands and paths

import { invoke } from '@tauri-apps/api/core';

interface IntentPrediction {
  intent: string;
  confidence: number;
  context: string[];
  suggestions: string[];
  explanation: string;
}

interface SystemContext {
  workingDirectory: string;
  projectType: string | null;
  runningProcesses: string[];
  systemResources: {
    cpu: number;
    memory: number;
    disk: number;
  };
  recentFiles: string[];
  gitStatus?: {
    branch: string;
    hasChanges: boolean;
    ahead: number;
    behind: number;
  };
  frequentDirectories: string[]; // Actual directories user visits often
  availableCommands: string[]; // Commands that actually exist on system
  currentDirectoryContents: string[]; // Files/folders in current directory
}

interface WorkflowPattern {
  pattern: string[];
  frequency: number;
  context: string;
  success_rate: number;
  next_likely_commands: string[];
}

export class IntelligentPredictionEngine {
  private intentPatterns: Map<string, RegExp[]> = new Map();
  private contextCache: SystemContext | null = null;
  private workflowPatterns: WorkflowPattern[] = [];
  private lastUpdateTime: number = 0;
  private commandCache: Map<string, boolean> = new Map(); // Cache for command existence
  private pathCache: Map<string, boolean> = new Map(); // Cache for path existence

  constructor() {
    this.initializeIntentPatterns();
    this.loadWorkflowPatterns();
  }

  private initializeIntentPatterns() {
    // File operations
    this.intentPatterns.set('file_search', [
      /^(find|locate|search|show|list).*(file|files)/i,
      /^(where|what).*(file|files)/i,
      /^(ls|dir).*(large|big|small)/i
    ]);

    this.intentPatterns.set('file_management', [
      /^(copy|move|delete|remove|cp|mv|rm).*(file|files|folder|directory)/i,
      /^(create|make|mkdir|touch)/i,
      /^(compress|extract|zip|unzip|tar)/i
    ]);

    // System monitoring
    this.intentPatterns.set('system_monitor', [
      /^(show|check|what).*(cpu|memory|ram|disk|process)/i,
      /^(top|htop|ps|kill)/i,
      /^(monitor|watch|status)/i
    ]);

    // Network operations
    this.intentPatterns.set('network', [
      /^(ping|curl|wget|ssh|scp)/i,
      /^(check|test).*(connection|network|internet)/i,
      /^(port|netstat|lsof)/i
    ]);

    // Development operations
    this.intentPatterns.set('development', [
      /^(git|npm|yarn|pip|cargo|docker)/i,
      /^(build|compile|test|run|start|deploy)/i,
      /^(install|update|upgrade)/i
    ]);

    // Text processing
    this.intentPatterns.set('text_processing', [
      /^(grep|sed|awk|sort|uniq|wc|head|tail)/i,
      /^(search|find|replace|count).*(text|words|lines)/i
    ]);

    // System administration
    this.intentPatterns.set('system_admin', [
      /^(sudo|su|chmod|chown|mount|umount)/i,
      /^(service|systemctl|brew|apt|yum)/i,
      /^(backup|restore|sync)/i
    ]);
  }

  private async loadWorkflowPatterns() {
    try {
      this.workflowPatterns = await invoke<WorkflowPattern[]>('get_learned_workflow_patterns');
    } catch (error) {
      console.warn('Could not load workflow patterns:', error);
    }
  }

  // Validate if a command exists and is executable
  private async validateCommand(command: string): Promise<boolean> {
    const baseCommand = command.split(' ')[0];
    
    if (this.commandCache.has(baseCommand)) {
      return this.commandCache.get(baseCommand)!;
    }

    try {
      // Check if command exists using 'which' or 'type'
      const result = await invoke('execute_simple_command', { 
        command: `which ${baseCommand} 2>/dev/null || echo "not found"`,
        directory: this.contextCache?.workingDirectory || '~'
      }) as string;
      const isValid = !result.includes('not found') && result.trim().length > 0;
      this.commandCache.set(baseCommand, isValid);
      return isValid;
    } catch {
      this.commandCache.set(baseCommand, false);
      return false;
    }
  }

  // Validate if a path exists and correct it if needed
  private async validateAndCorrectPath(path: string): Promise<string> {
    if (this.pathCache.has(path)) {
      return this.pathCache.get(path) ? path : '';
    }

    if (!this.contextCache) return '';

    try {
      // First check if path exists as-is
      const pathCheckResult = await invoke('execute_simple_command', {
        command: `test -e "${path}" && echo "exists" || echo "not found"`,
        directory: this.contextCache.workingDirectory
      }) as string;

      if (pathCheckResult.includes('exists')) {
        this.pathCache.set(path, true);
        return path;
      }

      // If path doesn't exist and it's relative, try to find it in frequent directories
      if (!path.startsWith('/') && !path.startsWith('~')) {
        // Validate frequent directories first to remove deleted ones
        const validFrequentDirs = await this.validateFrequentDirectories();
        
        for (const frequentDir of validFrequentDirs) {
          const fullPath = `${frequentDir}/${path}`;
          const fullPathCheck = await invoke('execute_simple_command', {
            command: `test -e "${fullPath}" && echo "exists" || echo "not found"`,
            directory: this.contextCache.workingDirectory
          }) as string;
          
          if (fullPathCheck.includes('exists')) {
            this.pathCache.set(path, true);
            return fullPath;
          }
        }
        
        // Try common directory patterns for the path
        const commonPaths = await this.findPathInCommonLocations(path);
        if (commonPaths) {
          this.pathCache.set(path, true);
          return commonPaths;
        }
      }

      this.pathCache.set(path, false);
      return '';
    } catch {
      this.pathCache.set(path, false);
      return '';
    }
  }

  // Validate frequent directories and remove non-existent ones
  private async validateFrequentDirectories(): Promise<string[]> {
    if (!this.contextCache) return [];
    
    try {
      const validatedDirs = await invoke('validate_frequent_directories', {
        frequent_dirs: this.contextCache.frequentDirectories || []
      }) as string[];
      
      // Update the context cache with cleaned directories
      if (this.contextCache) {
        this.contextCache.frequentDirectories = validatedDirs;
      }
      
      return validatedDirs;
    } catch (error) {
      console.warn('Could not validate frequent directories:', error);
      return this.contextCache.frequentDirectories || [];
    }
  }

  // Find a path in common locations like Desktop, Documents, Downloads, etc.
  private async findPathInCommonLocations(targetPath: string): Promise<string | null> {
    const commonLocations = [
      '~/Desktop',
      '~/Documents', 
      '~/Downloads',
      '~/Projects',
      '~/Development',
      '~/Code',
      '/usr/local',
      '/opt'
    ];
    
    for (const location of commonLocations) {
      try {
        const fullPath = `${location}/${targetPath}`;
        const pathCheck = await invoke('execute_simple_command', {
          command: `test -e "${fullPath}" && echo "exists" || echo "not found"`,
          directory: this.contextCache?.workingDirectory || '~'
        }) as string;
        
        if (pathCheck.includes('exists')) {
          return fullPath;
        }
      } catch {
        continue;
      }
    }
    
    return null;
  }

  // Get real, existing files in current directory
  private async getCurrentDirectoryFiles(): Promise<string[]> {
    if (!this.contextCache) return [];
    
    try {
      const result = await invoke('execute_simple_command', {
        command: 'ls -1A', // -1 for one per line, -A to show hidden but not . and ..
        directory: this.contextCache.workingDirectory
      }) as string;
      
      return result.split('\n')
        .map((line: string) => line.trim())
        .filter((line: string) => line.length > 0);
    } catch {
      return [];
    }
  }

  // Validate suggestions and filter out invalid ones
  private async validateSuggestions(suggestions: string[]): Promise<string[]> {
    const validSuggestions: string[] = [];
    
    for (const suggestion of suggestions) {
      const parts = suggestion.split(' ');
      const command = parts[0];
      
      // Check if command exists
      const isValidCommand = await this.validateCommand(command);
      if (!isValidCommand) continue;
      
      // Check paths in the suggestion
      let validSuggestion = suggestion;
      let isValid = true;
      
      for (let i = 1; i < parts.length; i++) {
        const part = parts[i];
        // Skip flags and options
        if (part.startsWith('-') || part.startsWith('--')) continue;
        
        // Check if it looks like a path
        if (part.includes('/') || part.includes('.') || !part.match(/^[A-Z_]+$/)) {
          const correctedPath = await this.validateAndCorrectPath(part);
          if (correctedPath) {
            if (correctedPath !== part) {
              // Update suggestion with corrected path
              validSuggestion = validSuggestion.replace(part, correctedPath);
            }
          } else {
            // Path doesn't exist, skip this suggestion
            isValid = false;
            break;
          }
        }
      }
      
      if (isValid) {
        validSuggestions.push(validSuggestion);
      }
    }
    
    return validSuggestions;
  }

  // Main prediction method
  async getPredictiveCompletions(
    input: string,
    sessionId: string,
    _cursorPosition: number = input.length
  ): Promise<IntentPrediction[]> {
    await this.updateSystemContext(sessionId);
    
    const predictions: IntentPrediction[] = [];
    
    // 1. Intent detection
    const detectedIntents = this.detectIntents(input);
    
    // 2. For each intent, generate contextual suggestions
    for (const intent of detectedIntents) {
      const prediction = await this.generateIntentPrediction(intent, input, sessionId);
      if (prediction) {
        predictions.push(prediction);
      }
    }

    // 3. Add workflow-based predictions
    const workflowPredictions = await this.getWorkflowPredictions(input, sessionId);
    predictions.push(...workflowPredictions);

    // 4. Add proactive suggestions based on system state
    const proactiveSuggestions = await this.getProactiveSuggestions();
    predictions.push(...proactiveSuggestions);

    // Sort by confidence and relevance
    return predictions.sort((a, b) => b.confidence - a.confidence).slice(0, 8);
  }

  private detectIntents(input: string): string[] {
    const intents: string[] = [];
    
    for (const [intent, patterns] of this.intentPatterns) {
      for (const pattern of patterns) {
        if (pattern.test(input)) {
          intents.push(intent);
          break;
        }
      }
    }

    // Fallback: detect intent from partial command
    if (intents.length === 0) {
      const firstWord = input.trim().split(' ')[0].toLowerCase();
      if (firstWord) {
        // Map common commands to intents
        const commandIntentMap: Record<string, string> = {
          'ls': 'file_search',
          'find': 'file_search',
          'grep': 'text_processing',
          'git': 'development',
          'npm': 'development',
          'docker': 'development',
          'ps': 'system_monitor',
          'top': 'system_monitor',
          'kill': 'system_monitor'
        };
        
        if (commandIntentMap[firstWord]) {
          intents.push(commandIntentMap[firstWord]);
        }
      }
    }

    return intents.length > 0 ? intents : ['general'];
  }

  private async generateIntentPrediction(
    intent: string,
    input: string,
    _sessionId: string
  ): Promise<IntentPrediction | null> {
    if (!this.contextCache) return null;

    switch (intent) {
      case 'file_search':
        return await this.generateFileSearchPrediction(input);
      
      case 'file_management':
        return await this.generateFileManagementPrediction(input);
      
      case 'system_monitor':
        return await this.generateSystemMonitorPrediction(input);
      
      case 'development':
        return await this.generateDevelopmentPrediction(input);
      
      case 'network':
        return await this.generateNetworkPrediction(input);
      
      case 'text_processing':
        return await this.generateTextProcessingPrediction(input);
      
      default:
        return await this.generateGeneralPrediction(input);
    }
  }

  private async generateFileSearchPrediction(input: string): Promise<IntentPrediction> {
    const suggestions = [];
    const context = this.contextCache!;
    
    // Get current directory files for context-aware suggestions
    const currentFiles = await this.getCurrentDirectoryFiles();
    const hasLogFiles = currentFiles.some(file => file.endsWith('.log'));
    const hasJsFiles = currentFiles.some(file => file.endsWith('.js') || file.endsWith('.ts'));
    const hasTextFiles = currentFiles.some(file => file.endsWith('.txt') || file.endsWith('.md'));

    if (input.includes('large') || input.includes('big')) {
      suggestions.push(
        'find . -type f -size +100M -exec ls -lh {} \\;',
        'du -h * | sort -hr | head -10',
        'find . -type f -size +50M -print0 | xargs -0 ls -lh'
      );
    } else if (input.includes('recent') || input.includes('new')) {
      suggestions.push(
        'find . -type f -mtime -7 -ls',
        'ls -lt | head -20',
        'find . -type f -newermt "1 week ago" -ls'
      );
    } else {
      // Provide context-aware suggestions based on current directory contents
      if (hasLogFiles) {
        suggestions.push('find . -name "*.log" -type f');
        suggestions.push('ls -la *.log');
      }
      if (hasJsFiles) {
        suggestions.push('find . -name "*.js" -o -name "*.ts" -type f');
        suggestions.push('ls -la *.{js,ts}');
      }
      if (hasTextFiles) {
        suggestions.push('find . -name "*.txt" -o -name "*.md" -type f');
        suggestions.push('ls -la *.{txt,md}');
      }
      
      // Always include these basic ones
      suggestions.push('ls -la');
      suggestions.push('find . -type f | head -20');
    }

    // Validate suggestions and filter out invalid ones
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'file_search',
      confidence: 0.9,
      context: [context.workingDirectory, context.projectType || 'unknown'],
      suggestions: validSuggestions,
      explanation: 'Search for files based on criteria like size, modification time, or name pattern'
    };
  }

  private async generateFileManagementPrediction(input: string): Promise<IntentPrediction> {
    const suggestions = [];
    const currentFiles = await this.getCurrentDirectoryFiles();
    
    if (input.includes('copy') || input.includes('cp')) {
      // Suggest real files for copying
      if (currentFiles.length > 0) {
        const firstFile = currentFiles[0];
        suggestions.push(`cp ${firstFile} ${firstFile}.backup`);
        if (currentFiles.some(f => f.includes('.'))) {
          const fileWithExt = currentFiles.find(f => f.includes('.'));
          suggestions.push(`cp ${fileWithExt} ./backup/`);
        }
      }
      suggestions.push('cp -r source/ destination/');
      suggestions.push('rsync -av source/ destination/');
    } else if (input.includes('move') || input.includes('mv')) {
      // Suggest real files for moving
      if (currentFiles.length > 0) {
        const firstFile = currentFiles[0];
        suggestions.push(`mv ${firstFile} ./archive/`);
        suggestions.push(`mv ${firstFile} ${firstFile}.old`);
      }
      suggestions.push('mv oldname newname');
      suggestions.push('find . -name "*.tmp" -exec mv {} /tmp/ \\;');
    } else if (input.includes('delete') || input.includes('rm')) {
      // Suggest careful deletion commands
      if (currentFiles.some(f => f.endsWith('.tmp') || f.endsWith('.log'))) {
        suggestions.push('rm *.tmp');
        suggestions.push('rm *.log');
      }
      suggestions.push('rm -i filename');  // Interactive deletion
      suggestions.push('find . -name "*.tmp" -delete');
    } else {
      // General file management
      if (currentFiles.length > 0) {
        suggestions.push(`ls -la ${currentFiles[0]}`);
        suggestions.push('mkdir new_directory');
      }
    }

    // Validate suggestions
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'file_management',
      confidence: 0.85,
      context: [this.contextCache!.workingDirectory],
      suggestions: validSuggestions,
      explanation: 'Manage files and directories with copy, move, or delete operations'
    };
  }

  private async generateSystemMonitorPrediction(input: string): Promise<IntentPrediction> {
    const suggestions = [];
    const resources = this.contextCache!.systemResources;

    if (input.includes('cpu')) {
      suggestions.push(
        'top -o cpu',
        'ps aux --sort=-%cpu | head -10',
        'htop'
      );
    } else if (input.includes('memory') || input.includes('ram')) {
      suggestions.push(
        'free -h',
        'ps aux --sort=-%mem | head -10',
        'top -o mem'
      );
    } else if (input.includes('disk')) {
      suggestions.push(
        'df -h',
        'du -sh * | sort -hr',
        'ncdu'
      );
    } else {
      suggestions.push(
        'top',
        'htop',
        'ps aux',
        'iostat',
        'vmstat'
      );
    }

    // Add proactive suggestions based on current resource usage
    if (resources.cpu > 80) {
      suggestions.unshift('ps aux --sort=-%cpu | head -5  # High CPU usage detected');
    }
    if (resources.memory > 85) {
      suggestions.unshift('free -h  # High memory usage detected');
    }
    if (resources.disk > 90) {
      suggestions.unshift('df -h  # Disk space is running low');
    }

    // Validate suggestions
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'system_monitor',
      confidence: 0.9,
      context: [`CPU: ${resources.cpu}%`, `Memory: ${resources.memory}%`, `Disk: ${resources.disk}%`],
      suggestions: validSuggestions,
      explanation: 'Monitor system resources and running processes'
    };
  }

  private async generateDevelopmentPrediction(input: string): Promise<IntentPrediction> {
    const suggestions = [];
    const context = this.contextCache!;
    const currentFiles = await this.getCurrentDirectoryFiles();

    // Git operations
    if (input.includes('git') || context.gitStatus) {
      if (context.gitStatus?.hasChanges) {
        suggestions.push(
          'git status',
          'git add .',
          'git commit -m "Update: "',
          'git diff'
        );
      } else {
        suggestions.push(
          'git pull',
          'git status',
          'git log --oneline -10',
          'git branch'
        );
      }
    }

    // Project-specific commands based on actual files in directory
    const hasPackageJson = currentFiles.includes('package.json');
    const hasCargoToml = currentFiles.includes('Cargo.toml');
    const hasRequirementsTxt = currentFiles.includes('requirements.txt');
    const hasPyprojectToml = currentFiles.includes('pyproject.toml');

    if (hasPackageJson || context.projectType === 'node') {
      suggestions.push(
        'npm install',
        'npm run dev',
        'npm test',
        'npm run build'
      );
    }
    
    if (hasCargoToml || context.projectType === 'rust') {
      suggestions.push(
        'cargo build',
        'cargo test',
        'cargo run',
        'cargo check'
      );
    }
    
    if (hasRequirementsTxt || hasPyprojectToml || context.projectType === 'python') {
      if (hasRequirementsTxt) {
        suggestions.push('pip install -r requirements.txt');
      }
      suggestions.push(
        'python -m venv venv',
        'source venv/bin/activate',
        'python main.py'
      );
    }

    // Add context-aware suggestions based on actual files
    const pythonFiles = currentFiles.filter(f => f.endsWith('.py'));
    if (pythonFiles.length > 0) {
      suggestions.push(`python ${pythonFiles[0]}`);
    }

    const jsFiles = currentFiles.filter(f => f.endsWith('.js') || f.endsWith('.ts'));
    if (jsFiles.length > 0) {
      suggestions.push(`node ${jsFiles[0]}`);
    }

    // Validate suggestions
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'development',
      confidence: 0.95,
      context: [context.projectType || 'unknown', context.gitStatus?.branch || 'no-git'],
      suggestions: validSuggestions,
      explanation: 'Development workflow commands for your current project'
    };
  }

  private async generateNetworkPrediction(_input: string): Promise<IntentPrediction> {
    const suggestions = [
      'ping google.com',
      'curl -I https://example.com',
      'netstat -tulpn',
      'lsof -i :8080',
      'nslookup domain.com',
      'wget https://example.com/file.zip'
    ];

    // Validate suggestions
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'network',
      confidence: 0.8,
      context: ['network operations'],
      suggestions: validSuggestions,
      explanation: 'Network diagnostic and communication commands'
    };
  }

  private async generateTextProcessingPrediction(input: string): Promise<IntentPrediction> {
    const suggestions = [];
    const currentFiles = await this.getCurrentDirectoryFiles();
    const textFiles = currentFiles.filter(f => f.endsWith('.txt') || f.endsWith('.md') || f.endsWith('.log'));

    if (input.includes('search') || input.includes('find')) {
      suggestions.push('grep -r "search_term" .');
      if (textFiles.length > 0) {
        suggestions.push(`grep -i "pattern" ${textFiles[0]}`);
        suggestions.push(`find . -name "*.txt" -exec grep -l "search" {} \\;`);
      }
    } else {
      if (textFiles.length > 0) {
        const firstTextFile = textFiles[0];
        suggestions.push(
          `grep "pattern" ${firstTextFile}`,
          `sed "s/old/new/g" ${firstTextFile}`,
          `awk '{print $1}' ${firstTextFile}`,
          `wc -l ${firstTextFile}`
        );
      }
      suggestions.push(
        'sort file.txt | uniq',
        'head -10 filename',
        'tail -10 filename'
      );
    }

    // Validate suggestions
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'text_processing',
      confidence: 0.8,
      context: ['text_tools'],
      suggestions: validSuggestions,
      explanation: 'Search, filter, and manipulate text content'
    };
  }

  private async generateGeneralPrediction(_input: string): Promise<IntentPrediction> {
    const currentFiles = await this.getCurrentDirectoryFiles();
    const suggestions = ['ls -la', 'pwd', 'cd ..', 'history | tail -10'];
    
    // Add context-aware suggestions
    if (currentFiles.length > 0) {
      suggestions.push(`cat ${currentFiles[0]}`);
      if (currentFiles.length > 1) {
        suggestions.push(`ls ${currentFiles[1]}`);
      }
    }

    // Validate suggestions
    const validSuggestions = await this.validateSuggestions(suggestions);

    return {
      intent: 'general',
      confidence: 0.5,
      context: ['general'],
      suggestions: validSuggestions,
      explanation: 'Common terminal commands'
    };
  }

  private async getWorkflowPredictions(_input: string, sessionId: string): Promise<IntentPrediction[]> {
    const predictions: IntentPrediction[] = [];
    
    try {
      const recentCommands = await invoke<string[]>('get_recent_command_sequence', { sessionId, limit: 5 });
      
      // Find matching workflow patterns
      for (const pattern of this.workflowPatterns) {
        if (this.matchesPattern(recentCommands, pattern.pattern)) {
          predictions.push({
            intent: 'workflow',
            confidence: pattern.success_rate,
            context: [pattern.context],
            suggestions: pattern.next_likely_commands,
            explanation: `Workflow suggestion based on your usage patterns (${Math.round(pattern.success_rate * 100)}% confidence)`
          });
        }
      }
    } catch (error) {
      console.warn('Could not get workflow predictions:', error);
    }

    return predictions;
  }

  private async getProactiveSuggestions(): Promise<IntentPrediction[]> {
    const suggestions: IntentPrediction[] = [];
    const context = this.contextCache!;

    // Disk space warning
    if (context.systemResources.disk > 90) {
      suggestions.push({
        intent: 'maintenance',
        confidence: 0.9,
        context: ['disk_space_low'],
        suggestions: [
          'du -sh * | sort -hr | head -10  # Find largest directories',
          'find . -name "*.log" -size +10M  # Find large log files',
          'docker system prune  # Clean up Docker if installed'
        ],
        explanation: '⚠️ Disk space is running low. Here are some cleanup suggestions.'
      });
    }

    // High CPU usage
    if (context.systemResources.cpu > 85) {
      suggestions.push({
        intent: 'performance',
        confidence: 0.8,
        context: ['high_cpu'],
        suggestions: [
          'ps aux --sort=-%cpu | head -10  # Show CPU-intensive processes',
          'top -o cpu  # Monitor CPU usage in real-time',
          'killall -9 process_name  # Kill specific process if needed'
        ],
        explanation: '🔥 High CPU usage detected. Consider checking running processes.'
      });
    }

    // Git repository with uncommitted changes
    if (context.gitStatus?.hasChanges) {
      suggestions.push({
        intent: 'git_workflow',
        confidence: 0.7,
        context: ['uncommitted_changes'],
        suggestions: [
          'git status  # Review changes',
          'git add . && git commit -m "WIP: "  # Quick commit',
          'git stash  # Temporarily store changes'
        ],
        explanation: '📝 You have uncommitted changes. Consider saving your work.'
      });
    }

    return suggestions;
  }

  private matchesPattern(recentCommands: string[], pattern: string[]): boolean {
    if (recentCommands.length < pattern.length) return false;
    
    const recent = recentCommands.slice(-pattern.length);
    return pattern.every((cmd, index) => {
      return recent[index].startsWith(cmd) || recent[index].includes(cmd);
    });
  }

  private async updateSystemContext(sessionId: string) {
    const now = Date.now();
    if (now - this.lastUpdateTime < 5000) return; // Update every 5 seconds max

    try {
      this.contextCache = await invoke<SystemContext>('get_enhanced_system_context', { sessionId });
      this.lastUpdateTime = now;
    } catch (error) {
      console.warn('Could not update system context:', error);
    }
  }

  // Public method to get real-time suggestions as user types
  async getRealtimeSuggestions(
    input: string,
    sessionId: string,
    maxSuggestions: number = 5
  ): Promise<string[]> {
    if (input.length < 2) return [];

    const predictions = await this.getPredictiveCompletions(input, sessionId);
    const allSuggestions = predictions.flatMap(p => p.suggestions);
    
    // Filter and rank suggestions
    const relevantSuggestions = allSuggestions
      .filter(suggestion => 
        suggestion.toLowerCase().includes(input.toLowerCase()) ||
        input.toLowerCase().includes(suggestion.toLowerCase().split(' ')[0])
      )
      .slice(0, maxSuggestions);

    return relevantSuggestions;
  }
}

export const intelligentPredictor = new IntelligentPredictionEngine();
