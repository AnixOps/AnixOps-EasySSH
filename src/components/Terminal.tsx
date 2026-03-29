import React, { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { SearchAddon } from '@xterm/addon-search';
import { invoke } from '@tauri-apps/api/core';
import '@xterm/xterm/css/xterm.css';
import { useSessionStore } from '../stores/sessionStore';
import { addCommandToHistory, getCommandHistory } from '../stores/commandHistoryStore';
import { StatusDot } from './design-system';

// Terminal theme colors (xterm expects hex values)
const TERMINAL_THEME = {
  background: '#1e1e1e',
  foreground: '#d4d4d4',
  cursor: '#ffffff',
} as const;

interface TerminalProps {
  sessionId: string;
  onClose?: () => void;
  onActivate?: () => void;
}

export const Terminal: React.FC<TerminalProps> = ({ sessionId, onClose, onActivate }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const searchAddonRef = useRef<SearchAddon | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [isSearching, setIsSearching] = useState(false);

  // Track isSearching via ref to avoid stale closure in onData callback
  const isSearchingRef = useRef(isSearching);
  useEffect(() => {
    isSearchingRef.current = isSearching;
  }, [isSearching]);

  const currentCommandRef = useRef('');
  const historyIndexRef = useRef(-1);
  const historyRef = useRef<string[]>([]);

  // 加载命令历史
  const loadHistory = useCallback(() => {
    if (sessionId && sessionId !== 'new') {
      historyRef.current = getCommandHistory(sessionId);
      historyIndexRef.current = historyRef.current.length;
    }
  }, [sessionId]);

  // Focus handler - properly captures onActivate via closure
  const focusTerminal = useCallback(() => {
    onActivate?.();
    terminalRef.current?.focus();
  }, [onActivate]);

  // 清除当前行
  const clearCurrentLine = useCallback(() => {
    const terminal = terminalRef.current;
    if (!terminal) return;

    const len = currentCommandRef.current.length;
    for (let i = 0; i < len; i++) {
      terminal.write('\b \b');
    }
    currentCommandRef.current = '';
  }, []);

  // 显示命令
  const showCommand = useCallback((cmd: string) => {
    const terminal = terminalRef.current;
    if (!terminal) return;

    clearCurrentLine();
    currentCommandRef.current = cmd;
    terminal.write(cmd);
  }, [clearCurrentLine]);

  // 向上箭头 - 上一条命令
  const historyUp = useCallback(() => {
    const history = historyRef.current;
    if (history.length === 0) return;

    if (historyIndexRef.current > 0) {
      historyIndexRef.current -= 1;
      showCommand(history[historyIndexRef.current]);
    }
  }, [showCommand]);

  // 向下箭头 - 下一条命令
  const historyDown = useCallback(() => {
    const history = historyRef.current;
    if (history.length === 0) return;

    if (historyIndexRef.current < history.length - 1) {
      historyIndexRef.current += 1;
      showCommand(history[historyIndexRef.current]);
    } else {
      historyIndexRef.current = history.length;
      clearCurrentLine();
    }
  }, [showCommand, clearCurrentLine]);

  useEffect(() => {
    const terminal = new XTerm({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Consolas, Monaco, "Courier New", monospace',
      theme: TERMINAL_THEME,
      rows: 24,
      cols: 80,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    fitAddonRef.current = fitAddon;

    // 添加搜索插件
    const searchAddon = new SearchAddon();
    terminal.loadAddon(searchAddon);
    searchAddonRef.current = searchAddon;

    // 尝试启用 WebGL 加速 (静默失败，使用canvas回退)
    try {
      const webgl = new WebglAddon();
      terminal.loadAddon(webgl);
    } catch {
      // WebGL不可用时，xterm.js会自动回退到canvas渲染器
    }

    terminal.open(containerRef.current!);
    fitAddon.fit();

    terminalRef.current = terminal;

    // 显示欢迎信息
    terminal.writeln('\x1b[32mEasySSH Terminal\x1b[0m');
    if (sessionId && sessionId !== 'new') {
      terminal.writeln('Session: ' + sessionId.substring(0, 8) + '...');
      terminal.writeln('');
      terminal.writeln('提示: 按 Ctrl+F 搜索, 上/下箭头查看历史');
      terminal.writeln('');
      loadHistory();
      queueMicrotask(() => {
        focusTerminal();
      });
    } else {
      terminal.writeln('');
      terminal.writeln('\x1b[33m请从侧边栏选择一个服务器连接\x1b[0m');
      terminal.writeln('');
    }

    // 处理用户输入
    terminal.onData(async (data) => {
      // 新建终端不允许输入
      if (!sessionId || sessionId === 'new') {
        return;
      }

      // Ctrl+F 搜索
      if (data === '\x06') { // Ctrl+F
        setIsSearching(true);
        return;
      }

      // 搜索模式下的处理
      if (isSearchingRef.current) {
        if (data === '\x1b') { // ESC 退出搜索
          setIsSearching(false);
        } else if (data === '\r') { // Enter 执行搜索
          // 搜索已输入的内容
          const searchTerm = currentCommandRef.current;
          if (searchTerm) {
            searchAddonRef.current?.findNext(searchTerm);
          }
          currentCommandRef.current = '';
          setIsSearching(false);
        } else if (data === '\x7f') { // 退格
          if (currentCommandRef.current.length > 0) {
            currentCommandRef.current = currentCommandRef.current.slice(0, -1);
            terminal.write('\b \b');
          }
        } else if (data >= ' ' && data <= '~') {
          currentCommandRef.current += data;
          terminal.write(data);
        }
        return;
      }

      // 过滤控制字符，只处理可见字符和特定按键
      if (data === '\r') {
        // 回车 - 执行命令
        terminal.writeln('');
        const cmd = currentCommandRef.current.trim();

        if (cmd) {
          // 保存到历史
          addCommandToHistory(sessionId, cmd);
          historyRef.current = getCommandHistory(sessionId);
          historyIndexRef.current = historyRef.current.length;

          // 清空当前命令
          currentCommandRef.current = '';

          // Look up the SSH session ID from the store (sessionId is terminalSessionId)
          const sshSessionId = useSessionStore.getState().terminalSessions.find(s => s.id === sessionId)?.sessionId;
          if (!sshSessionId) {
            terminal.writeln('\x1b[31m错误: Session not found\x1b[0m');
            return;
          }

          try {
            const result = await invoke<string>('ssh_execute', {
              sessionId: sshSessionId,
              command: cmd
            });
            terminal.writeln(result || '(无输出)');
          } catch (e) {
            terminal.writeln(`\x1b[31m错误: ${e}\x1b[0m`);
          }
        }
        terminal.write('\x1b[36m$\x1b[0m ');
      } else if (data === '\x7f') {
        // 退格
        if (currentCommandRef.current.length > 0) {
          currentCommandRef.current = currentCommandRef.current.slice(0, -1);
          terminal.write('\b \b');
        }
      } else if (data === '\x1b[A') {
        // 上箭头
        historyUp();
      } else if (data === '\x1b[B') {
        // 下箭头
        historyDown();
      } else if (data >= ' ' && data <= '~') {
        // 可打印字符
        currentCommandRef.current += data;
        terminal.write(data);
      }
    });

    setIsConnected(true);

    // 窗口/容器大小变化时重新fit
    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
    });
    resizeObserver.observe(containerRef.current!);

    const handleResize = () => {
      fitAddon.fit();
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      resizeObserver.disconnect();
      fitAddon.dispose();
      searchAddon.dispose();
      terminal.dispose();
    };
  }, [sessionId, loadHistory, focusTerminal]);

  const handleReconnect = async () => {
    if (terminalRef.current && sessionId && sessionId !== 'new') {
      terminalRef.current.writeln('\r\n\x1b[33m重新连接中...\x1b[0m\r\n');
      setIsConnected(false);
      // Look up the SSH session ID from the store (sessionId is terminalSessionId)
      const sshSessionId = useSessionStore.getState().terminalSessions.find(s => s.id === sessionId)?.sessionId;
      if (!sshSessionId) {
        terminalRef.current.writeln('\x1b[31m错误: Session not found\x1b[0m\r\n');
        return;
      }
      try {
        await invoke('ssh_disconnect', { sessionId: sshSessionId });
        terminalRef.current.writeln('\x1b[32m已断开连接\x1b[0m\r\n');
        terminalRef.current.writeln('\x1b[33m请重新从侧边栏连接服务器\x1b[0m\r\n');
      } catch (e) {
        terminalRef.current.writeln(`\x1b[31m错误: ${e}\x1b[0m`);
      }
    }
  };

  const handleClear = () => {
    if (terminalRef.current) {
      terminalRef.current.clear();
      if (sessionId && sessionId !== 'new') {
        terminalRef.current.writeln('\x1b[32m已清屏\x1b[0m\r\n');
        terminalRef.current.write('\x1b[36m$\x1b[0m ');
      }
    }
  };

  const handleSearch = () => {
    if (terminalRef.current) {
      // 简单的搜索提示
      terminalRef.current.writeln('\r\n\x1b[33m搜索模式: 输入要搜索的内容，按Enter搜索，ESC退出\x1b[0m\r\n');
      terminalRef.current.write('\x1b[36msearch: \x1b[0m');
      setIsSearching(true);
      currentCommandRef.current = '';
      focusTerminal();
    }
  };

  return (
    <div className="flex flex-col h-full bg-slate-900">
      {/* 终端工具栏 */}
      <div className="flex items-center justify-between px-3 py-1 bg-slate-800 border-b border-slate-700">
        <div className="flex items-center gap-2">
          <StatusDot variant={isConnected ? 'online' : 'offline'} />
          <span className="text-xs text-slate-400">
            {sessionId && sessionId !== 'new' ? `Session: ${sessionId.slice(0, 8)}...` : '未连接'}
          </span>
          {isSearching && (
            <span className="text-xs text-yellow-400">搜索模式</span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleSearch}
            disabled={!isConnected}
            aria-label="Search terminal content"
            className="px-2 py-1 text-xs text-slate-400 hover:text-white hover:bg-slate-700 rounded disabled:opacity-50"
            title="搜索 (Ctrl+F)"
          >
            Search
          </button>
          <button
            onClick={handleClear}
            disabled={!isConnected}
            aria-label="Clear terminal screen"
            className="px-2 py-1 text-xs text-slate-400 hover:text-white hover:bg-slate-700 rounded disabled:opacity-50"
            title="清屏"
          >
            Clear
          </button>
          <button
            onClick={handleReconnect}
            disabled={!isConnected}
            aria-label="Reconnect to server"
            className="px-2 py-1 text-xs text-slate-400 hover:text-white hover:bg-slate-700 rounded disabled:opacity-50"
          >
            Reconnect
          </button>
          {onClose && (
            <button
              onClick={onClose}
              aria-label="Close terminal tab"
              className="px-2 py-1 text-xs text-slate-400 hover:text-white hover:bg-slate-700 rounded"
            >
              Close
            </button>
          )}
        </div>
      </div>

      {/* xterm 容器 */}
      <div
        ref={containerRef}
        className="flex-1 overflow-hidden"
        tabIndex={0}
        role="application"
        aria-label="Terminal output"
        onMouseDown={focusTerminal}
        onFocus={focusTerminal}
      />
    </div>
  );
};
