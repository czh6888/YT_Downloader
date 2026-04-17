import { useState, useEffect } from 'react';
import {
  AppShell,
  Group,
  Text,
  UnstyledButton,
  Stack,
  ActionIcon,
  useMantineColorScheme,
  rem,
  Tooltip,
  Badge,
} from '@mantine/core';
import {
  IconDownload,
  IconHistory,
  IconSettings,
  IconSun,
  IconMoon,
  IconBrandYoutube,
  IconFlask,
} from '@tabler/icons-react';
import { useAppStore } from './store/appStore';
import { DownloadPage } from './pages/Download';
import { HistoryPage } from './pages/History';
import { SettingsPage } from './pages/Settings';
import { TestRunnerPage } from './pages/TestRunner';
import * as api from './api/tauri';
import type { ProgressEvent } from './types';

const navItems = [
  { page: 'download' as const, icon: IconDownload, label: 'Download' },
  { page: 'history' as const, icon: IconHistory, label: 'History' },
  { page: 'settings' as const, icon: IconSettings, label: 'Settings' },
  { page: 'test' as const, icon: IconFlask, label: 'Test Runner' },
];

export default function App() {
  const { currentPage, setPage, theme, setTheme, tasks } = useAppStore();
  const { setColorScheme } = useMantineColorScheme();

  // Initialize: load config, history, download state
  useEffect(() => {
    async function init() {
      try {
        const cfg = await api.loadConfig();
        useAppStore.getState().setConfig(cfg);
        useAppStore.getState().setSaveDir(cfg.general.download_dir);
        useAppStore.getState().setAudioOnly(cfg.general.audio_only);
        useAppStore.getState().setAudioFormat(cfg.defaults.audio_format);
        useAppStore.getState().setSubtitlesEnabled(cfg.defaults.subtitles_enabled);
        useAppStore.getState().setSubtitleLangs(cfg.defaults.subtitle_langs);
        useAppStore.getState().setAskEachTime(cfg.defaults.ask_each_time);
      } catch (e) {
        console.error('Failed to load config:', e);
      }
      try {
        const hist = await api.getHistory('');
        useAppStore.getState().setHistory(hist);
      } catch (e) {
        console.error('Failed to load history:', e);
      }
      try {
        const state = await api.getDownloadState();
        for (const t of state) {
          useAppStore.getState().addTask(t);
        }
      } catch (e) {
        console.error('Failed to load download state:', e);
      }
    }
    init();
  }, []);

  // Theme sync
  useEffect(() => {
    setColorScheme(theme);
  }, [theme, setColorScheme]);

  // Listen for download progress events
  useEffect(() => {
    const unsub = api.onDownloadProgress(async (event: ProgressEvent) => {
      const { updateTask, addTask, tasks: currentTasks } = useAppStore.getState();
      let existing = currentTasks.find((t) => t.id === event.id);
      if (!existing) {
        // Task not in store yet — fetch it from backend
        try {
          const state = await api.getDownloadState();
          const newTask = state.find((t) => t.id === event.id);
          if (newTask) {
            addTask(newTask);
            existing = newTask;
          }
        } catch (e) {
          console.error('Failed to sync new task:', e);
          return;
        }
      }
      if (existing) {
        const speedStr = event.speed
          ? event.speed > 1024 * 1024
            ? `${(event.speed / (1024 * 1024)).toFixed(1)} MiB/s`
            : `${(event.speed / 1024).toFixed(1)} KiB/s`
          : '---';
        const etaStr = event.eta !== null && event.eta > 0
          ? `${Math.floor(event.eta / 60)}:${String(event.eta % 60).padStart(2, '0')}`
          : '--:--';
        updateTask(event.id, {
          progress: event.progress,
          downloaded_bytes: event.downloaded,
          total_bytes: event.total,
          speed: speedStr,
          eta: etaStr,
          speed_bytes: event.speed,
          eta_seconds: event.eta,
          speed_history: event.speed_history,
          status: 'Downloading',
        });
      }
    });
    return () => {
      unsub.then((fn: () => void) => fn());
    };
  }, []);

  // Listen for download completion
  useEffect(() => {
    const unsub = api.onDownloadComplete(async (taskId: number) => {
      useAppStore.getState().updateTask(taskId, {
        status: 'Done',
        progress: 1.0,
      });
      // Reload history
      try {
        const hist = await api.getHistory('');
        useAppStore.getState().setHistory(hist);
      } catch (e) {
        console.error('Failed to reload history:', e);
      }
    });
    return () => {
      unsub.then((fn: () => void) => fn());
    };
  }, []);

  // Listen for download errors
  useEffect(() => {
    const unsub = api.onDownloadError((msg: string) => {
      console.error('Download error:', msg);
    });
    return () => {
      unsub.then((fn: () => void) => fn());
    };
  }, []);

  // Active download count for badge
  const activeCount = tasks.filter(
    (t) => t.status === 'Downloading' || t.status === 'Queued' || t.status === 'Fetching'
  ).length;

  return (
    <AppShell
      header={{ height: 48 }}
      navbar={{ width: 64, breakpoint: 'sm', collapsed: { desktop: false } }}
      padding="md"
    >
      {/* Header / Title Bar */}
      <AppShell.Header>
        <Group h="100%" px="md" justify="space-between" style={{ WebkitAppRegion: 'drag' }}>
          <Group gap="xs" style={{ WebkitAppRegion: 'no-drag' }}>
            <IconBrandYoutube size={20} />
            <Text size="sm" fw={600}>YT Downloader</Text>
          </Group>
          <Group gap="xs" style={{ WebkitAppRegion: 'no-drag' }}>
            <Tooltip label={theme === 'dark' ? 'Light Mode' : 'Dark Mode'}>
              <ActionIcon
                variant="subtle"
                size="sm"
                onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
              >
                {theme === 'dark' ? <IconSun size={16} /> : <IconMoon size={16} />}
              </ActionIcon>
            </Tooltip>
          </Group>
        </Group>
      </AppShell.Header>

      {/* Sidebar */}
      <AppShell.Navbar p="xs">
        <Stack gap="xs">
          {navItems.map((item) => {
            const isActive = currentPage === item.page;
            return (
              <Tooltip key={item.page} label={item.label} position="right">
                <UnstyledButton
                  onClick={() => setPage(item.page)}
                  style={{
                    width: rem(40),
                    height: rem(40),
                    borderRadius: 6,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    background: isActive ? 'var(--mantine-primary-color-filled)' : 'transparent',
                    color: isActive ? 'var(--mantine-color-white)' : 'var(--mantine-color-text)',
                    transition: 'background 100ms',
                    position: 'relative',
                  }}
                >
                  <item.icon size={20} />
                  {item.page === 'download' && activeCount > 0 && (
                    <Badge
                      size="xs"
                      color="red"
                      style={{
                        position: 'absolute',
                        top: -4,
                        right: -4,
                        minWidth: 16,
                        height: 16,
                        padding: 0,
                        fontSize: 10,
                      }}
                    >
                      {activeCount}
                    </Badge>
                  )}
                </UnstyledButton>
              </Tooltip>
            );
          })}
        </Stack>
      </AppShell.Navbar>

      {/* Main Content */}
      <AppShell.Main>
        {currentPage === 'download' && <DownloadPage />}
        {currentPage === 'history' && <HistoryPage />}
        {currentPage === 'settings' && <SettingsPage />}
        {currentPage === 'test' && <TestRunnerPage />}
      </AppShell.Main>
    </AppShell>
  );
}
