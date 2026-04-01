import React, { useState, useEffect, useCallback } from 'react';
import { SessionRecordingManager, RecordingMetadata, RecordingConfig, RecordingState } from '../../../core/src/session_recording';
import './RecordingManager.css';

export interface RecordingManagerProps {
  /** Session ID to associate with recordings */
  sessionId?: string;
  /** Server ID to associate with recordings */
  serverId?: string;
  /** Callback when recording starts */
  onRecordingStart?: (recordingId: string) => void;
  /** Callback when recording stops */
  onRecordingStop?: (metadata: RecordingMetadata) => void;
  /** Callback when recording state changes */
  onStateChange?: (state: RecordingState) => void;
}

export const RecordingManager: React.FC<RecordingManagerProps> = ({
  sessionId,
  serverId,
  onRecordingStart,
  onRecordingStop,
  onStateChange,
}) => {
  const [manager, setManager] = useState<SessionRecordingManager | null>(null);
  const [recordings, setRecordings] = useState<RecordingMetadata[]>([]);
  const [activeRecording, setActiveRecording] = useState<string | null>(null);
  const [recordingState, setRecordingState] = useState<RecordingState>('idle');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);

  // Recording settings
  const [settings, setSettings] = useState<Partial<RecordingConfig>>({
    title: '',
    recordInput: true,
    enablePrivacyFilter: true,
    autoMarkCommands: true,
    idleTimeLimit: 1.0,
  });

  // Initialize manager
  useEffect(() => {
    const initManager = async () => {
      try {
        // @ts-ignore - Rust FFI
        const mgr = await SessionRecordingManager.new('./recordings');
        setManager(mgr);
        const list = await mgr.list_recordings();
        setRecordings(list);
      } catch (e) {
        setError('Failed to initialize recording manager');
      }
    };

    initManager();
  }, []);

  // Start recording
  const startRecording = useCallback(async () => {
    if (!manager) return;

    setIsLoading(true);
    setError(null);

    try {
      const config: RecordingConfig = {
        width: 80,
        height: 24,
        title: settings.title || undefined,
        recordInput: settings.recordInput ?? true,
        enablePrivacyFilter: settings.enablePrivacyFilter ?? true,
        autoMarkCommands: settings.autoMarkCommands ?? true,
        idleTimeLimit: settings.idleTimeLimit,
        outputDir: './recordings',
      };

      const recordingId = await manager.start_recording(config, serverId);
      setActiveRecording(recordingId);
      setRecordingState('recording');

      if (onRecordingStart) {
        onRecordingStart(recordingId);
      }

      setShowSettings(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to start recording');
    } finally {
      setIsLoading(false);
    }
  }, [manager, settings, serverId, onRecordingStart]);

  // Stop recording
  const stopRecording = useCallback(async () => {
    if (!manager || !activeRecording) return;

    setIsLoading(true);

    try {
      const metadata = await manager.stop_recording(activeRecording);
      setActiveRecording(null);
      setRecordingState('idle');
      setRecordings(prev => [metadata, ...prev]);

      if (onRecordingStop) {
        onRecordingStop(metadata);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to stop recording');
    } finally {
      setIsLoading(false);
    }
  }, [manager, activeRecording, onRecordingStop]);

  // Pause/Resume recording
  const togglePause = useCallback(async () => {
    if (!manager || !activeRecording) return;

    try {
      if (recordingState === 'recording') {
        await manager.pause_recording(activeRecording);
        setRecordingState('paused');
      } else if (recordingState === 'paused') {
        await manager.resume_recording(activeRecording);
        setRecordingState('recording');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to toggle pause');
    }
  }, [manager, activeRecording, recordingState]);

  // Add mark during recording
  const addMark = useCallback(async (label: string) => {
    if (!manager || !activeRecording) return;

    try {
      await manager.add_mark(activeRecording, label, '#FF9800');
    } catch (e) {
      console.error('Failed to add mark:', e);
    }
  }, [manager, activeRecording]);

  // Delete recording
  const deleteRecording = useCallback(async (recordingId: string) => {
    if (!manager) return;

    if (!confirm('Are you sure you want to delete this recording?')) return;

    try {
      await manager.delete_recording(recordingId);
      setRecordings(prev => prev.filter(r => r.id !== recordingId));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete recording');
    }
  }, [manager]);

  // Export recording
  const exportRecording = useCallback(async (recordingId: string, format: 'asciicast' | 'json' | 'text' | 'gif') => {
    if (!manager) return;

    try {
      // @ts-ignore
      const player = await manager.get_player(recordingId);
      const text = player.exportToText ? player.exportToText() : '';

      // Create and download file
      const blob = new Blob([text], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `recording-${recordingId}.${format === 'text' ? 'txt' : 'cast'}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to export recording');
    }
  }, [manager]);

  // Search in recordings
  const searchRecordings = useCallback(async (query: string) => {
    if (!manager || !query.trim()) {
      const all = await manager?.list_recordings();
      setRecordings(all || []);
      return;
    }

    try {
      // @ts-ignore
      const results = await manager.search_all_recordings(query);
      // Filter recordings that have matches
      const matchedIds = Object.keys(results);
      const allRecordings = await manager.list_recordings();
      setRecordings(allRecordings.filter(r => matchedIds.includes(r.id)));
    } catch (e) {
      console.error('Search failed:', e);
    }
  }, [manager]);

  // Update state callback
  useEffect(() => {
    if (onStateChange) {
      onStateChange(recordingState);
    }
  }, [recordingState, onStateChange]);

  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <div className="recording-manager">
      {/* Header */}
      <div className="recording-header">
        <h3>Session Recordings</h3>

        {/* Recording controls */}
        <div className="recording-controls">
          {!activeRecording ? (
            <button
              className="btn-record"
              onClick={() => setShowSettings(true)}
              disabled={isLoading}
            >
              <span className="record-icon">●</span>
              Start Recording
            </button>
          ) : (
            <>
              <div className={`recording-status ${recordingState}`}>
                <span className="status-indicator" />
                {recordingState === 'recording' ? 'Recording' : 'Paused'}
              </div>

              <button
                className="btn-pause"
                onClick={togglePause}
                disabled={isLoading}
              >
                {recordingState === 'recording' ? '⏸ Pause' : '▶ Resume'}
              </button>

              <button
                className="btn-mark"
                onClick={() => {
                  const label = prompt('Enter mark label:');
                  if (label) addMark(label);
                }}
              >
                📝 Add Mark
              </button>

              <button
                className="btn-stop"
                onClick={stopRecording}
                disabled={isLoading}
              >
                ⏹ Stop
              </button>
            </>
          )}
        </div>
      </div>

      {/* Error message */}
      {error && (
        <div className="recording-error">
          <span className="error-icon">⚠</span>
          {error}
          <button onClick={() => setError(null)}>×</button>
        </div>
      )}

      {/* Settings modal */}
      {showSettings && (
        <div className="settings-modal">
          <div className="settings-content">
            <h4>Recording Settings</h4>

            <label>
              Title (optional):
              <input
                type="text"
                value={settings.title || ''}
                onChange={e => setSettings(s => ({ ...s, title: e.target.value }))}
                placeholder="Enter recording title..."
              />
            </label>

            <label className="checkbox">
              <input
                type="checkbox"
                checked={settings.recordInput}
                onChange={e => setSettings(s => ({ ...s, recordInput: e.target.checked }))}
              />
              Record user input
            </label>

            <label className="checkbox">
              <input
                type="checkbox"
                checked={settings.enablePrivacyFilter}
                onChange={e => setSettings(s => ({ ...s, enablePrivacyFilter: e.target.checked }))}
              />
              Filter sensitive data (passwords, keys)
            </label>

            <label className="checkbox">
              <input
                type="checkbox"
                checked={settings.autoMarkCommands}
                onChange={e => setSettings(s => ({ ...s, autoMarkCommands: e.target.checked }))}
              />
              Auto-mark commands
            </label>

            <div className="settings-actions">
              <button
                className="btn-cancel"
                onClick={() => setShowSettings(false)}
              >
                Cancel
              </button>
              <button
                className="btn-start"
                onClick={startRecording}
                disabled={isLoading}
              >
                {isLoading ? 'Starting...' : 'Start Recording'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Search bar */}
      <div className="recording-search">
        <input
          type="text"
          placeholder="Search in recordings..."
          onChange={e => searchRecordings(e.target.value)}
        />
      </div>

      {/* Recordings list */}
      <div className="recordings-list">
        {recordings.length === 0 ? (
          <div className="no-recordings">
            <p>No recordings yet</p>
            <p className="hint">Click "Start Recording" to capture your terminal session</p>
          </div>
        ) : (
          recordings.map(recording => (
            <div key={recording.id} className="recording-item">
              <div className="recording-info">
                <h4>{recording.title || `Recording ${recording.id.slice(0, 8)}`}</h4>
                <div className="recording-meta">
                  <span>⏱ {formatDuration(recording.duration)}</span>
                  <span>📦 {formatFileSize(recording.file_size)}</span>
                  <span>📝 {recording.command_count} marks</span>
                  <span>📅 {formatDate(recording.created_at)}</span>
                  {recording.has_input && <span className="badge">Input captured</span>}
                </div>
              </div>

              <div className="recording-actions">
                <button
                  className="btn-play"
                  title="Play"
                >
                  ▶
                </button>

                <div className="export-menu">
                  <button className="btn-export" title="Export">
                    💾
                  </button>
                  <div className="export-options">
                    <button onClick={() => exportRecording(recording.id, 'text')}>
                      Export as Text
                    </button>
                    <button onClick={() => exportRecording(recording.id, 'asciicast')}>
                      Export as Asciicast
                    </button>
                    <button onClick={() => exportRecording(recording.id, 'json')}>
                      Export as JSON
                    </button>
                  </div>
                </div>

                <button
                  className="btn-delete"
                  onClick={() => deleteRecording(recording.id)}
                  title="Delete"
                >
                  🗑
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Storage info */}
      <div className="storage-info">
        <span>Total: {recordings.length} recordings</span>
        <span>
          Storage: {formatFileSize(recordings.reduce((sum, r) => sum + r.file_size, 0))}
        </span>
      </div>
    </div>
  );
};

export default RecordingManager;
