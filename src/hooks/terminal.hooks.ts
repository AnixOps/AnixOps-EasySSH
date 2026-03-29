import { useCallback, useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';

export interface TerminalInstance {
  id: string;
  terminal: Terminal;
  fitAddon: FitAddon;
  container: HTMLDivElement | null;
}

export function useTerminalInstance(id: string) {
  const instanceRef = useRef<TerminalInstance | null>(null);

  const createTerminal = useCallback((container: HTMLDivElement): TerminalInstance => {
    const terminal = new Terminal({
      theme: {
        background: '#0f172a',
        foreground: '#e2e8f0',
        cursor: '#22d3ee',
        selectionBackground: '#334155',
      },
      fontSize: 14,
      fontFamily: 'JetBrains Mono, Menlo, Monaco, Consolas, monospace',
      cursorBlink: true,
      cursorStyle: 'block',
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(container);
    fitAddon.fit();

    const instance: TerminalInstance = {
      id,
      terminal,
      fitAddon,
      container,
    };

    instanceRef.current = instance;
    return instance;
  }, [id]);

  const disposeTerminal = useCallback(() => {
    if (instanceRef.current) {
      instanceRef.current.terminal.dispose();
      instanceRef.current = null;
    }
  }, []);

  const fitTerminal = useCallback(() => {
    if (instanceRef.current) {
      instanceRef.current.fitAddon.fit();
    }
  }, []);

  useEffect(() => {
    return () => {
      disposeTerminal();
    };
  }, [disposeTerminal]);

  return {
    instance: instanceRef,
    createTerminal,
    disposeTerminal,
    fitTerminal,
  };
}
