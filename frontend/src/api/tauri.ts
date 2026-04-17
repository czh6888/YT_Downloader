import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  Config,
  DownloadTask,
  DownloadParams,
  FormatInfo,
  HistoryEntry,
  ProgressEvent,
} from '../types';

// === Commands ===

export async function checkYtDlp(): Promise<string[] | null> {
  return invoke<string[] | null>('check_yt_dlp');
}

export async function fetchInfo(
  url: string,
  cookieArgs: string[]
): Promise<{
  info: { title: string; thumbnail: string; uploader: string; duration: number | null; description: string };
  formats: FormatInfo[];
}> {
  // Tauri v2 uses camelCase for parameter deserialization
  return invoke('fetch_info', { url, cookieArgs });
}

export async function startDownload(
  params: DownloadParams
): Promise<number> {
  return invoke<number>('start_download', { params });
}

export async function pauseDownload(taskId: number): Promise<void> {
  return invoke('pause_download', { taskId });
}

export async function resumeDownload(taskId: number): Promise<void> {
  return invoke('resume_download', { taskId });
}

export async function cancelDownload(taskId: number): Promise<void> {
  return invoke('cancel_download', { taskId });
}

export async function getDownloadState(): Promise<DownloadTask[]> {
  return invoke('get_download_state');
}

export async function loadConfig(): Promise<Config> {
  return invoke('load_config');
}

export async function saveConfig(config: Config): Promise<void> {
  return invoke('save_config', { config });
}

export async function getHistory(query: string): Promise<HistoryEntry[]> {
  return invoke('get_history', { query });
}

export async function deleteHistoryEntry(id: number): Promise<void> {
  return invoke('delete_history', { id });
}

export async function clearHistory(): Promise<void> {
  return invoke('clear_history');
}

export async function extractCookies(browser: string): Promise<{ file: string | null; fallback: string | null; success: boolean }> {
  return invoke('extract_cookies', { browser });
}

export async function detectFfmpeg(): Promise<boolean> {
  return invoke('detect_ffmpeg');
}

export async function openFolder(path: string): Promise<void> {
  return invoke('open_folder', { path });
}

export async function showInExplorer(filePath: string): Promise<void> {
  return invoke('show_in_explorer', { filePath });
}

export async function deleteFile(filePath: string): Promise<void> {
  return invoke('delete_file', { filePath });
}

export async function getClipboard(): Promise<string> {
  return invoke('get_clipboard');
}

// === Event listeners ===

export function onDownloadProgress(
  callback: (event: ProgressEvent) => void
): Promise<() => void> {
  return listen('download-progress', ({ payload }) => {
    callback(payload as ProgressEvent);
  });
}

export function onDownloadComplete(
  callback: (taskId: number) => void
): Promise<() => void> {
  return listen('download-complete', ({ payload }) => {
    callback(payload as number);
  });
}

export function onDownloadError(
  callback: (msg: string) => void
): Promise<() => void> {
  return listen('download-error', ({ payload }) => {
    callback(payload as string);
  });
}

export function onClipboardText(
  callback: (text: string) => void
): Promise<() => void> {
  return listen('clipboard-text', ({ payload }) => {
    callback(payload as string);
  });
}
