import { create } from 'zustand';
import type {
  DownloadTask,
  FormatInfo,
  HistoryEntry,
  Config,
  VideoInfo,
  Page,
  Browser,
} from '../types';

interface AppState {
  // Navigation
  currentPage: Page;
  setPage: (page: Page) => void;

  // Theme
  theme: 'light' | 'dark';
  setTheme: (t: 'light' | 'dark') => void;

  // Language
  language: 'en' | 'zh';
  setLanguage: (l: 'en' | 'zh') => void;

  // Download page
  url: string;
  setUrl: (u: string) => void;
  browser: Browser;
  setBrowser: (b: Browser) => void;
  videoInfo: VideoInfo | null;
  setVideoInfo: (info: VideoInfo | null) => void;
  formats: FormatInfo[];
  setFormats: (fmts: FormatInfo[]) => void;
  selectedFormats: string[];
  setSelectedFormats: (ids: string[]) => void;
  toggleFormat: (id: string) => void;
  fetchingInfo: boolean;
  setFetchingInfo: (v: boolean) => void;
  saveDir: string;
  setSaveDir: (d: string) => void;
  audioOnly: boolean;
  setAudioOnly: (v: boolean) => void;
  audioFormat: string;
  setAudioFormat: (f: string) => void;
  subtitlesEnabled: boolean;
  setSubtitlesEnabled: (v: boolean) => void;
  subtitleLangs: string;
  setSubtitleLangs: (l: string) => void;
  askEachTime: boolean;
  setAskEachTime: (v: boolean) => void;

  // Queue
  tasks: DownloadTask[];
  addTask: (t: DownloadTask) => void;
  updateTask: (id: number, updates: Partial<DownloadTask>) => void;
  removeTask: (id: number) => void;
  cancelTask: (id: number) => void;

  // History
  history: HistoryEntry[];
  setHistory: (h: HistoryEntry[]) => void;
  deleteHistoryEntry: (id: number) => void;
  historySearch: string;
  setHistorySearch: (q: string) => void;

  // Config
  config: Config | null;
  setConfig: (c: Config | null) => void;

  // Format dialog
  formatDialogOpen: boolean;
  setFormatDialogOpen: (v: boolean) => void;
  formatFilter: 'video' | 'audio' | 'combined' | 'all';
  setFormatFilter: (f: 'video' | 'audio' | 'combined' | 'all') => void;
  formatSearch: string;
  setFormatSearch: (s: string) => void;

  // Status
  statusText: string;
  setStatusText: (s: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentPage: 'download',
  setPage: (page) => set({ currentPage: page }),

  theme: 'dark',
  setTheme: (theme) => set({ theme }),

  language: 'en',
  setLanguage: (language) => set({ language }),

  url: '',
  setUrl: (url) => set({ url }),
  browser: 'Chrome',
  setBrowser: (browser) => set({ browser }),
  videoInfo: null,
  setVideoInfo: (videoInfo) => set({ videoInfo }),
  formats: [],
  setFormats: (formats) => set({ formats }),
  selectedFormats: [],
  setSelectedFormats: (selectedFormats) => set({ selectedFormats }),
  toggleFormat: (id) =>
    set((state) => ({
      selectedFormats: state.selectedFormats.includes(id)
        ? []
        : [id],
    })),
  fetchingInfo: false,
  setFetchingInfo: (fetchingInfo) => set({ fetchingInfo }),
  saveDir: '',
  setSaveDir: (saveDir) => set({ saveDir }),
  audioOnly: false,
  setAudioOnly: (audioOnly) => set({ audioOnly }),
  audioFormat: 'm4a',
  setAudioFormat: (audioFormat) => set({ audioFormat }),
  subtitlesEnabled: false,
  setSubtitlesEnabled: (subtitlesEnabled) => set({ subtitlesEnabled }),
  subtitleLangs: 'zh-Hans,en',
  setSubtitleLangs: (subtitleLangs) => set({ subtitleLangs }),
  askEachTime: true,
  setAskEachTime: (askEachTime) => set({ askEachTime }),

  tasks: [],
  addTask: (task) =>
    set((state) => ({ tasks: [...state.tasks, task] })),
  updateTask: (id, updates) =>
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === id ? { ...t, ...updates } : t
      ),
    })),
  removeTask: (id) =>
    set((state) => ({ tasks: state.tasks.filter((t) => t.id !== id) })),
  cancelTask: (id) =>
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === id ? { ...t, status: 'Cancelled' as const } : t
      ),
    })),

  history: [],
  setHistory: (history) => set({ history }),
  deleteHistoryEntry: (id) =>
    set((state) => ({
      history: state.history.filter((h) => h.id !== id),
    })),
  historySearch: '',
  setHistorySearch: (historySearch) => set({ historySearch }),

  config: null,
  setConfig: (config) => set({ config }),

  formatDialogOpen: false,
  setFormatDialogOpen: (formatDialogOpen) => set({ formatDialogOpen }),
  formatFilter: 'video',
  setFormatFilter: (formatFilter) => set({ formatFilter }),
  formatSearch: '',
  setFormatSearch: (formatSearch) => set({ formatSearch }),

  statusText: 'Ready',
  setStatusText: (statusText) => set({ statusText }),
}));
