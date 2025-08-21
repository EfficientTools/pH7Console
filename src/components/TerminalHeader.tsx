import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GitBranch, Package, Code, ChevronDown, Folder, Home, File } from 'lucide-react';
import { 
  SiNodedotjs, 
  SiTypescript, 
  SiPython, 
  SiRust, 
  SiGo, 
  SiReact,
  SiVuedotjs,
  SiAngular,
  SiPhp,
  SiRuby
} from 'react-icons/si';

interface RepoInfo {
  repo_name: string | null;
  current_branch: string | null;
  has_changes: boolean;
  ahead: number;
  behind: number;
  remote_url: string | null;
  is_git_repo: boolean;
}

interface RuntimeInfo {
  node_version: string | null;
  npm_version: string | null;
  rust_version: string | null;
  python_version: string | null;
  git_version: string | null;
  go_version: string | null;
  java_version: string | null;
  project_type: string | null;
}

interface DirectoryInfo {
  name: string;
  path: string;
  is_directory: boolean;
}

interface TerminalHeaderProps {
  currentPath: string;
  onPathChange?: (newPath: string) => void;
  activeSessionId?: string;
}

const TerminalHeader: React.FC<TerminalHeaderProps> = ({ currentPath, onPathChange, activeSessionId }) => {
  const [repoInfo, setRepoInfo] = useState<RepoInfo | null>(null);
  const [runtimeInfo, setRuntimeInfo] = useState<RuntimeInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [showPathDropdown, setShowPathDropdown] = useState(false);
  const [parentDirectories, setParentDirectories] = useState<DirectoryInfo[]>([]);
  const [childDirectories, setChildDirectories] = useState<DirectoryInfo[]>([]);
  const [dropdownLoading, setDropdownLoading] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const fetchRepoInfo = async () => {
      try {
        setLoading(true);
        
        // First, get repository info
        const repo = await invoke<RepoInfo>('get_repo_info', { path: currentPath });
        setRepoInfo(repo);
        
        // Only fetch runtime info if we're in a git repository
        if (repo.is_git_repo) {
          const runtime = await invoke<RuntimeInfo>('get_runtime_info', { path: currentPath });
          setRuntimeInfo(runtime);
        } else {
          setRuntimeInfo(null);
        }
      } catch (error) {
        console.error('❌ TerminalHeader: Error fetching repository info:', error);
        setRepoInfo(null);
        setRuntimeInfo(null);
      } finally {
        setLoading(false);
      }
    };

    fetchRepoInfo();
  }, [currentPath]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowPathDropdown(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  const fetchDirectoryInfo = async () => {
    if (dropdownLoading) return;
    
    setDropdownLoading(true);
    try {
      const [parents, children] = await Promise.all([
        invoke<DirectoryInfo[]>('get_parent_directories', { currentPath }),
        invoke<DirectoryInfo[]>('get_child_directories', { currentPath })
      ]);
      
      setParentDirectories(parents);
      setChildDirectories(children);
    } catch (error) {
      console.error('Failed to fetch directory info:', error);
      setParentDirectories([]);
      setChildDirectories([]);
    } finally {
      setDropdownLoading(false);
    }
  };

  const handlePathClick = () => {
    if (!showPathDropdown) {
      fetchDirectoryInfo();
    }
    setShowPathDropdown(!showPathDropdown);
  };

  const handleDirectorySelect = async (itemPath: string, isDirectory: boolean = true) => {
    try {
      if (isDirectory) {
        console.log(`📁 TerminalHeader: Directory selected: ${itemPath}`);
        
        // First, change the terminal's working directory if we have an active session
        if (activeSessionId) {
          try {
            console.log(`🔄 TerminalHeader: Executing cd command to: ${itemPath}`);
            const result = await invoke<string>('execute_command', {
              sessionId: activeSessionId,
              command: `cd "${itemPath}" && pwd`
            });
            console.log(`✅ TerminalHeader: Successfully changed terminal directory. PWD result: ${result}`);
            
            // Also update the terminal prompt by sending a dummy command to refresh it
            setTimeout(async () => {
              try {
                await invoke<string>('execute_command', {
                  sessionId: activeSessionId,
                  command: 'echo "Directory changed to $(pwd)"'
                });
              } catch (error) {
                // Ignore errors for this notification
                console.log('Info: Directory change notification failed, but cd command succeeded');
              }
            }, 100);
            
          } catch (error) {
            console.error(`❌ TerminalHeader: Failed to change terminal directory to: ${itemPath}`, error);
            // Still update the visual path even if cd command fails
          }
        } else {
          console.log(`⚠️ TerminalHeader: No active session ID available for cd command to: ${itemPath}`);
          console.log(`   Available activeSessionId: ${activeSessionId}`);
        }
        
        // Then update the visual path display
        if (onPathChange) {
          console.log(`🎯 TerminalHeader: Updating visual path display to: ${itemPath}`);
          onPathChange(itemPath);
        } else {
          console.log(`⚠️ TerminalHeader: onPathChange callback not available`);
        }
      } else {
        // Handle file selection - open in appropriate editor
        console.log(`📄 TerminalHeader: File selected: ${itemPath}`);
        
        if (activeSessionId) {
          // Determine if it's a text file and open in appropriate editor
          const extension = itemPath.split('.').pop()?.toLowerCase();
          const textExtensions = ['txt', 'md', 'js', 'jsx', 'ts', 'tsx', 'py', 'rb', 'php', 'java', 'cpp', 'c', 'h', 'go', 'rs', 'swift', 'json', 'yaml', 'yml', 'xml', 'csv', 'html', 'css', 'scss', 'less', 'conf', 'cfg', 'ini', 'env', 'toml'];
          
          let command: string;
          if (textExtensions.includes(extension || '')) {
            // Open in nano for text files
            command = `nano "${itemPath}"`;
            console.log(`📝 TerminalHeader: Opening text file in nano: ${itemPath}`);
          } else {
            // Try to open with system default for other files
            command = `open "${itemPath}"`;
            console.log(`📂 TerminalHeader: Opening file with system default: ${itemPath}`);
          }
          
          try {
            const result = await invoke<string>('execute_command', {
              sessionId: activeSessionId,
              command: command
            });
            console.log(`✅ TerminalHeader: File opened successfully. Result: ${result}`);
          } catch (error) {
            console.error(`❌ TerminalHeader: Failed to open file: ${itemPath}`, error);
          }
        } else {
          console.log(`❌ TerminalHeader: No active session ID available for opening file: ${itemPath}`);
        }
      }
      setShowPathDropdown(false);
    } catch (error) {
      console.error('Failed to handle item selection:', error);
    }
  };

  const formatPath = (path: string) => {
    // Handle tilde expansion for home directory
    if (path === '~') {
      return '~';
    }
    
    // Convert home directory to tilde notation for display
    if (path.startsWith('/Users/')) {
      const pathParts = path.split('/');
      if (pathParts.length > 2) {
        const relativePath = path.replace(`/Users/${pathParts[2]}`, '~');
        const parts = relativePath.split('/');
        if (parts.length > 3) {
          return '~/' + parts.slice(-2).join('/');
        }
        return relativePath;
      }
    }
    
    // For other paths, show last two parts
    const parts = path.split('/');
    if (parts.length > 2) {
      return '.../' + parts.slice(-2).join('/');
    }
    return path;
  };

  // Enhanced programming language detection with proper icon matching
  const getLanguageInfo = () => {
    if (!runtimeInfo || !repoInfo?.is_git_repo) return null;

    // Priority-based detection: project type first, then available runtimes
    const detections = [];

    // Project type based detection (most accurate)
    if (runtimeInfo.project_type) {
      switch (runtimeInfo.project_type.toLowerCase()) {
        case 'typescript':
          // For TypeScript projects, show Node.js runtime version with TS label
          if (runtimeInfo.node_version) {
            detections.push({
              name: 'TypeScript',
              version: `Node ${runtimeInfo.node_version}`,
              icon: <SiTypescript className="text-blue-400 w-5 h-5" />,
              priority: 10,
              source: 'project'
            });
          }
          break;
        case 'javascript':
        case 'node':
          if (runtimeInfo.node_version) {
            detections.push({
              name: 'Node.js',
              version: runtimeInfo.node_version,
              icon: <SiNodedotjs className="text-green-400 w-5 h-5" />,
              priority: 9,
              source: 'project'
            });
          }
          break;
        case 'react':
          if (runtimeInfo.node_version) {
            detections.push({
              name: 'React',
              version: runtimeInfo.node_version,
              icon: <SiReact className="text-cyan-400 w-5 h-5" />,
              priority: 11,
              source: 'project'
            });
          }
          break;
        case 'vue':
          if (runtimeInfo.node_version) {
            detections.push({
              name: 'Vue.js',
              version: runtimeInfo.node_version,
              icon: <SiVuedotjs className="text-green-400 w-5 h-5" />,
              priority: 11,
              source: 'project'
            });
          }
          break;
        case 'angular':
          if (runtimeInfo.node_version) {
            detections.push({
              name: 'Angular',
              version: runtimeInfo.node_version,
              icon: <SiAngular className="text-red-400 w-5 h-5" />,
              priority: 11,
              source: 'project'
            });
          }
          break;
        case 'python':
          if (runtimeInfo.python_version) {
            detections.push({
              name: 'Python',
              version: runtimeInfo.python_version,
              icon: <SiPython className="text-yellow-400 w-5 h-5" />,
              priority: 10,
              source: 'project'
            });
          }
          break;
        case 'rust':
          if (runtimeInfo.rust_version) {
            detections.push({
              name: 'Rust',
              version: runtimeInfo.rust_version,
              // NOTE: default SiRust icon is a circled R, not the official gear logo
              icon: <SiRust className="text-orange-400 w-7 h-7 mr-2" />,
              priority: 10,
              source: 'project'
            });
          }
          break;
        case 'go':
          if (runtimeInfo.go_version) {
            detections.push({
              name: 'Go',
              version: runtimeInfo.go_version,
              icon: <SiGo className="text-cyan-400 w-5 h-5" />,
              priority: 10,
              source: 'project'
            });
          }
          break;
        case 'php':
          detections.push({
            name: 'PHP',
            version: 'detected',
            icon: <SiPhp className="text-purple-400 w-5 h-5" />,
            priority: 10,
            source: 'project'
          });
          break;
        case 'ruby':
          detections.push({
            name: 'Ruby',
            version: 'detected',
            icon: <SiRuby className="text-red-400 w-5 h-5" />,
            priority: 10,
            source: 'project'
          });
          break;
      }
    }

    // Runtime based detection (fallback)
    if (runtimeInfo.node_version) {
      detections.push({
        name: 'Node.js',
        version: runtimeInfo.node_version,
        icon: <SiNodedotjs className="text-green-400 w-5 h-5" />,
        priority: 5,
        source: 'runtime'
      });
    }
    if (runtimeInfo.python_version) {
      detections.push({
        name: 'Python',
        version: runtimeInfo.python_version,
        icon: <SiPython className="text-yellow-400 w-5 h-5" />,
        priority: 6,
        source: 'runtime'
      });
    }
    if (runtimeInfo.rust_version) {
      detections.push({
        name: 'Rust',
        version: runtimeInfo.rust_version,
        icon: <SiRust className="text-orange-400 w-5 h-5" />,
        priority: 6,
        source: 'runtime'
      });
    }
    if (runtimeInfo.go_version) {
      detections.push({
        name: 'Go',
        version: runtimeInfo.go_version,
        icon: <SiGo className="text-cyan-400 w-5 h-5" />,
        priority: 6,
        source: 'runtime'
      });
    }

    // Return the highest priority detection
    if (detections.length > 0) {
      const best = detections.sort((a, b) => b.priority - a.priority)[0];
      return best;
    }

    return null;
  };

  const languageInfo = getLanguageInfo();

  // Helper function to format version string
  const formatVersion = (version: string) => {
    if (!version) return '';
    // For versions that already include descriptive text (like "Node 18.17.0"), return as-is
    // For simple versions (like "18.17.0"), return as-is without adding prefix
    return version;
  };

  if (loading) {
    return (
      <div className="bg-gray-800 border-b border-gray-700 p-2 text-sm">
        <div className="flex items-center gap-4">
          <div className="w-48 h-4 bg-gray-700 rounded animate-pulse"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 border-b border-gray-700 p-2 text-sm">
      <div className="flex items-center gap-4 text-gray-300">
        {/* Repository Section */}
        {repoInfo?.is_git_repo && (
          <div className="flex items-center gap-3 px-2 py-1 bg-green-900/20 border border-green-700/30 rounded">
            <div className="flex items-center gap-2">
              <Package size={14} className="text-green-400" />
              <span className="font-medium text-green-300">
                {repoInfo.repo_name || 'Unknown Repository'}
              </span>
            </div>
            
            <div className="flex items-center gap-2">
              <GitBranch size={12} className="text-gray-400" />
              <span className="text-gray-400">
                {repoInfo.current_branch || 'main'}
              </span>
              
              {repoInfo.has_changes && (
                <span className="text-yellow-400 font-bold">•</span>
              )}
              
              {(repoInfo.ahead > 0 || repoInfo.behind > 0) && (
                <div className="flex gap-1">
                  {repoInfo.ahead > 0 && (
                    <span className="text-green-400 text-xs">↑{repoInfo.ahead}</span>
                  )}
                  {repoInfo.behind > 0 && (
                    <span className="text-red-400 text-xs">↓{repoInfo.behind}</span>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Path Section */}
        <div className="relative" ref={dropdownRef}>
          <div
            className="flex items-center gap-2 px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded cursor-pointer transition-colors"
            onClick={handlePathClick}
          >
            <Code size={12} className="text-gray-400" />
            <span className="font-medium">{formatPath(currentPath)}</span>
            <ChevronDown 
              size={10} 
              className={`text-gray-400 transition-transform ${showPathDropdown ? 'rotate-180' : ''}`}
            />
          </div>

          {/* Directory Navigation Dropdown */}
          {showPathDropdown && (
            <div className="absolute top-full left-0 mt-1 w-64 bg-gray-800 border border-gray-600 rounded-md shadow-xl z-50 max-h-80 overflow-y-auto">
              {dropdownLoading ? (
                <div className="p-3 text-center text-gray-400">
                  Loading directories...
                </div>
              ) : (
                <>
                  {/* Parent Directories */}
                  {parentDirectories.length > 0 && (
                    <>
                      <div className="px-3 py-2 text-xs font-medium text-gray-500 border-b border-gray-700 uppercase tracking-wide">
                        ↑ Parent Directories
                      </div>
                      {parentDirectories.slice(0, 5).map((dir, index) => (
                        <div
                          key={`parent-${index}`}
                          className="flex items-center gap-2 px-3 py-2 hover:bg-gray-700 cursor-pointer text-sm"
                          onClick={() => handleDirectorySelect(dir.path, true)}
                          title={`Navigate to ${dir.name === '' ? 'root' : dir.name}`}
                        >
                          <Home size={12} className="text-gray-400" />
                          <span>{dir.name === '' ? '/' : dir.name}</span>
                          <span className="ml-auto text-xs text-gray-500">cd →</span>
                        </div>
                      ))}
                    </>
                  )}

                  {/* Child Directories and Files */}
                  {childDirectories.length > 0 && (
                    <>
                      <div className="px-3 py-2 text-xs font-medium text-gray-500 border-b border-gray-700 border-t uppercase tracking-wide">
                        ↓ Contents
                      </div>
                      {childDirectories.slice(0, 10).map((item, index) => (
                        <div
                          key={`child-${index}`}
                          className={`flex items-center gap-2 px-3 py-2 cursor-pointer text-sm transition-colors ${
                            item.is_directory 
                              ? 'hover:bg-green-900/20 hover:text-green-300' 
                              : 'hover:bg-blue-900/20 hover:text-blue-300'
                          }`}
                          onClick={() => handleDirectorySelect(item.path, item.is_directory)}
                          title={item.is_directory ? `Navigate to ${item.name}` : `Open ${item.name}`}
                        >
                          {item.is_directory ? (
                            <Folder size={12} className="text-gray-400" />
                          ) : (
                            <File size={12} className="text-gray-400" />
                          )}
                          <span>{item.name}</span>
                          {item.is_directory && (
                            <span className="ml-auto text-xs text-gray-500">cd →</span>
                          )}
                        </div>
                      ))}
                      {childDirectories.length > 10 && (
                        <div className="px-3 py-2 text-xs text-gray-500 text-center italic">
                          ... and {childDirectories.length - 10} more items
                        </div>
                      )}
                    </>
                  )}

                  {/* No directories found */}
                  {parentDirectories.length === 0 && childDirectories.length === 0 && (
                    <div className="px-3 py-4 text-center text-gray-500 text-sm italic">
                      No accessible directories found
                    </div>
                  )}
                </>
              )}
            </div>
          )}
        </div>

        {/* Programming Language Info */}
        {languageInfo && (
          <div className="flex items-center gap-2 px-2 py-1 bg-blue-900/20 border border-blue-700/30 rounded">
            <div className="flex items-center gap-2">
              {languageInfo.icon}
              <span className="font-medium text-blue-300">
                {languageInfo.name}
              </span>
              <span className="text-xs text-gray-400">
                {formatVersion(languageInfo.version)}
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default TerminalHeader;
