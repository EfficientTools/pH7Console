import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Folder,
  FolderOpen,
  File,
  Search,
  ChevronRight,
  ChevronDown,
  RefreshCw,
  X,
  FileText,
  Code,
  Image,
  Video,
  Music,
  Archive,
  Settings,
  Terminal,
  ExternalLink,
  Eye,
  Copy,
  Info
} from 'lucide-react';

interface DirectoryInfo {
  name: string;
  path: string;
  is_directory: boolean;
}

interface FileExplorerItem extends DirectoryInfo {
  expanded?: boolean;
  level: number;
  children?: FileExplorerItem[];
  loading?: boolean;
}

interface FileExplorerProps {
  currentPath: string;
  onPathChange?: (newPath: string) => void;
  onFileSelect?: (filePath: string) => void;
  activeSessionId?: string;
}

export const FileExplorer: React.FC<FileExplorerProps> = ({
  currentPath,
  onPathChange,
  onFileSelect,
  activeSessionId
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [fileTree, setFileTree] = useState<FileExplorerItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [searchResults, setSearchResults] = useState<DirectoryInfo[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    item: FileExplorerItem;
  } | null>(null);
  // Close context menu when clicking elsewhere
  useEffect(() => {
    const handleClickOutside = () => {
      setContextMenu(null);
    };

    if (contextMenu) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  }, [contextMenu]);

  const searchInputRef = useRef<HTMLInputElement>(null);

  // Load initial directory
  useEffect(() => {
    loadDirectory(currentPath);
  }, [currentPath]);

  const loadDirectory = async (path: string) => {
    try {
      setLoading(true);
      const children = await invoke<DirectoryInfo[]>('get_child_directories', {
        currentPath: path
      });

      // Create root item for current directory
      const currentDir: FileExplorerItem = {
        name: path.split('/').pop() || 'Root',
        path: path,
        is_directory: true,
        level: 0,
        expanded: true,
        children: children.map(child => ({
          ...child,
          level: 1,
          expanded: false
        }))
      };

      setFileTree([currentDir]);
      setExpandedPaths(new Set([path]));
    } catch (error) {
      console.error('❌ FileExplorer: Failed to load directory:', error);
    } finally {
      setLoading(false);
    }
  };

  const toggleExpand = async (item: FileExplorerItem) => {
    if (!item.is_directory) return;

    const newExpandedPaths = new Set(expandedPaths);

    if (expandedPaths.has(item.path)) {
      // Collapse
      newExpandedPaths.delete(item.path);
      setExpandedPaths(newExpandedPaths);
    } else {
      // Expand and load children if not already loaded
      newExpandedPaths.add(item.path);
      setExpandedPaths(newExpandedPaths);

      if (!item.children) {
        try {
          const children = await invoke<DirectoryInfo[]>('get_child_directories', {
            currentPath: item.path
          });

          // Update tree with loaded children
          setFileTree(prevTree => updateTreeWithChildren(prevTree, item.path, children, item.level + 1));
        } catch {
          console.error('Failed to load directory children');
        }
      }
    }
  };

  const updateTreeWithChildren = (
    tree: FileExplorerItem[],
    targetPath: string,
    children: DirectoryInfo[],
    level: number
  ): FileExplorerItem[] => {
    return tree.map(item => {
      if (item.path === targetPath) {
        return {
          ...item,
          children: children.map(child => ({
            ...child,
            level,
            expanded: false
          }))
        };
      }
      if (item.children) {
        return {
          ...item,
          children: updateTreeWithChildren(item.children, targetPath, children, level)
        };
      }
      return item;
    });
  };

  // Enhanced file type detection and categorization
  const getFileCategory = (fileName: string, isDirectory: boolean) => {
    if (isDirectory) return 'directory';
    
    const extension = fileName.split('.').pop()?.toLowerCase();
    
    // Text/Code files that can be edited
    const textExtensions = ['txt', 'md', 'js', 'jsx', 'ts', 'tsx', 'py', 'rb', 'php', 'java', 'cpp', 'c', 'h', 'go', 'rs', 'swift', 'json', 'yaml', 'yml', 'xml', 'csv', 'html', 'css', 'scss', 'less', 'conf', 'cfg', 'ini', 'env', 'toml', 'sh', 'bash', 'zsh', 'fish', 'vim', 'lua', 'r', 'sql'];
    
    // Image files
    const imageExtensions = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp', 'tiff'];
    
    // Video files
    const videoExtensions = ['mp4', 'mov', 'avi', 'mkv', 'webm', 'm4v', 'wmv', 'flv'];
    
    // Audio files
    const audioExtensions = ['mp3', 'wav', 'flac', 'aac', 'm4a', 'ogg', 'wma'];
    
    // Archive files
    const archiveExtensions = ['zip', 'tar', 'gz', 'rar', '7z', 'bz2', 'xz', 'dmg'];
    
    // Document files
    const documentExtensions = ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'rtf'];
    
    // Executable files
    const executableExtensions = ['exe', 'app', 'deb', 'rpm', 'pkg', 'msi'];
    
    if (textExtensions.includes(extension || '')) return 'text';
    if (imageExtensions.includes(extension || '')) return 'image';
    if (videoExtensions.includes(extension || '')) return 'video';
    if (audioExtensions.includes(extension || '')) return 'audio';
    if (archiveExtensions.includes(extension || '')) return 'archive';
    if (documentExtensions.includes(extension || '')) return 'document';
    if (executableExtensions.includes(extension || '')) return 'executable';
    
    return 'unknown';
  };

  // Enhanced file opening with multiple options
  const openFileWithOption = async (item: FileExplorerItem, option: 'default' | 'editor' | 'terminal' | 'system' | 'reveal') => {
    if (!activeSessionId) {
      alert('Please create a terminal session first by clicking the "+" button in the Terminals tab');
      return;
    }

    try {
      await invoke('file_action', {
        sessionId: activeSessionId,
        filePath: item.path,
        action: option,
      });

    } catch (error) {
      console.error(`❌ FileExplorer: Failed to open file with option ${option}:`, error);
      alert(`Failed to open ${item.name} with ${option}\nError: ${error}`);
    }
  };

  // Main click handler for items
  const handleItemClick = async (item: FileExplorerItem) => {
    if (item.is_directory) {
      // Handle directory navigation
      try {
        if (onPathChange) {
          onPathChange(item.path);
        } else {
          return;
        }

        // Optionally change terminal directory if session is active
        if (activeSessionId) {
          try {
            await invoke('change_directory', {
              sessionId: activeSessionId,
              newPath: item.path
            });
          } catch (invokeError) {
            console.error(`❌ FileExplorer: Failed to change terminal directory:`, invokeError);
          }
        }

      } catch (error) {
        console.error('❌ FileExplorer: Failed to navigate to directory:', error);
      }
    } else {
      // Handle file opening with smart defaults
      await openFileWithOption(item, 'default');
      
      // Call the onFileSelect callback for additional handling
      if (onFileSelect) {
        onFileSelect(item.path);
      }
    }
  };

  // Enhanced context menu handler (right-click)
  const handleContextMenu = (event: React.MouseEvent, item: FileExplorerItem) => {
    event.preventDefault();
    // Set context menu state with position and item
    setContextMenu({
      x: event.clientX,
      y: event.clientY,
      item: item
    });
  };

  const searchDirectories = async (query: string) => {
    if (!query.trim()) {
      setSearchResults([]);
      setIsSearching(false);
      return;
    }

    setIsSearching(true);
    try {
      // Search in current directory and subdirectories
      const children = await invoke<DirectoryInfo[]>('get_child_directories', {
        currentPath: currentPath
      });

      // Filter results based on search query
      const filtered = children.filter(item =>
        item.name.toLowerCase().includes(query.toLowerCase())
      );

      setSearchResults(filtered);
    } catch (error) {
      console.error('Search failed:', error);
      setSearchResults([]);
    } finally {
      setIsSearching(false);
    }
  };

  const getFileIcon = (fileName: string, isDirectory: boolean) => {
    if (isDirectory) {
      return <Folder className="w-3 h-3 text-blue-500" />;
    }

    const extension = fileName.split('.').pop()?.toLowerCase();
    switch (extension) {
      case 'js':
      case 'jsx':
      case 'ts':
      case 'tsx':
      case 'py':
      case 'rb':
      case 'php':
      case 'java':
      case 'cpp':
      case 'c':
      case 'h':
      case 'go':
      case 'rs':
      case 'swift':
        return <Code className="w-3 h-3 text-green-400" />;

      case 'txt':
      case 'md':
      case 'json':
      case 'yaml':
      case 'yml':
      case 'xml':
      case 'csv':
        return <FileText className="w-3 h-3 text-blue-400" />;

      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'gif':
      case 'svg':
      case 'webp':
      case 'ico':
        return <Image className="w-3 h-3 text-purple-400" />;

      case 'mp4':
      case 'mov':
      case 'avi':
      case 'mkv':
      case 'webm':
        return <Video className="w-3 h-3 text-red-400" />;

      case 'mp3':
      case 'wav':
      case 'flac':
      case 'aac':
        return <Music className="w-3 h-3 text-pink-400" />;

      case 'zip':
      case 'tar':
      case 'gz':
      case 'rar':
      case '7z':
        return <Archive className="w-3 h-3 text-orange-400" />;

      case 'toml':
      case 'cfg':
      case 'conf':
      case 'ini':
      case 'env':
        return <Settings className="w-3 h-3 text-gray-400" />;

      default:
        return <File className="w-3 h-3 text-terminal-muted" />;
    }
  };

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setSearchQuery(value);

    // Debounce search
    const timeoutId = setTimeout(() => {
      searchDirectories(value);
    }, 300);

    return () => clearTimeout(timeoutId);
  };

  const clearSearch = () => {
    setSearchQuery('');
    setSearchResults([]);
    setIsSearching(false);
  };

  const renderTreeItem = (item: FileExplorerItem): React.ReactNode => {
    const isExpanded = expandedPaths.has(item.path);
    const hasChildren = item.is_directory && (item.children?.length || 0) > 0;
    const category = getFileCategory(item.name, item.is_directory);

    const handleArrowClick = (e: React.MouseEvent) => {
      e.stopPropagation(); // Prevent the item click
      toggleExpand(item);
    };

    // Generate smart tooltip based on file type
    const getTooltip = () => {
      if (item.is_directory) {
        return `📁 ${item.name} - Click to navigate, Right-click for options`;
      }
      
      switch (category) {
        case 'text':
          return `📝 ${item.name} - Click to open in editor (VS Code/nano), Right-click for more options`;
        case 'image':
          return `🖼️ ${item.name} - Click to open with system viewer, Right-click for options`;
        case 'video':
          return `🎥 ${item.name} - Click to open with system player, Right-click for options`;
        case 'audio':
          return `🎵 ${item.name} - Click to open with system player, Right-click for options`;
        case 'document':
          return `📄 ${item.name} - Click to open with system default, Right-click for options`;
        case 'archive':
          return `📦 ${item.name} - Archive file, Right-click to extract or view`;
        case 'executable':
          return `⚙️ ${item.name} - Executable file, Right-click to run or inspect`;
        default:
          return `📄 ${item.name} - Right-click for options`;
      }
    };

    return (
      <div key={item.path} className="select-none">
        <div
          className="flex items-center gap-1 py-2 px-2 hover:bg-terminal-border rounded cursor-pointer transition-colors group active:bg-ai-primary/20 min-h-[28px]"
          style={{ paddingLeft: `${item.level * 16 + 8}px` }}
          onClick={() => {
            handleItemClick(item);
          }}
          onContextMenu={(e) => handleContextMenu(e, item)}
          title={getTooltip()}
        >
          {/* Expand/Collapse Arrow */}
          {item.is_directory && (
            <div
              className="w-4 h-4 flex items-center justify-center hover:bg-terminal-border rounded"
              onClick={handleArrowClick}
              title={isExpanded ? "Collapse" : "Expand"}
            >
              {hasChildren ? (
                isExpanded ? (
                  <ChevronDown className="w-3 h-3 text-terminal-muted" />
                ) : (
                  <ChevronRight className="w-3 h-3 text-terminal-muted" />
                )
              ) : null}
            </div>
          )}

          {/* Icon */}
          <div className="w-4 h-4 flex items-center justify-center">
            {item.is_directory ? (
              isExpanded ? (
                <FolderOpen className="w-3 h-3 text-blue-400" />
              ) : (
                <Folder className="w-3 h-3 text-blue-500" />
              )
            ) : (
              getFileIcon(item.name, false)
            )}
          </div>

          {/* Name */}
          <span
            className={`text-sm truncate ${item.is_directory ? 'text-terminal-text' : 'text-terminal-muted'
              } group-hover:text-white transition-colors`}
            title={`${item.name} - ${item.is_directory ? 'Click to navigate to directory' : 'Click to open in editor (nano for text files)'}`}
          >
            {item.name}
          </span>
        </div>

        {/* Render children if expanded */}
        {item.is_directory && isExpanded && item.children && (
          <div>
            {item.children.map(child => renderTreeItem(child))}
          </div>
        )}
      </div>
    );
  };

  const renderSearchResults = () => {
    if (!searchQuery) return null;

    return (
      <div className="mt-2">
        <div className="text-xs text-terminal-muted px-2 py-1 border-b border-terminal-border">
          {isSearching ? 'Searching...' : `${searchResults.length} results`}
        </div>
        <div className="max-h-64 overflow-y-auto">
          {searchResults.map(item => {
            const category = getFileCategory(item.name, item.is_directory);
            const getSearchTooltip = () => {
              if (item.is_directory) {
                return `📁 ${item.name} - Click to navigate, Right-click for options`;
              }
              switch (category) {
                case 'text':
                  return `📝 ${item.name} - Click to open in editor, Right-click for more options`;
                case 'image':
                  return `🖼️ ${item.name} - Click to open with system viewer`;
                case 'video':
                  return `🎥 ${item.name} - Click to open with system player`;
                case 'audio':
                  return `🎵 ${item.name} - Click to open with system player`;
                default:
                  return `📄 ${item.name} - Click to open, Right-click for options`;
              }
            };

            return (
              <div
                key={item.path}
                className="flex items-center gap-2 py-2 px-2 hover:bg-terminal-border rounded cursor-pointer transition-colors group min-h-[28px]"
                onClick={() => handleItemClick({ ...item, level: 0 })}
                onContextMenu={(e) => handleContextMenu(e, { ...item, level: 0 })}
                title={getSearchTooltip()}
              >
                <div className="w-4 h-4 flex items-center justify-center">
                  {getFileIcon(item.name, item.is_directory)}
                </div>
                <span
                  className={`text-sm truncate ${item.is_directory ? 'text-terminal-text' : 'text-terminal-muted'
                    } group-hover:text-white transition-colors`}
                >
                  {item.name}
                </span>
                <span className="text-xs text-terminal-muted ml-auto opacity-60">
                  {category !== 'directory' ? category : 'folder'}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div className="h-full flex flex-col bg-terminal-surface">
      {/* Header */}
      <div className="p-3 border-b border-terminal-border">
        <div className="flex items-center justify-between mb-2">
          <div>
            <h2 className="text-sm font-semibold text-terminal-text">Explorer</h2>
            <div className="text-xs text-terminal-muted truncate" title={currentPath}>
              {currentPath}
            </div>
          </div>
          <div className="flex items-center gap-1">
            {!activeSessionId && (
              <button
                onClick={() => {
                  alert('Please create a terminal session first by clicking the "+" button in the Terminals tab');
                }}
                className="p-1 bg-orange-500 hover:bg-orange-600 rounded transition-colors text-xs text-white"
                title="Create Terminal Session"
              >
                No Terminal
              </button>
            )}
            <button
              onClick={() => {
                const parentPath = currentPath.split('/').slice(0, -1).join('/') || '/';
                if (parentPath !== currentPath) {
                  onPathChange?.(parentPath);
                }
              }}
              className="p-1 hover:bg-terminal-border rounded transition-colors"
              title="Go to Parent Directory"
            >
              <ChevronRight className="w-3 h-3 text-terminal-muted rotate-180" />
            </button>
            <button
              onClick={() => loadDirectory(currentPath)}
              className="p-1 hover:bg-terminal-border rounded transition-colors"
              title="Refresh"
            >
              <RefreshCw className="w-3 h-3 text-terminal-muted" />
            </button>
          </div>
        </div>

        {/* Search Input */}
        <div className="relative mb-2">
          <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 w-3 h-3 text-terminal-muted z-10" />
          <input
            ref={searchInputRef}
            type="text"
            placeholder="Search directories/files..."
            value={searchQuery}
            onChange={handleSearchChange}
            className="w-full pl-7 pr-7 py-2 bg-terminal-bg border border-terminal-border rounded text-sm text-terminal-text placeholder-terminal-muted focus:outline-none focus:border-ai-primary transition-colors"
          />
          {searchQuery && (
            <button
              onClick={clearSearch}
              className="absolute right-2 top-1/2 transform -translate-y-1/2 p-0.5 hover:bg-terminal-border rounded z-10"
            >
              <X className="w-3 h-3 text-terminal-muted" />
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="p-4 text-center text-terminal-muted">
            <RefreshCw className="w-4 h-4 mx-auto mb-2 animate-spin" />
            <p className="text-sm">Loading directories...</p>
          </div>
        ) : searchQuery ? (
          renderSearchResults()
        ) : (
          <div className="p-2">
            {fileTree.length === 0 ? (
              <div className="text-terminal-muted text-sm">No items to display</div>
            ) : (
              fileTree.map(item => renderTreeItem(item))
            )}
          </div>
        )}
      </div>

      {/* Enhanced Context Menu with modern categorization */}
      {contextMenu && (
        <div
          className="fixed bg-terminal-bg border border-terminal-border rounded-lg shadow-xl py-2 z-50 min-w-[220px]"
          style={{
            left: Math.min(contextMenu.x, window.innerWidth - 250),
            top: Math.min(contextMenu.y, window.innerHeight - 300),
          }}
          onMouseLeave={() => setContextMenu(null)}
        >
          {/* Primary actions */}
          <div className="px-1">
            <div className="text-xs text-terminal-muted px-3 py-1 font-medium uppercase tracking-wide">
              {contextMenu.item.is_directory ? 'Directory Actions' : 'File Actions'}
            </div>
            
            {contextMenu.item.is_directory ? (
              <>
                <button
                  onClick={() => {
                    handleItemClick(contextMenu.item);
                    setContextMenu(null);
                  }}
                  className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
                >
                  <Folder className="w-4 h-4" />
                  <span>Open Directory</span>
                </button>
                <button
                  onClick={async () => {
                    if (activeSessionId) {
                      try {
                        await invoke('change_directory', {
                          sessionId: activeSessionId,
                          newPath: contextMenu.item.path
                        });
                      } catch (error) {
                        console.error('Failed to change terminal directory:', error);
                      }
                    }
                    setContextMenu(null);
                  }}
                  className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
                >
                  <Terminal className="w-4 h-4" />
                  <span>Open in Terminal</span>
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={async () => {
                    await openFileWithOption(contextMenu.item, 'default');
                    setContextMenu(null);
                  }}
                  className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
                >
                  <ExternalLink className="w-4 h-4" />
                  <span>Smart Open</span>
                  <span className="text-xs text-terminal-muted ml-auto">
                    {getFileCategory(contextMenu.item.name, false)}
                  </span>
                </button>
                <button
                  onClick={async () => {
                    await openFileWithOption(contextMenu.item, 'editor');
                    setContextMenu(null);
                  }}
                  className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
                >
                  <FileText className="w-4 h-4" />
                  <span>Open in Editor</span>
                  <span className="text-xs text-terminal-muted ml-auto">macOS</span>
                </button>
              </>
            )}
          </div>

          {/* Secondary actions */}
          <div className="border-t border-terminal-border mt-2 pt-2 px-1">
            <div className="text-xs text-terminal-muted px-3 py-1 font-medium uppercase tracking-wide">
              System Actions
            </div>
            <button
              onClick={async () => {
                await openFileWithOption(contextMenu.item, 'reveal');
                setContextMenu(null);
              }}
              className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
            >
              <Eye className="w-4 h-4" />
              <span>Reveal in Finder</span>
            </button>
            <button
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(contextMenu.item.path);
                } catch (error) {
                  console.error('Failed to copy path:', error);
                }
                setContextMenu(null);
              }}
              className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
            >
              <Copy className="w-4 h-4" />
              <span>Copy Path</span>
            </button>
            <button
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(contextMenu.item.name);
                } catch (error) {
                  console.error('Failed to copy name:', error);
                }
                setContextMenu(null);
              }}
              className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
            >
              <Copy className="w-4 h-4" />
              <span>Copy Name</span>
            </button>
            
            {/* File properties for files only */}
            {!contextMenu.item.is_directory && (
              <button
                onClick={async () => {
                  if (activeSessionId) {
                    try {
                      await invoke('file_action', {
                        sessionId: activeSessionId,
                        filePath: contextMenu.item.path,
                        action: 'properties',
                      });
                    } catch (error) {
                      console.error('Failed to get file properties:', error);
                    }
                  }
                  setContextMenu(null);
                }}
                className="w-full flex items-center gap-3 px-3 py-2 text-sm text-terminal-text hover:bg-terminal-border rounded transition-colors"
              >
                <Info className="w-4 h-4" />
                <span>Properties</span>
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
