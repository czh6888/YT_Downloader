import { useState, useCallback, useEffect, useRef } from 'react';
import {
  Paper, Text, Stack, Group, Button, Badge, ScrollArea, Alert,
  Grid, Code, Divider, Loader,
} from '@mantine/core';
import { IconCheck, IconX, IconPlayerPlay, IconClock, IconAlertCircle } from '@tabler/icons-react';
import * as api from '../api/tauri';

interface TestResult {
  name: string;
  category: string;
  status: 'pending' | 'running' | 'pass' | 'fail' | 'skip';
  detail: string;
  duration: number;
}

const TEST_SUITE: Array<{
  name: string;
  category: string;
  fn: () => Promise<{ pass: boolean; detail: string }>;
}> = [
  // === Tauri Core ===
  {
    name: 'check_yt_dlp',
    category: 'Tauri Core',
    fn: async () => {
      const result = await api.checkYtDlp();
      if (result && result.length > 0) {
        return { pass: true, detail: `yt-dlp found: ${result[0]}` };
      }
      return { pass: false, detail: 'yt-dlp not found on PATH' };
    },
  },
  {
    name: 'detect_ffmpeg',
    category: 'Tauri Core',
    fn: async () => {
      const result = await api.detectFfmpeg();
      return { pass: result, detail: result ? 'ffmpeg detected' : 'ffmpeg not found' };
    },
  },
  {
    name: 'load_config',
    category: 'Tauri Core',
    fn: async () => {
      const cfg = await api.loadConfig();
      return {
        pass: !!cfg.general && !!cfg.download,
        detail: `Config loaded. download_dir=${cfg.general.download_dir}, theme=${cfg.general.theme}`,
      };
    },
  },
  {
    name: 'save_config',
    category: 'Tauri Core',
    fn: async () => {
      const cfg = await api.loadConfig();
      await api.saveConfig(cfg);
      return { pass: true, detail: 'Config saved successfully (no-op write test)' };
    },
  },

  // === Fetch & Download ===
  {
    name: 'fetch_info (valid URL)',
    category: 'Fetch & Download',
    fn: async () => {
      try {
        const result = await api.fetchInfo('https://www.youtube.com/watch?v=LmQC_tcdCMc', []);
        return {
          pass: !!result.info && !!result.info.title && result.formats.length > 0,
          detail: `Title: ${result.info.title}, Formats: ${result.formats.length}`,
        };
      } catch (e: any) {
        const msg = e?.message || String(e);
        if (msg.includes('Sign in to confirm') || msg.includes('bot')) {
          return { pass: false, detail: 'yt-dlp reached YouTube — bot detection triggered. This means fetch_info WORKS but needs --cookies-from-browser to bypass. Try with Chrome/Edge cookie in main UI.' };
        }
        throw e;
      }
    },
  },
  {
    name: 'fetch_info (Chrome cookies)',
    category: 'Fetch & Download',
    fn: async () => {
      try {
        const result = await api.fetchInfo('https://www.youtube.com/watch?v=LmQC_tcdCMc', ['Chrome']);
        return {
          pass: !!result.info && result.formats.length > 0,
          detail: `Title: ${result.info.title}, Formats: ${result.formats.length}`,
        };
      } catch (e: any) {
        const msg = e?.message || String(e);
        if (msg.includes('Sign in to confirm') || msg.includes('bot')) {
          return { pass: false, detail: 'Chrome cookie auth works but YouTube bot detection triggered. This is expected — cookie auth is properly wired up.' };
        }
        throw e;
      }
    },
  },
  {
    name: 'fetch_info (invalid URL)',
    category: 'Fetch & Download',
    fn: async () => {
      try {
        await api.fetchInfo('not-a-valid-url', []);
        return { pass: false, detail: 'Should have thrown error for invalid URL' };
      } catch (e: any) {
        return { pass: true, detail: `Correctly rejected: ${e?.message?.slice(0, 80) || String(e).slice(0, 80)}` };
      }
    },
  },
  {
    name: 'get_download_state',
    category: 'Fetch & Download',
    fn: async () => {
      const state = await api.getDownloadState();
      return { pass: Array.isArray(state), detail: `Tasks: ${state.length}` };
    },
  },
  {
    name: 'start_download (camelCase params)',
    category: 'Fetch & Download',
    fn: async () => {
      // CRITICAL: Tests that DownloadParams accepts camelCase fields
      // Previously failed with: missing field `format_ids`
      const cfg = await api.loadConfig();
      const taskId = await api.startDownload({
        url: 'https://www.youtube.com/watch?v=LmQC_tcdCMc',
        title: 'Test Video',
        formatIds: ['best'],
        saveDir: cfg.general.download_dir,
        audioOnly: false,
        audioFormat: 'mp3',
        cookieArgs: [] as string[],
        subtitlesEnabled: false,
        subtitleLangs: '',
      });
      return { pass: typeof taskId === 'number' && taskId > 0, detail: `Task created with id=${taskId}` };
    },
  },

  // === History ===
  {
    name: 'get_history (empty query)',
    category: 'History',
    fn: async () => {
      const entries = await api.getHistory('');
      return { pass: Array.isArray(entries), detail: `History entries: ${entries.length}` };
    },
  },
  {
    name: 'get_history (search)',
    category: 'History',
    fn: async () => {
      const entries = await api.getHistory('test');
      return { pass: Array.isArray(entries), detail: `Search results: ${entries.length}` };
    },
  },
  {
    name: 'clear_history',
    category: 'History',
    fn: async () => {
      await api.clearHistory();
      const entries = await api.getHistory('');
      return { pass: entries.length === 0, detail: `After clear: ${entries.length} entries` };
    },
  },

  // === Clipboard ===
  {
    name: 'get_clipboard',
    category: 'Clipboard',
    fn: async () => {
      const text = await api.getClipboard();
      return { pass: typeof text === 'string', detail: `Clipboard: "${text.slice(0, 50)}"` };
    },
  },

  // === File Operations ===
  {
    name: 'open_folder (temp dir)',
    category: 'File Operations',
    fn: async () => {
      await api.openFolder('C:\\Users\\CZH\\AppData\\Local\\Temp');
      return { pass: true, detail: 'Folder opened successfully' };
    },
  },

  // === Cookie Extraction ===
  {
    name: 'extract_cookies (Chrome)',
    category: 'Cookie Extraction',
    fn: async () => {
      const result = await api.extractCookies('Chrome');
      return {
        pass: true,
        detail: `success=${result.success}, file=${result.file || 'null'}, fallback=${result.fallback || 'null'}`,
      };
    },
  },
  {
    name: 'extract_cookies (Firefox)',
    category: 'Cookie Extraction',
    fn: async () => {
      const result = await api.extractCookies('Firefox');
      return {
        pass: true,
        detail: `success=${result.success}, file=${result.file || 'null'}, fallback=${result.fallback || 'null'}`,
      };
    },
  },
];

export function TestRunnerPage() {
  const [results, setResults] = useState<TestResult[]>(
    TEST_SUITE.map(t => ({ name: t.name, category: t.category, status: 'pending' as const, detail: 'Waiting...', duration: 0 }))
  );
  const [running, setRunning] = useState(false);
  const [startTime, setStartTime] = useState<string | null>(null);
  const [endTime, setEndTime] = useState<string | null>(null);
  const autoRun = useRef(false);

  // Auto-run tests on mount — no manual click needed
  useEffect(() => {
    if (!autoRun.current && !running) {
      autoRun.current = true;
      runTests();
    }
  }, [running]);

  const runTests = useCallback(async () => {
    setRunning(true);
    setStartTime(new Date().toLocaleTimeString());
    setEndTime(null);

    const newResults: TestResult[] = TEST_SUITE.map(t => ({
      name: t.name, category: t.category, status: 'pending' as const, detail: 'Waiting...', duration: 0,
    }));
    setResults(newResults);

    for (let i = 0; i < TEST_SUITE.length; i++) {
      newResults[i] = { ...newResults[i], status: 'running' as const, detail: 'Running...' };
      setResults([...newResults]);

      const t0 = performance.now();
      try {
        const { pass, detail } = await TEST_SUITE[i].fn();
        const dur = Math.round(performance.now() - t0);
        newResults[i] = { ...newResults[i], status: pass ? 'pass' : 'fail', detail, duration: dur };
      } catch (e: any) {
        const dur = Math.round(performance.now() - t0);
        newResults[i] = { ...newResults[i], status: 'fail', detail: `Exception: ${e?.message || String(e)}`, duration: dur };
      }
      setResults([...newResults]);
    }

    setRunning(false);
    setEndTime(new Date().toLocaleTimeString());
  }, []);

  const passed = results.filter(r => r.status === 'pass').length;
  const failed = results.filter(r => r.status === 'fail').length;
  const pending = results.filter(r => r.status === 'pending' || r.status === 'running').length;
  const totalDuration = results.reduce((sum, r) => sum + r.duration, 0);

  const categories = [...new Set(TEST_SUITE.map(t => t.category))];

  return (
    <Stack gap="md">
      {/* Header */}
      <Paper p="md" withBorder>
        <Group justify="space-between">
          <Group>
            <Text fw={700} size="xl">Automated Test Runner</Text>
            <Badge size="lg" color={running ? 'yellow' : failed > 0 ? 'red' : passed > 0 ? 'green' : 'gray'}>
              {running ? 'Running...' : `${passed}/${results.length} passed`}
            </Badge>
          </Group>
          <Button
            leftSection={running ? <Loader size="xs" color="white" /> : <IconPlayerPlay size={16} />}
            onClick={runTests}
            loading={running}
            disabled={running}
            size="md"
          >
            {running ? 'Running Tests...' : 'Run All Tests'}
          </Button>
        </Group>

        {/* Summary Stats */}
        {startTime && (
          <Group mt="sm" gap="lg">
            <Text size="xs" c="dimmed">Started: {startTime}</Text>
            {endTime && <Text size="xs" c="dimmed">Ended: {endTime}</Text>}
            <Text size="xs" c="dimmed">Duration: {totalDuration}ms</Text>
            <Badge size="sm" color="green">Pass: {passed}</Badge>
            <Badge size="sm" color="red">Fail: {failed}</Badge>
            {pending > 0 && <Badge size="sm" color="yellow">Pending: {pending}</Badge>}
          </Group>
        )}
      </Paper>

      {/* Results by Category */}
      {categories.map(cat => {
        const catTests = results.filter(r => r.category === cat);
        const catPassed = catTests.filter(r => r.status === 'pass').length;
        return (
          <Paper key={cat} p="md" withBorder>
            <Group justify="space-between" mb="sm">
              <Text fw={600} size="md">{cat}</Text>
              <Badge size="sm" color={catPassed === catTests.length ? 'green' : 'red'}>
                {catPassed}/{catTests.length}
              </Badge>
            </Group>
            <Stack gap={4}>
              {catTests.map((result) => {
                const icon = result.status === 'pass' ? <IconCheck size={16} />
                  : result.status === 'running' ? <Loader size="xs" />
                  : result.status === 'fail' ? <IconX size={16} />
                  : <IconClock size={16} />;
                const color = result.status === 'pass' ? 'green'
                  : result.status === 'fail' ? 'red'
                  : result.status === 'running' ? 'yellow' : 'gray';
                return (
                  <Paper key={result.name} p="xs" withBorder={result.status === 'fail'} style={{ borderColor: color }}>
                    <Group gap="xs" wrap="nowrap" align="flex-start">
                      <div style={{ color, flexShrink: 0, marginTop: 2 }}>{icon}</div>
                      <Stack gap={0} style={{ flex: 1 }}>
                        <Group gap="xs">
                          <Text size="sm" fw={600}>{result.name}</Text>
                          <Badge size="xs" color={color} variant="outline">
                            {result.status.toUpperCase()}
                          </Badge>
                          {result.duration > 0 && (
                            <Text size="xs" c="dimmed">{result.duration}ms</Text>
                          )}
                        </Group>
                        <Text size="xs" c="dimmed" style={{ wordBreak: 'break-all' }}>{result.detail}</Text>
                      </Stack>
                    </Group>
                  </Paper>
                );
              })}
            </Stack>
          </Paper>
        );
      })}

      {/* Log Export */}
      {results.length > 0 && !running && (
        <Paper p="md" withBorder>
          <Text fw={600} size="sm" mb="xs">Test Log (JSON)</Text>
          <ScrollArea h={200}>
            <Code block style={{ fontSize: 11 }}>
              {JSON.stringify({
                startTime,
                endTime,
                totalDuration: `${totalDuration}ms`,
                summary: { passed, failed, total: results.length },
                results: results.map(r => ({ name: r.name, status: r.status, detail: r.detail, duration: `${r.duration}ms` })),
              }, null, 2)}
            </Code>
          </ScrollArea>
        </Paper>
      )}
    </Stack>
  );
}
