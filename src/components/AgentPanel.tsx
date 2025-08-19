import React, { useState, useEffect } from 'react';
import { useAIStore } from '../store/aiStore';
import { Bot, Play, X, CheckCircle, AlertCircle, Clock } from 'lucide-react';

export const AgentPanel: React.FC = () => {
  const [isAgentMode, setIsAgentMode] = useState(false);
  const [taskDescription, setTaskDescription] = useState('');
  const [activeTasks, setActiveTasks] = useState<string[]>([]);
  const [isCreatingTask, setIsCreatingTask] = useState(false);
  
  const {
    createAgentTask,
    getActiveAgentTasks,
    cancelAgentTask,
    isModelLoaded,
  } = useAIStore();

  useEffect(() => {
    const loadActiveTasks = async () => {
      if (isModelLoaded && isAgentMode) {
        const tasks = await getActiveAgentTasks();
        setActiveTasks(tasks);
      }
    };

    loadActiveTasks();
    
    // Refresh tasks every 5 seconds when agent mode is active
    const interval = isAgentMode ? setInterval(loadActiveTasks, 5000) : null;
    
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isAgentMode, isModelLoaded, getActiveAgentTasks]);

  const handleCreateTask = async () => {
    if (!taskDescription.trim() || isCreatingTask) return;

    setIsCreatingTask(true);
    try {
      await createAgentTask(taskDescription);
      setTaskDescription('');
      
      // Refresh active tasks
      const tasks = await getActiveAgentTasks();
      setActiveTasks(tasks);
    } catch (error) {
      console.error('Failed to create agent task:', error);
    } finally {
      setIsCreatingTask(false);
    }
  };

  const handleCancelTask = async (taskId: string) => {
    try {
      await cancelAgentTask(taskId);
      
      // Refresh active tasks
      const tasks = await getActiveAgentTasks();
      setActiveTasks(tasks);
    } catch (error) {
      console.error('Failed to cancel task:', error);
    }
  };

  const getTaskStatus = (task: string) => {
    // Parse task string to extract status info
    if (task.includes('Completed')) return 'completed';
    if (task.includes('Failed')) return 'failed';
    if (task.includes('Running')) return 'running';
    return 'pending';
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-400" />;
      case 'failed':
        return <AlertCircle className="w-4 h-4 text-red-400" />;
      case 'running':
        return <Play className="w-4 h-4 text-blue-400" />;
      default:
        return <Clock className="w-4 h-4 text-yellow-400" />;
    }
  };

  if (!isModelLoaded) {
    return (
      <div className="p-4 text-center text-terminal-muted">
        <Bot className="w-8 h-8 mx-auto mb-2 opacity-50" />
        <p className="text-sm">AI Agent mode requires the model to be loaded</p>
      </div>
    );
  }

  return (
    <div className="h-full bg-terminal-surface flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-terminal-border">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Bot className="w-5 h-5 text-ai-primary" />
            <h2 className="font-semibold text-terminal-text">AI Agent</h2>
          </div>
          
          <div className="flex items-center space-x-2">
            <button
              onClick={() => setIsAgentMode(!isAgentMode)}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                isAgentMode
                  ? 'bg-ai-primary text-white'
                  : 'bg-terminal-bg text-terminal-text hover:bg-terminal-border'
              }`}
            >
              {isAgentMode ? 'Active' : 'Inactive'}
            </button>
          </div>
        </div>
      </div>

      {!isAgentMode ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center max-w-sm">
            <Bot className="w-12 h-12 mx-auto mb-4 text-terminal-muted opacity-50" />
            <h3 className="text-lg font-medium text-terminal-text mb-2">
              Autonomous Agent Mode
            </h3>
            <p className="text-sm text-terminal-muted mb-4">
              Let the AI agent perform complex tasks automatically. Describe what you want to accomplish and the agent will break it down into steps and execute them.
            </p>
            <button
              onClick={() => setIsAgentMode(true)}
              className="px-4 py-2 bg-ai-primary text-white rounded hover:bg-ai-primary/80 transition-colors"
            >
              Enable Agent Mode
            </button>
          </div>
        </div>
      ) : (
        <div className="flex-1 flex flex-col">
          {/* Task Creation */}
          <div className="p-4 border-b border-terminal-border">
            <div className="space-y-3">
              <div className="flex items-center space-x-2">
                <Play className="w-4 h-4 text-ai-secondary" />
                <span className="text-sm font-medium text-terminal-text">Create New Task</span>
              </div>
              
              <textarea
                value={taskDescription}
                onChange={(e) => setTaskDescription(e.target.value)}
                placeholder="Describe the task you want the agent to perform (e.g., 'Create a new React project with TypeScript', 'Build and test the current project', 'Commit all changes with a meaningful message')"
                className="w-full bg-terminal-bg border border-terminal-border rounded-md px-3 py-2 text-sm text-terminal-text resize-none focus-ring"
                rows={3}
              />
              
              <button
                onClick={handleCreateTask}
                disabled={!taskDescription.trim() || isCreatingTask}
                className="w-full bg-ai-primary hover:bg-ai-primary/80 disabled:opacity-50 disabled:cursor-not-allowed text-white px-3 py-2 rounded-md text-sm font-medium transition-colors focus-ring"
              >
                {isCreatingTask ? 'Creating Task...' : 'Create Agent Task'}
              </button>
            </div>
          </div>

          {/* Active Tasks */}
          <div className="flex-1 overflow-y-auto">
            <div className="p-4">
              <div className="flex items-center space-x-2 mb-3">
                <Clock className="w-4 h-4 text-ai-secondary" />
                <span className="text-sm font-medium text-terminal-text">Active Tasks</span>
                <span className="text-xs text-terminal-muted">({activeTasks.length})</span>
              </div>

              {activeTasks.length === 0 ? (
                <div className="text-center py-8">
                  <Bot className="w-8 h-8 mx-auto text-terminal-muted mb-2 opacity-50" />
                  <p className="text-sm text-terminal-muted">
                    No active tasks. Create a task above to get started.
                  </p>
                </div>
              ) : (
                <div className="space-y-3">
                  {activeTasks.map((task, index) => {
                    const taskId = task.split(':')[0];
                    const taskDesc = task.split(':')[1] || task;
                    const status = getTaskStatus(task);
                    
                    return (
                      <div
                        key={index}
                        className="bg-terminal-bg border border-terminal-border rounded-lg p-3"
                      >
                        <div className="flex items-start justify-between">
                          <div className="flex-1">
                            <div className="flex items-center space-x-2 mb-1">
                              {getStatusIcon(status)}
                              <span className="text-xs font-medium text-terminal-text capitalize">
                                {status}
                              </span>
                            </div>
                            
                            <p className="text-sm text-terminal-text leading-relaxed">
                              {taskDesc.trim()}
                            </p>
                            
                            <div className="mt-2 text-xs text-terminal-muted">
                              Task ID: {taskId}
                            </div>
                          </div>
                          
                          <button
                            onClick={() => handleCancelTask(taskId)}
                            className="ml-2 p-1 text-terminal-muted hover:text-red-400 transition-colors"
                            title="Cancel task"
                          >
                            <X className="w-4 h-4" />
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </div>

          {/* Agent Status */}
          <div className="p-4 border-t border-terminal-border">
            <div className="flex items-center justify-between text-xs text-terminal-muted">
              <div className="flex items-center space-x-2">
                <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
                <span>Agent mode active</span>
              </div>
              <span>{activeTasks.length} task(s) in queue</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
