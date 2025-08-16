use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::time::{sleep, Duration};

use super::learning_engine::LearningEngine;

/// Agent mode for autonomous task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub steps: Vec<AgentStep>,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub id: String,
    pub command: String,
    pub description: String,
    pub expected_outcome: String,
    pub status: StepStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub dependencies: Vec<String>, // Step IDs this step depends on
    pub conditional: Option<StepCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Waiting,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    pub condition_type: ConditionType,
    pub expected_value: String,
    pub operator: ConditionOperator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    FileExists,
    DirectoryExists,
    CommandOutput,
    ExitCode,
    OutputContains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
}

/// Intelligent agent for autonomous task execution
pub struct IntelligentAgent {
    learning_engine: LearningEngine,
    active_tasks: VecDeque<AgentTask>,
    task_history: Vec<AgentTask>,
    capabilities: AgentCapabilities,
    safety_checks: SafetySettings,
}

#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub max_concurrent_tasks: usize,
    pub allowed_commands: Vec<String>,
    pub forbidden_commands: Vec<String>,
    pub max_execution_time_seconds: u64,
    pub auto_confirm_safe_operations: bool,
    pub learning_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct SafetySettings {
    pub require_confirmation_for_destructive: bool,
    pub sandbox_mode: bool,
    pub max_file_operations_per_task: usize,
    pub allowed_directories: Vec<String>,
    pub forbidden_directories: Vec<String>,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 3,
            allowed_commands: vec![
                "ls".to_string(), "cd".to_string(), "pwd".to_string(),
                "git".to_string(), "npm".to_string(), "cargo".to_string(),
                "find".to_string(), "grep".to_string(), "cat".to_string(),
                "mkdir".to_string(), "touch".to_string(),
            ],
            forbidden_commands: vec![
                "rm -rf /".to_string(), "sudo rm".to_string(),
                "format".to_string(), "fdisk".to_string(),
            ],
            max_execution_time_seconds: 300, // 5 minutes
            auto_confirm_safe_operations: true,
            learning_enabled: true,
        }
    }
}

impl Default for SafetySettings {
    fn default() -> Self {
        Self {
            require_confirmation_for_destructive: true,
            sandbox_mode: false,
            max_file_operations_per_task: 50,
            allowed_directories: vec![
                "~/".to_string(),
                "/tmp".to_string(),
                "/home".to_string(),
                "/Users".to_string(),
            ],
            forbidden_directories: vec![
                "/".to_string(),
                "/etc".to_string(),
                "/usr".to_string(),
                "/var".to_string(),
                "/sys".to_string(),
            ],
        }
    }
}

impl IntelligentAgent {
    pub fn new(learning_engine: LearningEngine) -> Self {
        Self {
            learning_engine,
            active_tasks: VecDeque::new(),
            task_history: Vec::new(),
            capabilities: AgentCapabilities::default(),
            safety_checks: SafetySettings::default(),
        }
    }

    /// Create a new autonomous task from natural language description
    pub async fn create_task_from_description(&mut self, description: &str) -> Result<String, String> {
        let task_id = uuid::Uuid::new_v4().to_string();
        
        // Parse natural language into executable steps
        let steps = self.parse_natural_language_to_steps(description).await?;
        
        let task = AgentTask {
            id: task_id.clone(),
            description: description.to_string(),
            steps,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            progress: 0.0,
        };

        // Validate task safety
        self.validate_task_safety(&task)?;

        self.active_tasks.push_back(task);
        Ok(task_id)
    }

    /// Parse natural language into executable steps
    async fn parse_natural_language_to_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let desc_lower = description.to_lowercase();

        // Common task patterns
        if desc_lower.contains("create") && (desc_lower.contains("project") || desc_lower.contains("app")) {
            steps.extend(self.create_project_steps(description)?);
        } else if desc_lower.contains("git") && desc_lower.contains("commit") {
            steps.extend(self.create_git_commit_steps(description)?);
        } else if desc_lower.contains("build") || desc_lower.contains("compile") {
            steps.extend(self.create_build_steps(description)?);
        } else if desc_lower.contains("test") {
            steps.extend(self.create_test_steps(description)?);
        } else if desc_lower.contains("deploy") {
            steps.extend(self.create_deploy_steps(description)?);
        } else if desc_lower.contains("backup") {
            steps.extend(self.create_backup_steps(description)?);
        } else if desc_lower.contains("clean") || desc_lower.contains("cleanup") {
            steps.extend(self.create_cleanup_steps(description)?);
        } else if desc_lower.contains("install") {
            steps.extend(self.create_install_steps(description)?);
        } else {
            // Fallback: try to generate steps using learning engine
            steps.extend(self.generate_steps_from_learning(description)?);
        }

        if steps.is_empty() {
            return Err("Could not parse the description into executable steps".to_string());
        }

        Ok(steps)
    }

    /// Create project setup steps
    fn create_project_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        // Determine project type
        let project_type = if description.contains("react") || description.contains("javascript") {
            "react"
        } else if description.contains("rust") {
            "rust"
        } else if description.contains("python") {
            "python"
        } else {
            "generic"
        };

        match project_type {
            "react" => {
                steps.push(AgentStep {
                    id: format!("{}_1", step_id_base),
                    command: "npx create-react-app my-app".to_string(),
                    description: "Create React application".to_string(),
                    expected_outcome: "React app created successfully".to_string(),
                    status: StepStatus::Waiting,
                    retry_count: 0,
                    max_retries: 2,
                    dependencies: vec![],
                    conditional: None,
                });

                steps.push(AgentStep {
                    id: format!("{}_2", step_id_base),
                    command: "cd my-app && npm install".to_string(),
                    description: "Install dependencies".to_string(),
                    expected_outcome: "Dependencies installed".to_string(),
                    status: StepStatus::Waiting,
                    retry_count: 0,
                    max_retries: 2,
                    dependencies: vec![format!("{}_1", step_id_base)],
                    conditional: None,
                });
            },
            "rust" => {
                steps.push(AgentStep {
                    id: format!("{}_1", step_id_base),
                    command: "cargo new my-rust-project".to_string(),
                    description: "Create Rust project".to_string(),
                    expected_outcome: "Rust project created".to_string(),
                    status: StepStatus::Waiting,
                    retry_count: 0,
                    max_retries: 2,
                    dependencies: vec![],
                    conditional: None,
                });

                steps.push(AgentStep {
                    id: format!("{}_2", step_id_base),
                    command: "cd my-rust-project && cargo build".to_string(),
                    description: "Build Rust project".to_string(),
                    expected_outcome: "Project builds successfully".to_string(),
                    status: StepStatus::Waiting,
                    retry_count: 0,
                    max_retries: 2,
                    dependencies: vec![format!("{}_1", step_id_base)],
                    conditional: None,
                });
            },
            _ => {
                steps.push(AgentStep {
                    id: format!("{}_1", step_id_base),
                    command: "mkdir new-project && cd new-project".to_string(),
                    description: "Create project directory".to_string(),
                    expected_outcome: "Directory created".to_string(),
                    status: StepStatus::Waiting,
                    retry_count: 0,
                    max_retries: 1,
                    dependencies: vec![],
                    conditional: None,
                });
            }
        }

        Ok(steps)
    }

    /// Create git commit steps
    fn create_git_commit_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        // Extract commit message if provided
        let commit_message = if description.contains("message") {
            // Try to extract message from quotes or after "message:"
            description.split("message").nth(1)
                .map(|s| s.trim_start_matches(':').trim().trim_matches('"'))
                .unwrap_or("Auto commit")
        } else {
            "Auto commit"
        };

        steps.push(AgentStep {
            id: format!("{}_1", step_id_base),
            command: "git status".to_string(),
            description: "Check repository status".to_string(),
            expected_outcome: "Repository status displayed".to_string(),
            status: StepStatus::Waiting,
            retry_count: 0,
            max_retries: 1,
            dependencies: vec![],
            conditional: None,
        });

        steps.push(AgentStep {
            id: format!("{}_2", step_id_base),
            command: "git add .".to_string(),
            description: "Stage all changes".to_string(),
            expected_outcome: "Changes staged".to_string(),
            status: StepStatus::Waiting,
            retry_count: 0,
            max_retries: 1,
            dependencies: vec![format!("{}_1", step_id_base)],
            conditional: None,
        });

        steps.push(AgentStep {
            id: format!("{}_3", step_id_base),
            command: format!("git commit -m \"{}\"", commit_message),
            description: "Commit changes".to_string(),
            expected_outcome: "Changes committed".to_string(),
            status: StepStatus::Waiting,
            retry_count: 0,
            max_retries: 1,
            dependencies: vec![format!("{}_2", step_id_base)],
            conditional: None,
        });

        Ok(steps)
    }

    /// Create build steps
    fn create_build_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        // Detect build system
        if description.contains("npm") || description.contains("node") {
            steps.push(AgentStep {
                id: format!("{}_1", step_id_base),
                command: "npm run build".to_string(),
                description: "Build with npm".to_string(),
                expected_outcome: "Build completed successfully".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 2,
                dependencies: vec![],
                conditional: Some(StepCondition {
                    condition_type: ConditionType::FileExists,
                    expected_value: "package.json".to_string(),
                    operator: ConditionOperator::Equals,
                }),
            });
        } else if description.contains("cargo") || description.contains("rust") {
            steps.push(AgentStep {
                id: format!("{}_1", step_id_base),
                command: "cargo build --release".to_string(),
                description: "Build with Cargo".to_string(),
                expected_outcome: "Release build completed".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 2,
                dependencies: vec![],
                conditional: Some(StepCondition {
                    condition_type: ConditionType::FileExists,
                    expected_value: "Cargo.toml".to_string(),
                    operator: ConditionOperator::Equals,
                }),
            });
        }

        Ok(steps)
    }

    /// Create test steps
    fn create_test_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        if description.contains("npm") || description.contains("jest") {
            steps.push(AgentStep {
                id: format!("{}_1", step_id_base),
                command: "npm test".to_string(),
                description: "Run npm tests".to_string(),
                expected_outcome: "All tests pass".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 1,
                dependencies: vec![],
                conditional: None,
            });
        } else if description.contains("cargo") || description.contains("rust") {
            steps.push(AgentStep {
                id: format!("{}_1", step_id_base),
                command: "cargo test".to_string(),
                description: "Run Rust tests".to_string(),
                expected_outcome: "All tests pass".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 1,
                dependencies: vec![],
                conditional: None,
            });
        }

        Ok(steps)
    }

    /// Create deployment steps
    fn create_deploy_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        // Basic deployment steps
        steps.push(AgentStep {
            id: format!("{}_1", step_id_base),
            command: "npm run build".to_string(),
            description: "Build for production".to_string(),
            expected_outcome: "Production build ready".to_string(),
            status: StepStatus::Waiting,
            retry_count: 0,
            max_retries: 2,
            dependencies: vec![],
            conditional: Some(StepCondition {
                condition_type: ConditionType::FileExists,
                expected_value: "package.json".to_string(),
                operator: ConditionOperator::Equals,
            }),
        });

        Ok(steps)
    }

    /// Create backup steps
    fn create_backup_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        let backup_dir = format!("backup_{}", Utc::now().format("%Y%m%d_%H%M%S"));

        steps.push(AgentStep {
            id: format!("{}_1", step_id_base),
            command: format!("mkdir -p {}", backup_dir),
            description: "Create backup directory".to_string(),
            expected_outcome: "Backup directory created".to_string(),
            status: StepStatus::Waiting,
            retry_count: 0,
            max_retries: 1,
            dependencies: vec![],
            conditional: None,
        });

        steps.push(AgentStep {
            id: format!("{}_2", step_id_base),
            command: format!("cp -r . {}/", backup_dir),
            description: "Copy files to backup".to_string(),
            expected_outcome: "Files backed up successfully".to_string(),
            status: StepStatus::Waiting,
            retry_count: 0,
            max_retries: 2,
            dependencies: vec![format!("{}_1", step_id_base)],
            conditional: None,
        });

        Ok(steps)
    }

    /// Create cleanup steps
    fn create_cleanup_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        if description.contains("node_modules") || description.contains("npm") {
            steps.push(AgentStep {
                id: format!("{}_1", step_id_base),
                command: "rm -rf node_modules".to_string(),
                description: "Remove node_modules".to_string(),
                expected_outcome: "node_modules removed".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 1,
                dependencies: vec![],
                conditional: Some(StepCondition {
                    condition_type: ConditionType::DirectoryExists,
                    expected_value: "node_modules".to_string(),
                    operator: ConditionOperator::Equals,
                }),
            });
        }

        if description.contains("target") || description.contains("rust") {
            steps.push(AgentStep {
                id: format!("{}_2", step_id_base),
                command: "cargo clean".to_string(),
                description: "Clean Rust build artifacts".to_string(),
                expected_outcome: "Build artifacts cleaned".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 1,
                dependencies: vec![],
                conditional: Some(StepCondition {
                    condition_type: ConditionType::FileExists,
                    expected_value: "Cargo.toml".to_string(),
                    operator: ConditionOperator::Equals,
                }),
            });
        }

        Ok(steps)
    }

    /// Create installation steps
    fn create_install_steps(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        if description.contains("dependencies") || description.contains("npm") {
            steps.push(AgentStep {
                id: format!("{}_1", step_id_base),
                command: "npm install".to_string(),
                description: "Install npm dependencies".to_string(),
                expected_outcome: "Dependencies installed".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 2,
                dependencies: vec![],
                conditional: None,
            });
        }

        Ok(steps)
    }

    /// Generate steps using learning engine patterns
    fn generate_steps_from_learning(&self, description: &str) -> Result<Vec<AgentStep>, String> {
        // Use learning engine to suggest commands based on description
        let suggestions = self.learning_engine.suggest_commands(description, "", 5);
        
        if suggestions.is_empty() {
            return Err("No learned patterns match the description".to_string());
        }

        let mut steps = Vec::new();
        let step_id_base = uuid::Uuid::new_v4().to_string();

        for (i, command) in suggestions.iter().enumerate() {
            steps.push(AgentStep {
                id: format!("{}_{}", step_id_base, i + 1),
                command: command.clone(),
                description: format!("Execute: {}", command),
                expected_outcome: "Command executed successfully".to_string(),
                status: StepStatus::Waiting,
                retry_count: 0,
                max_retries: 1,
                dependencies: if i > 0 { vec![format!("{}_{}", step_id_base, i)] } else { vec![] },
                conditional: None,
            });
        }

        Ok(steps)
    }

    /// Validate task safety before execution
    fn validate_task_safety(&self, task: &AgentTask) -> Result<(), String> {
        for step in &task.steps {
            // Check forbidden commands
            for forbidden in &self.capabilities.forbidden_commands {
                if step.command.contains(forbidden) {
                    return Err(format!("Forbidden command detected: {}", forbidden));
                }
            }

            // Check if command is in allowed list (if restrictive mode)
            if !self.capabilities.allowed_commands.is_empty() {
                let cmd_parts: Vec<&str> = step.command.split_whitespace().collect();
                if let Some(base_cmd) = cmd_parts.first() {
                    if !self.capabilities.allowed_commands.iter().any(|allowed| base_cmd.starts_with(allowed)) {
                        return Err(format!("Command not in allowed list: {}", base_cmd));
                    }
                }
            }

            // Check for destructive operations
            if self.safety_checks.require_confirmation_for_destructive {
                if step.command.contains("rm") && step.command.contains("-rf") {
                    return Err("Destructive operation requires manual confirmation".to_string());
                }
            }
        }

        Ok(())
    }

    /// Execute a single task step
    pub async fn execute_step(
        &mut self, 
        step: &mut AgentStep,
        session_id: &str,
        terminal_execute_fn: impl Fn(&str, &str) -> Box<dyn std::future::Future<Output = Result<(String, bool), String>> + Send>
    ) -> Result<bool, String> {
        step.status = StepStatus::Running;
        
        // Check conditional if present
        if let Some(condition) = &step.conditional {
            if !self.check_step_condition(condition).await? {
                step.status = StepStatus::Skipped;
                return Ok(true); // Consider skipped as success
            }
        }

        // Execute command - simplified for now
        // TODO: Implement proper async execution with pinned futures
        let result: Result<(String, bool), String> = Ok(("Command execution placeholder".to_string(), true));
        
        match result {
            Ok((output, success)) => {
                if success {
                    step.status = StepStatus::Completed;
                    
                    // Learn from successful execution
                    if self.capabilities.learning_enabled {
                        self.learning_engine.learn_from_interaction(
                            step.command.clone(),
                            output,
                            step.description.clone(),
                            true,
                            None,
                        );
                    }
                    
                    Ok(true)
                } else {
                    step.retry_count += 1;
                    if step.retry_count >= step.max_retries {
                        step.status = StepStatus::Failed;
                        
                        // Learn from failure
                        if self.capabilities.learning_enabled {
                            self.learning_engine.learn_from_interaction(
                                step.command.clone(),
                                output,
                                step.description.clone(),
                                false,
                                None,
                            );
                        }
                        
                        Ok(false)
                    } else {
                        // Retry after a delay
                        sleep(Duration::from_secs(2)).await;
                        Ok(false) // Will retry
                    }
                }
            }
            Err(error) => {
                step.retry_count += 1;
                if step.retry_count >= step.max_retries {
                    step.status = StepStatus::Failed;
                    Ok(false)
                } else {
                    sleep(Duration::from_secs(2)).await;
                    Ok(false)
                }
            }
        }
    }

    /// Check if a step condition is met
    async fn check_step_condition(&self, condition: &StepCondition) -> Result<bool, String> {
        match &condition.condition_type {
            ConditionType::FileExists => {
                let exists = std::path::Path::new(&condition.expected_value).exists();
                Ok(match condition.operator {
                    ConditionOperator::Equals => exists,
                    ConditionOperator::NotEquals => !exists,
                    _ => false,
                })
            }
            ConditionType::DirectoryExists => {
                let exists = std::path::Path::new(&condition.expected_value).is_dir();
                Ok(match condition.operator {
                    ConditionOperator::Equals => exists,
                    ConditionOperator::NotEquals => !exists,
                    _ => false,
                })
            }
            _ => Ok(true), // Default to true for unsupported conditions
        }
    }

    /// Get current task status
    pub fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        self.active_tasks.iter()
            .find(|task| task.id == task_id)
            .map(|task| task.status.clone())
            .or_else(|| {
                self.task_history.iter()
                    .find(|task| task.id == task_id)
                    .map(|task| task.status.clone())
            })
    }

    /// Get all active tasks
    pub fn get_active_tasks(&self) -> Vec<&AgentTask> {
        self.active_tasks.iter().collect()
    }

    /// Cancel a task
    pub fn cancel_task(&mut self, task_id: &str) -> Result<(), String> {
        if let Some(task) = self.active_tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Cancelled;
            Ok(())
        } else {
            Err("Task not found".to_string())
        }
    }

    /// Update agent capabilities
    pub fn update_capabilities(&mut self, capabilities: AgentCapabilities) {
        self.capabilities = capabilities;
    }

    /// Update safety settings
    pub fn update_safety_settings(&mut self, safety: SafetySettings) {
        self.safety_checks = safety;
    }
}
