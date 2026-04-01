import React, { useState, useCallback } from 'react';
import './ExportManager.css';

export type ExportFormat = 'asciicast' | 'json' | 'text' | 'gif' | 'mp4';

export interface ExportOptions {
  format: ExportFormat;
  startTime?: number;
  endTime?: number;
  width?: number;
  height?: number;
  quality?: number;
}

export interface ExportManagerProps {
  recordingId: string;
  recordingPath: string;
  duration: number;
  onExport?: (url: string, format: ExportFormat) => void;
  onClose?: () => void;
}

export const ExportManager: React.FC<ExportManagerProps> = ({
  recordingId,
  recordingPath,
  duration,
  onExport,
  onClose,
}) => {
  const [selectedFormat, setSelectedFormat] = useState<ExportFormat>('asciicast');
  const [trimEnabled, setTrimEnabled] = useState(false);
  const [trimStart, setTrimStart] = useState(0);
  const [trimEnd, setTrimEnd] = useState(duration);
  const [isExporting, setIsExporting] = useState(false);
  const [exportProgress, setExportProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [showCloudOptions, setShowCloudOptions] = useState(false);

  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const handleExport = useCallback(async () => {
    setIsExporting(true);
    setExportProgress(0);
    setError(null);

    try {
      const options: ExportOptions = {
        format: selectedFormat,
        startTime: trimEnabled ? trimStart : undefined,
        endTime: trimEnabled ? trimEnd : undefined,
      };

      // Simulate progress
      const progressInterval = setInterval(() => {
        setExportProgress(prev => {
          if (prev >= 90) {
            clearInterval(progressInterval);
            return 90;
          }
          return prev + Math.random() * 15;
        });
      }, 200);

      // Call Rust export function
      // @ts-ignore
      const result = await window.electron?.invoke('export_recording', {
        recordingId,
        options,
      });

      clearInterval(progressInterval);
      setExportProgress(100);

      if (result?.success && onExport) {
        onExport(result.filePath, selectedFormat);
      }

      // Delay close to show completion
      setTimeout(() => {
        onClose?.();
      }, 500);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setIsExporting(false);
    }
  }, [recordingId, selectedFormat, trimEnabled, trimStart, trimEnd, onExport, onClose]);

  const handleUploadToAsciinema = useCallback(async () => {
    setIsExporting(true);
    setError(null);

    try {
      // @ts-ignore
      const result = await window.electron?.invoke('upload_to_asciinema', {
        recordingId,
        title: `EasySSH Recording - ${new Date().toLocaleString()}`,
      });

      if (result?.url) {
        // Copy link to clipboard
        navigator.clipboard.writeText(result.url);
        alert(`Uploaded! Link copied to clipboard: ${result.url}`);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Upload failed');
    } finally {
      setIsExporting(false);
    }
  }, [recordingId]);

  const formatOptions = [
    { id: 'asciicast' as ExportFormat, label: 'Asciicast', desc: 'Original format, best quality', icon: '📝' },
    { id: 'json' as ExportFormat, label: 'JSON', desc: 'Structured data format', icon: '📊' },
    { id: 'text' as ExportFormat, label: 'Plain Text', desc: 'Extracted output only', icon: '📄' },
    { id: 'gif' as ExportFormat, label: 'GIF', desc: 'Animated image (requires agg)', icon: '🎬' },
    { id: 'mp4' as ExportFormat, label: 'MP4', desc: 'Video format (requires ffmpeg)', icon: '🎥' },
  ];

  return (
    <div className="export-manager">
      <div className="export-header">
        <h3>Export Recording</h3>
        <button className="btn-close" onClick={onClose}>×</button>
      </div>

      {error && (
        <div className="export-error">
          <span className="error-icon">⚠</span>
          {error}
        </div>
      )}

      {/* Format Selection */}
      <div className="export-section">
        <h4>Select Format</h4>
        <div className="format-grid">
          {formatOptions.map(format => (
            <button
              key={format.id}
              className={`format-option ${selectedFormat === format.id ? 'selected' : ''}`}
              onClick={() => setSelectedFormat(format.id)}
              disabled={isExporting}
            >
              <span className="format-icon">{format.icon}</span>
              <span className="format-label">{format.label}</span>
              <span className="format-desc">{format.desc}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Trim Options */}
      <div className="export-section">
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={trimEnabled}
            onChange={(e) => setTrimEnabled(e.target.checked)}
            disabled={isExporting}
          />
          Trim recording
        </label>

        {trimEnabled && (
          <div className="trim-controls">
            <div className="trim-range">
              <label>
                Start: {formatDuration(trimStart)}
                <input
                  type="range"
                  min={0}
                  max={duration - 1}
                  step={0.1}
                  value={trimStart}
                  onChange={(e) => {
                    const val = parseFloat(e.target.value);
                    setTrimStart(Math.min(val, trimEnd - 1));
                  }}
                  disabled={isExporting}
                />
              </label>
              <label>
                End: {formatDuration(trimEnd)}
                <input
                  type="range"
                  min={1}
                  max={duration}
                  step={0.1}
                  value={trimEnd}
                  onChange={(e) => {
                    const val = parseFloat(e.target.value);
                    setTrimEnd(Math.max(val, trimStart + 1));
                  }}
                  disabled={isExporting}
                />
              </label>
            </div>

            {/* Visual timeline preview */}
            <div className="trim-preview">
              <div className="timeline-bar">
                <div
                  className="trim-start-handle"
                  style={{ left: `${(trimStart / duration) * 100}%` }}
                />
                <div
                  className="trim-selected"
                  style={{
                    left: `${(trimStart / duration) * 100}%`,
                    width: `${((trimEnd - trimStart) / duration) * 100}%`,
                  }}
                />
                <div
                  className="trim-end-handle"
                  style={{ left: `${(trimEnd / duration) * 100}%` }}
                />
              </div>
              <div className="trim-info">
                Selected: {formatDuration(trimEnd - trimStart)}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Cloud sharing */}
      <div className="export-section">
        <button
          className="btn-cloud-toggle"
          onClick={() => setShowCloudOptions(!showCloudOptions)}
          disabled={isExporting}
        >
          ☁️ Cloud Sharing {showCloudOptions ? '▲' : '▼'}
        </button>

        {showCloudOptions && (
          <div className="cloud-options">
            <p className="cloud-desc">
              Upload to asciinema.org to share with others
            </p>
            <button
              className="btn-upload"
              onClick={handleUploadToAsciinema}
              disabled={isExporting}
            >
              {isExporting ? 'Uploading...' : 'Upload to asciinema.org'}
            </button>
          </div>
        )}
      </div>

      {/* Export Progress */}
      {isExporting && (
        <div className="export-progress">
          <div
            className="progress-bar"
            style={{ width: `${Math.min(exportProgress, 100)}%` }}
          />
          <span className="progress-text">{Math.round(exportProgress)}%</span>
        </div>
      )}

      {/* Actions */}
      <div className="export-actions">
        <button
          className="btn-cancel"
          onClick={onClose}
          disabled={isExporting}
        >
          Cancel
        </button>
        <button
          className="btn-export"
          onClick={handleExport}
          disabled={isExporting}
        >
          {isExporting ? 'Exporting...' : `Export as ${selectedFormat.toUpperCase()}`}
        </button>
      </div>

      {/* Format tips */}
      <div className="format-tips">
        <h4>💡 Tips</h4>
        <ul>
          <li><strong>Asciicast:</strong> Best for archiving and re-playing in EasySSH</li>
          <li><strong>Text:</strong> Great for sharing logs or documentation</li>
          <li><strong>GIF:</strong> Perfect for embedding in README files or blog posts</li>
          <li><strong>MP4:</strong> Universal video format for any platform</li>
        </ul>
      </div>
    </div>
  );
};

export default ExportManager;
