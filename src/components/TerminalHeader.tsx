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
  refreshToken?: number;
}

const runtimeCache = new Map<string, { expiresAt: number; value: RuntimeInfo }>();
const RUNTIME_CACHE_MS = 30_000;

const TerminalHeader: React.FC<TerminalHeaderProps> = ({
  currentPath,
  onPathChange,
  activeSessionId,
  refreshToken = 0,
}) => {
  const [repoInfo, setRepoInfo] = useState<RepoInfo | null>(null);
  const [runtimeInfo, setRuntimeInfo] = useState<RuntimeInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [showPathDropdown, setShowPathDropdown] = useState(false);
  const [parentDirectories, setParentDirectories] = useState<DirectoryInfo[]>([]);
  const [childDirectories, setChildDirectories] = useState<DirectoryInfo[]>([]);
  const [dropdownLoading, setDropdownLoading] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(async () => {
      try {
        setLoading(true);
        const repo = await invoke<RepoInfo>('get_repo_info', { path: currentPath });
        if (cancelled) return;
        setRepoInfo(repo);

        if (repo.is_git_repo) {
          const cached = runtimeCache.get(currentPath);
          const runtime = cached && cached.expiresAt > Date.now()
            ? cached.value
            : await invoke<RuntimeInfo>('get_runtime_info', { path: currentPath });
          if (cancelled) return;
          runtimeCache.set(currentPath, {
            expiresAt: Date.now() + RUNTIME_CACHE_MS,
            value: runtime,
          });
          setRuntimeInfo(runtime);
        } else {
          setRuntimeInfo(null);
        }
      } catch (error) {
        if (cancelled) return;
        console.error('❌ TerminalHeader: Error fetching repository info:', error);
        setRepoInfo(null);
        setRuntimeInfo(null);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 120);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [currentPath, refreshToken]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowPathDropdown(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setShowPathDropdown(false);
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
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
        if (!activeSessionId) throw new Error('No active terminal session');
        const workingDirectory = await invoke<string>('change_directory', {
          sessionId: activeSessionId,
          newPath: itemPath,
        });
        onPathChange?.(workingDirectory);
      } else {
        if (!activeSessionId) throw new Error('No active terminal session');
        await invoke('execute_file', {
          sessionId: activeSessionId,
          filePath: itemPath,
        });
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
      <div className="h-11 shrink-0 overflow-hidden border-b border-terminal-border bg-terminal-surface px-3 py-2 text-sm" aria-label="Loading workspace context">
        <div className="flex items-center gap-4">
          <div className="w-48 h-4 bg-terminal-border rounded animate-pulse"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-11 shrink-0 overflow-hidden border-b border-terminal-border bg-terminal-surface px-3 py-2 text-sm">
      <div className="flex min-w-0 items-center gap-2 overflow-hidden text-terminal-muted">
        {/* Repository Section */}
        {repoInfo?.is_git_repo && (
          <div className="hidden min-w-0 items-center gap-2 overflow-hidden rounded-md border border-emerald-500/20 bg-emerald-500/10 px-2 py-1 2xl:flex">
            <div className="flex min-w-0 items-center gap-2">
              <Package size={14} className="text-green-400" />
              <span className="max-w-40 truncate font-medium text-emerald-300">
                {repoInfo.repo_name || 'Unknown Repository'}
              </span>
            </div>
            
            <div className="flex items-center gap-1.5 font-mono text-xs">
              <GitBranch size={12} className="text-terminal-muted" />
              <span className="max-w-28 truncate text-terminal-muted">
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
        <div className="relative min-w-0 flex-1" ref={dropdownRef}>
          <button
            type="button"
            className="flex max-w-full items-center gap-2 rounded-md border border-transparent bg-terminal-border/70 px-2 py-1 text-terminal-text transition-colors hover:border-terminal-muted/30 hover:bg-terminal-border focus:outline-none focus-visible:ring-2 focus-visible:ring-ai-primary"
            onClick={handlePathClick}
            aria-expanded={showPathDropdown}
            aria-haspopup="menu"
            title={currentPath}
          >
            <Code size={12} className="shrink-0 text-terminal-muted" />
            <span className="truncate font-mono text-xs font-medium">{formatPath(currentPath)}</span>
            <ChevronDown 
              size={10} 
              className={`shrink-0 text-terminal-muted transition-transform ${showPathDropdown ? 'rotate-180' : ''}`}
            />
          </button>

          {/* Directory Navigation Dropdown */}
          {showPathDropdown && (
            <div
              className="absolute left-0 top-full z-50 mt-1 max-h-80 w-[min(18rem,calc(100vw-1.5rem))] overflow-y-auto rounded-lg border border-terminal-border bg-terminal-surface py-1 shadow-2xl"
              role="menu"
              aria-label="Workspace navigation"
            >
              {dropdownLoading ? (
                <div className="p-3 text-center text-terminal-muted" role="status">
                  Loading directories...
                </div>
              ) : (
                <>
                  {/* Parent Directories */}
                  {parentDirectories.length > 0 && (
                    <>
                      <div className="px-3 py-2 text-[11px] font-medium text-terminal-muted border-b border-terminal-border uppercase tracking-wide">
                        ↑ Parent Directories
                      </div>
                      {parentDirectories.slice(0, 5).map((dir) => (
                        <button
                          type="button"
                          role="menuitem"
                          key={`parent-${dir.path}`}
                          className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-terminal-text transition-colors hover:bg-terminal-border focus:outline-none focus-visible:bg-terminal-border"
                          onClick={() => handleDirectorySelect(dir.path, true)}
                          title={`Navigate to ${dir.name === '' ? 'root' : dir.name}`}
                        >
                          <Home size={12} className="shrink-0 text-terminal-muted" />
                          <span className="truncate">{dir.name === '' ? '/' : dir.name}</span>
                          <span className="ml-auto text-xs text-terminal-muted">cd →</span>
                        </button>
                      ))}
                    </>
                  )}

                  {/* Child Directories and Files */}
                  {childDirectories.length > 0 && (
                    <>
                      <div className="px-3 py-2 text-[11px] font-medium text-terminal-muted border-b border-terminal-border border-t uppercase tracking-wide">
                        ↓ Contents
                      </div>
                      {childDirectories.slice(0, 10).map((item) => (
                        <button
                          type="button"
                          role="menuitem"
                          key={`child-${item.path}`}
                          className={`flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors focus:outline-none focus-visible:bg-terminal-border ${
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
                          <span className="truncate">{item.name}</span>
                          {item.is_directory && (
                            <span className="ml-auto text-xs text-terminal-muted">cd →</span>
                          )}
                        </button>
                      ))}
                      {childDirectories.length > 10 && (
                        <div className="px-3 py-2 text-xs text-terminal-muted text-center italic">
                          ... and {childDirectories.length - 10} more items
                        </div>
                      )}
                    </>
                  )}

                  {/* No directories found */}
                  {parentDirectories.length === 0 && childDirectories.length === 0 && (
                    <div className="px-3 py-4 text-center text-terminal-muted text-sm italic">
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
          <div className="hidden min-w-0 items-center gap-2 overflow-hidden rounded-md border border-blue-500/20 bg-blue-500/10 px-2 py-1 2xl:flex">
            <div className="flex items-center gap-2">
              {languageInfo.icon}
              <span className="font-medium text-blue-300">
                {languageInfo.name}
              </span>
              <span className="font-mono text-xs text-terminal-muted">
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
