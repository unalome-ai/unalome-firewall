import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ShieldCheck,
  Search,
  Code,
  Braces,
  FileText,
  File,
  Trash2,
  RotateCcw,
  Eye,
  X,
  ChevronDown,
  ChevronRight,
  Settings,
  AlertTriangle,
} from "lucide-react";
import type { ProtectedFile, SafetyNetStats, RestoreResult } from "@/types";
import { cn } from "@/lib/utils";

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function timeAgo(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

function getDateGroup(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - date.getTime()) / 86400000);
  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return "This Week";
  return "Older";
}

function getFileIcon(path: string) {
  const ext = path.split(".").pop()?.toLowerCase() || "";
  if (["ts", "tsx", "js", "jsx", "py", "rs", "go", "java", "c", "cpp", "rb", "php", "swift", "kt"].includes(ext))
    return Code;
  if (["json", "yaml", "yml", "toml", "xml"].includes(ext)) return Braces;
  if (["md", "txt", "csv"].includes(ext)) return FileText;
  return File;
}

function getFileName(path: string): { name: string; dir: string } {
  const parts = path.split("/");
  const name = parts.pop() || path;
  const dir = parts.length > 2 ? ".../" + parts.slice(-2).join("/") : parts.join("/");
  return { name, dir };
}

const TIME_PERIODS = [
  { label: "1h", ms: 60 * 60 * 1000 },
  { label: "24h", ms: 24 * 60 * 60 * 1000 },
  { label: "7d", ms: 7 * 24 * 60 * 60 * 1000 },
  { label: "30d", ms: 30 * 24 * 60 * 60 * 1000 },
  { label: "All", ms: 0 },
];

export function SafetyNet() {
  const [files, setFiles] = useState<ProtectedFile[]>([]);
  const [stats, setStats] = useState<SafetyNetStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [, setTotal] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [actionFilter, setActionFilter] = useState<string>("all");
  const [agentFilter, setAgentFilter] = useState<string>("all");
  const [timePeriod, setTimePeriod] = useState("All");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [previewContent, setPreviewContent] = useState<string | null>(null);
  const [previewFile, setPreviewFile] = useState<ProtectedFile | null>(null);
  const [restoreConfirm, setRestoreConfirm] = useState<ProtectedFile | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [storageLimit, setStorageLimit] = useState(500);
  const [retentionDays, setRetentionDays] = useState(30);

  const loadData = useCallback(async () => {
    try {
      const result = await invoke<{ files: ProtectedFile[]; total: number }>("get_protected_files", {
        limit: 200,
        offset: 0,
        agentId: agentFilter !== "all" ? agentFilter : null,
        actionType: actionFilter !== "all" ? actionFilter : null,
        search: searchQuery || null,
      });
      setFiles(result.files);
      setTotal(result.total);

      const s = await invoke<SafetyNetStats>("get_safety_net_stats");
      setStats(s);
      if (s.storage_limit_bytes > 0) {
        setStorageLimit(Math.round(s.storage_limit_bytes / (1024 * 1024)));
      }
    } catch (e) {
      console.error("Failed to load safety net data:", e);
    } finally {
      setLoading(false);
    }
  }, [searchQuery, actionFilter, agentFilter]);

  useEffect(() => {
    loadData();
    const unlisten = listen<ProtectedFile>("file-protected", (event) => {
      setFiles((prev) => [event.payload, ...prev]);
      setTotal((prev) => prev + 1);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [loadData]);

  const showToast = (msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(null), 3000);
  };

  const handleRestore = async (file: ProtectedFile) => {
    try {
      const result = await invoke<RestoreResult>("restore_file", {
        id: file.id,
        snapshotPath: file.snapshot_path,
        originalPath: file.original_path,
      });
      if (result.success) {
        showToast(`Restored ${getFileName(file.original_path).name}`);
        setFiles((prev) => prev.map((f) => (f.id === file.id ? { ...f, restored: true } : f)));
      } else {
        showToast(`Failed: ${result.message}`);
      }
    } catch (e) {
      showToast(`Error: ${e}`);
    }
    setRestoreConfirm(null);
  };

  const handleRestoreMultiple = async () => {
    const selected = files.filter((f) => selectedIds.has(f.id));
    const payload = selected.map((f) => ({
      id: f.id,
      snapshot_path: f.snapshot_path,
      original_path: f.original_path,
    }));
    try {
      const results = await invoke<RestoreResult[]>("restore_multiple", { files: payload });
      const successes = results.filter((r) => r.success).length;
      showToast(`Restored ${successes} of ${results.length} files`);
      setSelectedIds(new Set());
      loadData();
    } catch (e) {
      showToast(`Error: ${e}`);
    }
  };

  const handleDelete = async (file: ProtectedFile) => {
    try {
      await invoke("delete_snapshot", { id: file.id, snapshotPath: file.snapshot_path });
      setFiles((prev) => prev.filter((f) => f.id !== file.id));
      setTotal((prev) => prev - 1);
    } catch (e) {
      console.error("Failed to delete snapshot:", e);
    }
  };

  const handleDeleteSelected = async () => {
    for (const file of files.filter((f) => selectedIds.has(f.id))) {
      await handleDelete(file);
    }
    setSelectedIds(new Set());
  };

  const handlePreview = async (file: ProtectedFile) => {
    try {
      const content = await invoke<string>("preview_file", { snapshotPath: file.snapshot_path });
      setPreviewContent(content);
      setPreviewFile(file);
    } catch (e) {
      console.error("Failed to preview:", e);
    }
  };

  const handleClearAll = async () => {
    if (!confirm("Delete ALL snapshots? This cannot be undone.")) return;
    try {
      const deleted = await invoke<number>("clear_old_snapshots", { beforeDate: "9999-12-31" });
      showToast(`Cleared ${deleted} snapshots`);
      loadData();
    } catch (e) {
      showToast(`Error: ${e}`);
    }
  };

  const handleSaveSettings = async () => {
    try {
      await invoke("update_safety_net_settings", {
        maxStorageBytes: storageLimit * 1024 * 1024,
        retentionDays,
      });
      showToast("Settings saved");
      loadData();
    } catch (e) {
      showToast(`Error: ${e}`);
    }
  };

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // Filter files by time period
  const filteredFiles = files.filter((f) => {
    const period = TIME_PERIODS.find((p) => p.label === timePeriod);
    if (!period || period.ms === 0) return true;
    return new Date(f.created_at).getTime() >= Date.now() - period.ms;
  });

  // Group files by date
  const grouped: Record<string, ProtectedFile[]> = {};
  for (const file of filteredFiles) {
    const group = getDateGroup(file.created_at);
    if (!grouped[group]) grouped[group] = [];
    grouped[group].push(file);
  }
  const groupOrder = ["Today", "Yesterday", "This Week", "Older"];

  const storagePercent = stats ? (stats.total_storage_bytes / stats.storage_limit_bytes) * 100 : 0;
  const storageColor = storagePercent > 80 ? "bg-red-500" : storagePercent > 60 ? "bg-yellow-500" : "bg-emerald-500";

  const agentNames = stats ? Object.keys(stats.by_agent) : [];

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="w-8 h-8 rounded-full border-2 border-cyan-500 border-t-transparent animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Toast */}
      {toast && (
        <div className="fixed top-4 right-4 z-50 glass-card px-4 py-3 text-sm text-white/90 animate-in fade-in slide-in-from-top-2">
          {toast}
        </div>
      )}

      {/* Hero Status */}
      <div className="glass-card p-5 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-xl bg-cyan-500/15 flex items-center justify-center">
            <ShieldCheck className="w-6 h-6 text-cyan-400 animate-pulse" />
          </div>
          <div>
            <h2 className="text-lg font-bold flex items-center gap-2">
              Your files are protected
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
            </h2>
            <p className="text-sm text-white/50">
              {stats
                ? `${stats.total_files} file${stats.total_files !== 1 ? "s" : ""} protected · ${formatBytes(stats.total_storage_bytes)} of ${formatBytes(stats.storage_limit_bytes)}`
                : "Safety Net is ready"}
            </p>
          </div>
        </div>

        {stats && stats.total_files > 0 && (
          <div className="w-48">
            <div className="flex justify-between text-xs text-white/40 mb-1">
              <span>Storage</span>
              <span>{storagePercent.toFixed(0)}%</span>
            </div>
            <div className="h-2 rounded-full bg-white/10 overflow-hidden">
              <div
                className={cn("h-full rounded-full transition-all", storageColor)}
                style={{ width: `${Math.min(storagePercent, 100)}%` }}
              />
            </div>
            {storagePercent > 80 && (
              <p className="text-xs text-red-400 mt-1 flex items-center gap-1">
                <AlertTriangle className="w-3 h-3" />
                Approaching storage limit
              </p>
            )}
          </div>
        )}
      </div>

      {/* Time Period Filter */}
      <div className="flex items-center gap-1 p-1 rounded-xl glass w-fit">
        {TIME_PERIODS.map((p) => (
          <button
            key={p.label}
            onClick={() => setTimePeriod(p.label)}
            className={cn(
              "px-4 py-1.5 rounded-lg text-xs font-medium transition-all",
              timePeriod === p.label
                ? "bg-white/15 text-white shadow-sm"
                : "text-white/40 hover:text-white/60"
            )}
          >
            {p.label}
          </button>
        ))}
      </div>

      {/* Toolbar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-white/30" />
          <input
            type="text"
            placeholder="Search files..."
            className="w-full pl-9 pr-4 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-white placeholder-white/30 outline-none focus:ring-1 focus:ring-cyan-500/50"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <div className="flex items-center rounded-lg bg-white/5 p-0.5">
          {(["all", "modified", "deleted"] as const).map((f) => (
            <button
              key={f}
              onClick={() => setActionFilter(f)}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium transition-all capitalize",
                actionFilter === f
                  ? "bg-white/10 text-white shadow-sm"
                  : "text-white/40 hover:text-white/60"
              )}
            >
              {f === "all" ? "All" : f}
            </button>
          ))}
        </div>

        {agentNames.length > 0 && (
          <select
            value={agentFilter}
            onChange={(e) => setAgentFilter(e.target.value)}
            className="px-3 py-2 rounded-lg glass text-xs text-white/70 outline-none bg-transparent"
          >
            <option value="all">All Agents</option>
            {agentNames.map((name) => (
              <option key={name} value={name}>{name}</option>
            ))}
          </select>
        )}
      </div>

      {/* Empty State */}
      {filteredFiles.length === 0 && files.length > 0 && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-yellow-500/15 flex items-center justify-center mx-auto mb-4">
            <AlertTriangle className="w-8 h-8 text-yellow-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">No files in this period</h3>
          <p className="text-white/50 max-w-md mx-auto">
            No protected files in the selected time range. Try a wider period.
          </p>
        </div>
      )}
      {files.length === 0 && (
        <div className="glass-card p-12 text-center">
          <div className="w-16 h-16 rounded-2xl bg-cyan-500/15 flex items-center justify-center mx-auto mb-4">
            <ShieldCheck className="w-8 h-8 text-cyan-400" />
          </div>
          <h3 className="text-xl font-bold mb-2">All quiet</h3>
          <p className="text-white/50 max-w-md mx-auto">
            Your Safety Net is ready. Files will be automatically backed up when agents modify them.
          </p>
        </div>
      )}

      {/* File List grouped by date */}
      {groupOrder.map((group) => {
        const items = grouped[group];
        if (!items || items.length === 0) return null;
        return (
          <div key={group} className="space-y-2">
            <h3 className="text-xs font-semibold text-white/40 uppercase tracking-wider px-1">
              {group} ({items.length})
            </h3>
            {items.map((file) => {
              const { name, dir } = getFileName(file.original_path);
              const Icon = getFileIcon(file.original_path);
              const isSelected = selectedIds.has(file.id);

              return (
                <div
                  key={file.id}
                  className={cn(
                    "glass-card p-4 flex items-center gap-3 group transition-all",
                    isSelected && "ring-1 ring-cyan-500/50"
                  )}
                >
                  {/* Checkbox */}
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => toggleSelect(file.id)}
                    className="w-4 h-4 rounded border-white/20 bg-white/5 accent-cyan-500 shrink-0"
                  />

                  {/* File icon */}
                  <div className="w-9 h-9 rounded-lg bg-white/5 flex items-center justify-center shrink-0">
                    <Icon className="w-4 h-4 text-white/50" />
                  </div>

                  {/* File info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-sm truncate">{name}</span>
                      {file.agent_name && (
                        <span className="px-1.5 py-0.5 rounded-full bg-white/5 text-[10px] text-white/40 shrink-0">
                          {file.agent_name}
                        </span>
                      )}
                      <span
                        className={cn(
                          "px-1.5 py-0.5 rounded-full text-[10px] font-medium shrink-0",
                          file.action_type === "deleted"
                            ? "bg-red-500/15 text-red-400"
                            : "bg-yellow-500/15 text-yellow-400"
                        )}
                      >
                        {file.action_type === "deleted" ? "Deleted" : "Modified"}
                      </span>
                      {file.restored && (
                        <span className="px-1.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 text-[10px] shrink-0">
                          Restored
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-xs text-white/30 mt-0.5">
                      <span className="truncate">{dir}</span>
                      <span>·</span>
                      <span>{timeAgo(file.created_at)}</span>
                      <span>·</span>
                      <span>{formatBytes(file.file_size)}</span>
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                    <button
                      onClick={() => handlePreview(file)}
                      className="p-2 rounded-lg glass glass-hover text-white/50 hover:text-white/80"
                      title="Preview"
                    >
                      <Eye className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => setRestoreConfirm(file)}
                      className="px-3 py-1.5 rounded-lg bg-cyan-500/15 text-cyan-400 hover:bg-cyan-500/25 text-xs font-medium transition-colors"
                    >
                      Restore
                    </button>
                    <button
                      onClick={() => handleDelete(file)}
                      className="p-2 rounded-lg glass glass-hover text-white/30 hover:text-red-400"
                      title="Delete snapshot"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        );
      })}

      {/* Batch action bar */}
      {selectedIds.size > 0 && (
        <div className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 glass-card px-5 py-3 flex items-center gap-4 shadow-2xl">
          <span className="text-sm text-white/70">{selectedIds.size} file{selectedIds.size !== 1 ? "s" : ""} selected</span>
          <button
            onClick={handleRestoreMultiple}
            className="px-4 py-2 rounded-lg bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/30 text-sm font-medium transition-colors flex items-center gap-2"
          >
            <RotateCcw className="w-4 h-4" />
            Restore All
          </button>
          <button
            onClick={handleDeleteSelected}
            className="px-4 py-2 rounded-lg bg-red-500/15 text-red-400 hover:bg-red-500/25 text-sm font-medium transition-colors flex items-center gap-2"
          >
            <Trash2 className="w-4 h-4" />
            Delete Snapshots
          </button>
          <button
            onClick={() => setSelectedIds(new Set())}
            className="p-1.5 rounded-lg glass glass-hover text-white/40"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {/* Settings (collapsible) */}
      <div className="mt-4">
        <button
          onClick={() => setShowSettings(!showSettings)}
          className="flex items-center gap-2 text-sm text-white/40 hover:text-white/60 transition-colors"
        >
          {showSettings ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          <Settings className="w-4 h-4" />
          Settings
        </button>

        {showSettings && (
          <div className="glass-card p-5 mt-3 space-y-4">
            <div>
              <label className="text-xs text-white/50 font-medium">
                Storage Limit: {storageLimit >= 1024 ? `${(storageLimit / 1024).toFixed(1)} GB` : `${storageLimit} MB`}
              </label>
              <input
                type="range"
                min={100}
                max={2048}
                step={100}
                value={storageLimit}
                onChange={(e) => setStorageLimit(Number(e.target.value))}
                className="w-full mt-2 accent-cyan-500"
              />
              <div className="flex justify-between text-[10px] text-white/30 mt-1">
                <span>100 MB</span>
                <span>2 GB</span>
              </div>
            </div>

            <div>
              <label className="text-xs text-white/50 font-medium">
                Retention: {retentionDays} days
              </label>
              <input
                type="range"
                min={7}
                max={90}
                step={1}
                value={retentionDays}
                onChange={(e) => setRetentionDays(Number(e.target.value))}
                className="w-full mt-2 accent-cyan-500"
              />
              <div className="flex justify-between text-[10px] text-white/30 mt-1">
                <span>7 days</span>
                <span>90 days</span>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <button
                onClick={handleSaveSettings}
                className="px-4 py-2 rounded-lg bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/30 text-sm font-medium transition-colors"
              >
                Save Settings
              </button>
              <button
                onClick={handleClearAll}
                className="px-4 py-2 rounded-lg bg-red-500/15 text-red-400 hover:bg-red-500/25 text-sm font-medium transition-colors"
              >
                Clear All Snapshots
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Preview Modal */}
      {previewContent !== null && previewFile && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={() => { setPreviewContent(null); setPreviewFile(null); }}>
          <div className="glass-card w-full max-w-2xl max-h-[80vh] flex flex-col m-4" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between p-4 border-b border-white/10">
              <div>
                <h3 className="font-medium text-sm">{getFileName(previewFile.original_path).name}</h3>
                <p className="text-xs text-white/40">{timeAgo(previewFile.created_at)}</p>
              </div>
              <button onClick={() => { setPreviewContent(null); setPreviewFile(null); }} className="p-2 rounded-lg glass glass-hover text-white/50">
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="overflow-auto flex-1 p-4">
              <pre className="text-xs text-white/70 font-mono whitespace-pre-wrap break-all">{previewContent}</pre>
            </div>
          </div>
        </div>
      )}

      {/* Restore Confirmation Modal */}
      {restoreConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={() => setRestoreConfirm(null)}>
          <div className="glass-card w-full max-w-md p-6 m-4" onClick={(e) => e.stopPropagation()}>
            <h3 className="font-bold text-lg mb-2">Restore file?</h3>
            <p className="text-sm text-white/60 mb-1">
              Restore <span className="text-white font-medium">{getFileName(restoreConfirm.original_path).name}</span> to
              state from {timeAgo(restoreConfirm.created_at)}?
            </p>
            <p className="text-xs text-white/40 mb-4">The current version will also be backed up before restoring.</p>
            <div className="flex items-center gap-3 justify-end">
              <button
                onClick={() => setRestoreConfirm(null)}
                className="px-4 py-2 rounded-lg glass glass-hover text-sm text-white/60"
              >
                Cancel
              </button>
              <button
                onClick={() => handleRestore(restoreConfirm)}
                className="px-4 py-2 rounded-lg bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/30 text-sm font-medium transition-colors"
              >
                Restore
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
