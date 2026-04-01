import React, { useEffect, useRef, useState, useCallback, useImperativeHandle, forwardRef } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import './SessionPlayer.css';

// Asciinema Event Types
export type AsciinemaEventType = 'o' | 'i' | 'r' | 'm';

export interface AsciinemaEvent {
  time: number;
  type: AsciinemaEventType;
  data: string;
}

export interface AsciinemaHeader {
  version: number;
  width: number;
  height: number;
  timestamp?: number;
  duration?: number;
  idle_time_limit?: number;
  command?: string;
  title?: string;
  env?: Record<string, string>;
  shell?: string;
  term?: string;
  theme?: {
    fg?: string;
    bg?: string;
    palette?: string;
  };
}

export interface SessionMark {
  time: number;
  label: string;
  color?: string;
}

export type PlaybackState = 'idle' | 'playing' | 'paused' | 'finished';
export type PlaybackSpeed = 0.5 | 1 | 1.5 | 2 | 4;

export interface SessionPlayerProps {
  /** Recording data URL or content */
  recordingUrl?: string;
  /** Recording content as string */
  recordingContent?: string;
  /** Auto start playback */
  autoPlay?: boolean;
  /** Initial playback speed */
  initialSpeed?: PlaybackSpeed;
  /** Callback when playback finishes */
  onFinish?: () => void;
  /** Callback on time update */
  onTimeUpdate?: (time: number, duration: number) => void;
  /** Callback on mark hit */
  onMark?: (mark: SessionMark) => void;
  /** Custom controls */
  showControls?: boolean;
  /** Custom styles */
  className?: string;
  /** Terminal theme */
  theme?: 'dark' | 'light' | 'custom';
  /** Custom terminal options */
  terminalOptions?: any;
}

export interface SessionPlayerHandle {
  play: () => void;
  pause: () => void;
  stop: () => void;
  seek: (time: number) => void;
  setSpeed: (speed: PlaybackSpeed) => void;
  getCurrentTime: () => number;
  getDuration: () => number;
  getState: () => PlaybackState;
  addMark: (time: number, label: string, color?: string) => void;
  exportToText: () => string;
}

const formatTime = (seconds: number): string => {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const ms = Math.floor((seconds % 1) * 100);
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
};

export const SessionPlayer = forwardRef<SessionPlayerHandle, SessionPlayerProps>(({
  recordingUrl,
  recordingContent,
  autoPlay = false,
  initialSpeed = 1,
  onFinish,
  onTimeUpdate,
  onMark,
  showControls = true,
  className = '',
  theme = 'dark',
  terminalOptions = {},
}, ref) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const animationRef = useRef<number | null>(null);
  const eventsRef = useRef<AsciinemaEvent[]>([]);
  const marksRef = useRef<SessionMark[]>([]);
  const headerRef = useRef<AsciinemaHeader | null>(null);

  const [state, setState] = useState<PlaybackState>('idle');
  const [speed, setSpeed] = useState<PlaybackSpeed>(initialSpeed);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [marks, setMarks] = useState<SessionMark[]>([]);

  // Parse asciinema file content
  const parseAsciinemaContent = useCallback((content: string): boolean => {
    try {
      const lines = content.split('\n').filter(line => line.trim());
      if (lines.length === 0) return false;

      // Parse header
      const header: AsciinemaHeader = JSON.parse(lines[0]);
      headerRef.current = header;

      // Parse events
      const events: AsciinemaEvent[] = [];
      const parsedMarks: SessionMark[] = [];

      for (let i = 1; i < lines.length; i++) {
        const line = lines[i].trim();
        if (!line.startsWith('[') || !line.endsWith(']')) continue;

        try {
          const parts = line.slice(1, -1).split(',');
          if (parts.length >= 3) {
            const time = parseFloat(parts[0].trim());
            const type = parts[1].trim().replace(/"/g, '') as AsciinemaEventType;
            let data = parts[2].trim();

            // Remove quotes and unescape
            if (data.startsWith('"') && data.endsWith('"')) {
              data = JSON.parse(data);
            }

            events.push({ time, type, data });

            // Handle marks
            if (type === 'm') {
              try {
                const markData = JSON.parse(data);
                parsedMarks.push({
                  time,
                  label: markData.label || '',
                  color: markData.color,
                });
              } catch (e) {
                // Mark without JSON data
                parsedMarks.push({ time, label: data, color: '#4CAF50' });
              }
            }
          }
        } catch (e) {
          console.warn('Failed to parse event line:', line, e);
        }
      }

      eventsRef.current = events.sort((a, b) => a.time - b.time);
      marksRef.current = parsedMarks.sort((a, b) => a.time - b.time);
      setMarks(parsedMarks);

      // Calculate duration
      const lastEvent = events[events.length - 1];
      const calculatedDuration = lastEvent ? lastEvent.time : 0;
      setDuration(header.duration || calculatedDuration);

      // Initialize terminal size
      if (terminalRef.current && header.width && header.height) {
        terminalRef.current.resize(header.width, header.height);
      }

      return true;
    } catch (e) {
      setError('Failed to parse recording file');
      return false;
    }
  }, []);

  // Load recording
  useEffect(() => {
    const loadRecording = async () => {
      setIsLoading(true);
      setError(null);

      try {
        let content: string;

        if (recordingContent) {
          content = recordingContent;
        } else if (recordingUrl) {
          const response = await fetch(recordingUrl);
          if (!response.ok) throw new Error('Failed to load recording');
          content = await response.text();
        } else {
          return;
        }

        if (parseAsciinemaContent(content)) {
          setState('idle');
          setCurrentTime(0);

          // Reset terminal
          if (terminalRef.current) {
            terminalRef.current.clear();
            terminalRef.current.reset();
          }

          if (autoPlay) {
            play();
          }
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Unknown error');
      } finally {
        setIsLoading(false);
      }
    };

    loadRecording();
  }, [recordingUrl, recordingContent, autoPlay, parseAsciinemaContent]);

  // Initialize terminal
  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: false,
      scrollback: 10000,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: theme === 'light' ? {
        background: '#ffffff',
        foreground: '#333333',
        cursor: '#333333',
        selectionBackground: '#b5d5ff',
      } : {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        selectionBackground: '#264f78',
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
        brightBlack: '#666666',
        brightRed: '#f14c4c',
        brightGreen: '#23d18b',
        brightYellow: '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: '#e5e5e5',
      },
      ...terminalOptions,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    fitAddon.fit();

    terminalRef.current = term;
    fitAddonRef.current = fitAddon;

    const handleResize = () => {
      fitAddonRef.current?.fit();
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      term.dispose();
      terminalRef.current = null;
    };
  }, [theme, terminalOptions]);

  // Playback loop
  const play = useCallback(() => {
    if (state === 'playing') return;

    setState('playing');
    let lastFrameTime = performance.now();
    let lastEventIndex = 0;

    // Find starting event index
    for (let i = 0; i < eventsRef.current.length; i++) {
      if (eventsRef.current[i].time >= currentTime) {
        lastEventIndex = i;
        break;
      }
    }

    const animate = (now: number) => {
      if (state === 'paused' || state === 'finished') return;

      const deltaTime = (now - lastFrameTime) / 1000 * speed;
      const newTime = Math.min(currentTime + deltaTime, duration);

      // Process events up to current time
      const term = terminalRef.current;
      if (term) {
        while (lastEventIndex < eventsRef.current.length) {
          const event = eventsRef.current[lastEventIndex];
          if (event.time > newTime) break;

          switch (event.type) {
            case 'o': // Output
              term.write(event.data);
              break;
            case 'i': // Input
              // Visual feedback for input (optional)
              break;
            case 'r': // Resize
              const [cols, rows] = event.data.split('x').map(Number);
              if (cols && rows) {
                term.resize(cols, rows);
              }
              break;
            case 'm': // Mark
              const mark = marksRef.current.find(m => Math.abs(m.time - event.time) < 0.001);
              if (mark && onMark) {
                onMark(mark);
              }
              break;
          }

          lastEventIndex++;
        }
      }

      lastFrameTime = now;
      setCurrentTime(newTime);

      if (onTimeUpdate) {
        onTimeUpdate(newTime, duration);
      }

      if (newTime >= duration) {
        setState('finished');
        if (onFinish) onFinish();
        return;
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    animationRef.current = requestAnimationFrame(animate);
  }, [state, currentTime, duration, speed, onTimeUpdate, onFinish, onMark]);

  const pause = useCallback(() => {
    setState('paused');
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
  }, []);

  const stop = useCallback(() => {
    setState('idle');
    setCurrentTime(0);
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
    if (terminalRef.current) {
      terminalRef.current.clear();
      terminalRef.current.reset();
    }
  }, []);

  const seek = useCallback((time: number) => {
    const clampedTime = Math.max(0, Math.min(time, duration));
    setCurrentTime(clampedTime);

    // Re-render terminal at seek position
    if (terminalRef.current) {
      terminalRef.current.clear();
      terminalRef.current.reset();

      // Re-play all events up to seek time
      for (const event of eventsRef.current) {
        if (event.time > clampedTime) break;

        switch (event.type) {
          case 'o':
            terminalRef.current.write(event.data);
            break;
          case 'r':
            const [cols, rows] = event.data.split('x').map(Number);
            if (cols && rows) {
              terminalRef.current.resize(cols, rows);
            }
            break;
        }
      }
    }

    if (state === 'playing') {
      play();
    }
  }, [duration, state, play]);

  const changeSpeed = useCallback((newSpeed: PlaybackSpeed) => {
    setSpeed(newSpeed);
  }, []);

  const addMark = useCallback((time: number, label: string, color?: string) => {
    const newMark: SessionMark = { time, label, color };
    marksRef.current = [...marksRef.current, newMark].sort((a, b) => a.time - b.time);
    setMarks(marksRef.current);
  }, []);

  const exportToText = useCallback((): string => {
    return eventsRef.current
      .filter(e => e.type === 'o')
      .map(e => e.data)
      .join('');
  }, []);

  // Expose imperative handle
  useImperativeHandle(ref, () => ({
    play,
    pause,
    stop,
    seek,
    setSpeed: changeSpeed,
    getCurrentTime: () => currentTime,
    getDuration: () => duration,
    getState: () => state,
    addMark,
    exportToText,
  }), [play, pause, stop, seek, changeSpeed, currentTime, duration, state, addMark, exportToText]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Space: play/pause
      if (e.code === 'Space' && !e.repeat) {
        e.preventDefault();
        if (state === 'playing') {
          pause();
        } else {
          play();
        }
      }
      // Arrow keys: seek
      else if (e.code === 'ArrowLeft') {
        e.preventDefault();
        seek(currentTime - 5);
      }
      else if (e.code === 'ArrowRight') {
        e.preventDefault();
        seek(currentTime + 5);
      }
      // Speed controls
      else if (e.code === 'Digit1') {
        changeSpeed(0.5);
      }
      else if (e.code === 'Digit2') {
        changeSpeed(1);
      }
      else if (e.code === 'Digit3') {
        changeSpeed(1.5);
      }
      else if (e.code === 'Digit4') {
        changeSpeed(2);
      }
      else if (e.code === 'Digit5') {
        changeSpeed(4);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [state, currentTime, play, pause, seek, changeSpeed]);

  // Cleanup animation on unmount
  useEffect(() => {
    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, []);

  return (
    <div className={`session-player ${className}`}>
      {isLoading && (
        <div className="session-player-loading">
          <div className="spinner" />
          <span>Loading recording...</span>
        </div>
      )}

      {error && (
        <div className="session-player-error">
          <span className="error-icon">⚠</span>
          <span>{error}</span>
        </div>
      )}

      <div
        ref={containerRef}
        className={`terminal-container ${state === 'playing' ? 'playing' : ''}`}
      />

      {showControls && (
        <div className="player-controls">
          {/* Play/Pause button */}
          <button
            className="control-btn play-pause"
            onClick={() => state === 'playing' ? pause() : play()}
            title={state === 'playing' ? 'Pause (Space)' : 'Play (Space)'}
          >
            {state === 'playing' ? '⏸' : '▶'}
          </button>

          {/* Stop button */}
          <button
            className="control-btn stop"
            onClick={stop}
            title="Stop"
          >
            ⏹
          </button>

          {/* Time display */}
          <div className="time-display">
            <span className="current-time">{formatTime(currentTime)}</span>
            <span className="time-separator">/</span>
            <span className="duration">{formatTime(duration)}</span>
          </div>

          {/* Timeline scrubber */}
          <div className="timeline-container">
            <input
              type="range"
              min={0}
              max={duration || 1}
              step={0.01}
              value={currentTime}
              onChange={(e) => seek(parseFloat(e.target.value))}
              className="timeline-slider"
            />
            {/* Mark indicators */}
            {marks.map((mark, index) => (
              <div
                key={index}
                className="timeline-mark"
                style={{
                  left: `${(mark.time / duration) * 100}%`,
                  backgroundColor: mark.color || '#4CAF50',
                }}
                title={mark.label}
                onClick={() => seek(mark.time)}
              />
            ))}
          </div>

          {/* Speed controls */}
          <div className="speed-controls">
            {[0.5, 1, 1.5, 2, 4].map((s) => (
              <button
                key={s}
                className={`speed-btn ${speed === s ? 'active' : ''}`}
                onClick={() => changeSpeed(s as PlaybackSpeed)}
                title={`${s}x speed`}
              >
                {s}x
              </button>
            ))}
          </div>

          {/* Marks list toggle */}
          {marks.length > 0 && (
            <button
              className="control-btn marks-toggle"
              title="Show marks"
            >
              📝 {marks.length}
            </button>
          )}
        </div>
      )}

      {/* Marks panel */}
      {marks.length > 0 && (
        <div className="marks-panel">
          <h4>Marks</h4>
          <ul>
            {marks.map((mark, index) => (
              <li
                key={index}
                className={Math.abs(currentTime - mark.time) < 0.5 ? 'active' : ''}
                onClick={() => seek(mark.time)}
              >
                <span
                  className="mark-dot"
                  style={{ backgroundColor: mark.color || '#4CAF50' }}
                />
                <span className="mark-time">{formatTime(mark.time)}</span>
                <span className="mark-label">{mark.label}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
});

SessionPlayer.displayName = 'SessionPlayer';

export default SessionPlayer;
