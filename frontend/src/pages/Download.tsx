import {
  Paper,
  TextInput,
  Select,
  Button,
  Group,
  Stack,
  Text,
  Checkbox,
  Card,
  Image,
  Badge,
  Modal,
  Tabs,
  ScrollArea,
  ActionIcon,
  SegmentedControl,
  Box,
  Progress,
  Tooltip,
  Alert,
  Divider,
  Loader,
  Center,
} from '@mantine/core';
import {
  IconDownload,
  IconTrash,
  IconSearch,
  IconPlayerStop,
  IconAlertCircle,
  IconCircle,
  IconCircleFilled,
  IconRefresh,
} from '@tabler/icons-react';
import { useState } from 'react';
import { useAppStore } from '../store/appStore';
import * as api from '../api/tauri';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
} from 'recharts';
import type { FormatInfo, DownloadTask } from '../types';

const browserOptions = [
  { value: 'Chrome', label: 'Chrome' },
  { value: 'Edge', label: 'Edge' },
  { value: 'Firefox', label: 'Firefox' },
  { value: 'NoCookies', label: 'No Cookies' },
];

const audioFormats = ['m4a', 'mp3', 'flac', 'opus'];

const qualityOptions = [
  { value: 'best', label: 'Best Available' },
  { value: '2160', label: '2160p (4K)' },
  { value: '1440', label: '1440p (2K)' },
  { value: '1080', label: '1080p' },
  { value: '720', label: '720p' },
  { value: '480', label: '480p' },
  { value: '360', label: '360p' },
];

export function DownloadPage() {
  const {
    url, setUrl, browser, setBrowser, videoInfo, formats,
    saveDir,
    audioOnly, setAudioOnly, audioFormat, setAudioFormat,
    subtitlesEnabled, setSubtitlesEnabled, subtitleLangs, setSubtitleLangs,
    selectedFormats, setSelectedFormats, toggleFormat,
    statusText, setStatusText,
    formatDialogOpen, setFormatDialogOpen, formatFilter, setFormatFilter,
    formatSearch, setFormatSearch,
    tasks, removeTask, cancelTask, config,
  } = useAppStore();

  const [isDownloading, setIsDownloading] = useState(false);
  const [quality, setQuality] = useState('best');
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [fetchingInfo, setFetchingInfo] = useState(false);

  // Stacher7 flow: click Download → immediately open dialog → fetch info in background
  const handleDownloadClick = async () => {
    setFetchError(null);
    const trimmedUrl = url.trim();
    if (!trimmedUrl) {
      setFetchError('URL is empty! Please paste a valid URL first.');
      return;
    }

    // Immediately open format dialog (Stacher7 style)
    setFormatDialogOpen(true);
    setFetchingInfo(true);
    setStatusText('Fetching video info...');

    try {
      const cookieArgs = browser === 'NoCookies' ? [] : [browser];
      const result = await api.fetchInfo(trimmedUrl, cookieArgs);
      useAppStore.getState().setVideoInfo(result.info);
      useAppStore.getState().setFormats(result.formats);

      // Auto-select: SINGLE best COMBINED format (has both video+audio)
      const combined = result.formats.filter((f) => f.is_combined);
      if (combined.length > 0) {
        combined.sort((a, b) => (b.height || 0) - (a.height || 0));
        setSelectedFormats([combined[0].format_id]);
      } else {
        const videoFormats = result.formats.filter((f) => f.is_video && !f.is_combined);
        if (videoFormats.length > 0) {
          videoFormats.sort((a, b) => (b.height || 0) - (a.height || 0));
          setSelectedFormats([videoFormats[0].format_id]);
        } else {
          setSelectedFormats(['best']);
        }
      }
      setStatusText(`Found ${result.formats.length} formats`);
    } catch (e: any) {
      const errorMsg = e?.message || e?.toString() || String(e);
      setFetchError(`Fetch failed: ${errorMsg}`);
      setStatusText(`Error: ${errorMsg}`);
      console.error('Fetch error:', e);
    } finally {
      setFetchingInfo(false);
    }
  };

  // Confirm download from modal — start download + sync task to store
  const handleConfirmDownload = async () => {
    if (!url.trim() || selectedFormats.length === 0) {
      console.warn('Cannot download: missing URL or format');
      return;
    }
    setFormatDialogOpen(false);
    setIsDownloading(true);
    try {
      const taskId = await api.startDownload({
        url: url.trim(),
        title: videoInfo?.title || '',
        formatIds: selectedFormats,
        saveDir: saveDir || config?.general.download_dir || '',
        audioOnly: audioOnly,
        audioFormat: audioFormat,
        cookieArgs: browser === 'NoCookies' ? [] : [browser],
        subtitlesEnabled: subtitlesEnabled,
        subtitleLangs: subtitleLangs,
      });
      console.log('Download started, task ID:', taskId);
      setStatusText('Download started!');

      // Sync the new task from backend into the store
      try {
        const state = await api.getDownloadState();
        const existing = useAppStore.getState().tasks.find((t) => t.id === taskId);
        if (!existing) {
          const newTask = state.find((t) => t.id === taskId);
          if (newTask) {
            useAppStore.getState().addTask(newTask);
            console.log('Task synced to store:', taskId);
          }
        }
      } catch (syncErr) {
        console.error('Failed to sync download task:', syncErr);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setFetchError(`Download failed: ${msg}`);
      setStatusText(`Download failed: ${msg}`);
      console.error('Download error:', e);
    } finally {
      setIsDownloading(false);
    }
  };

  // Format dialog filtering
  const filteredFormats = formats.filter((fmt) => {
    if (formatFilter === 'video') return fmt.is_video && !fmt.is_combined;
    if (formatFilter === 'audio') return fmt.is_audio;
    if (formatFilter === 'combined') return fmt.is_combined;
    return true;
  }).filter((fmt) => {
    if (!formatSearch) return true;
    const lower = formatSearch.toLowerCase();
    return (
      fmt.resolution.toLowerCase().includes(lower) ||
      fmt.ext.toLowerCase().includes(lower) ||
      fmt.format_id.toLowerCase().includes(lower) ||
      fmt.note.toLowerCase().includes(lower) ||
      fmt.vcodec.toLowerCase().includes(lower) ||
      fmt.acodec.toLowerCase().includes(lower)
    );
  });

  const activeTasks = tasks.filter(
    (t) => t.status === 'Downloading' || t.status === 'Queued' || t.status === 'Fetching'
  );
  const doneTasks = tasks.filter(
    (t) => t.status === 'Done' || t.status === 'Cancelled' || (typeof t.status === 'object' && 'Failed' in t.status)
  );

  return (
    <Stack gap="md">
      {/* URL Input + Download button */}
      <Paper p="md" withBorder>
        <Stack gap="sm">
          <Group gap="xs">
            <TextInput
              placeholder="Paste URL here (YouTube, Bilibili, etc.)"
              value={url}
              onChange={(e) => setUrl(e.currentTarget.value)}
              size="md"
              style={{ flex: 1 }}
              leftSection={<IconDownload size={16} />}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleDownloadClick();
              }}
            />
            <Select
              value={browser}
              onChange={(v) => v && setBrowser(v as any)}
              data={browserOptions}
              w={150}
              size="md"
            />
            <Button
              onClick={handleDownloadClick}
              loading={fetchingInfo}
              disabled={fetchingInfo && formatDialogOpen}
              size="md"
              leftSection={fetchingInfo && !formatDialogOpen ? <Loader size={16} color="white" /> : <IconDownload size={16} />}
            >
              {fetchingInfo && !formatDialogOpen ? 'Fetching...' : 'Download'}
            </Button>
          </Group>
          <Text c="dimmed" size="xs">{statusText}</Text>
        </Stack>
      </Paper>

      {/* Error alert */}
      {fetchError && (
        <Alert variant="light" color="red" title="Error" icon={<IconAlertCircle />} onClose={() => setFetchError(null)}>
          <Text size="sm">{fetchError}</Text>
        </Alert>
      )}

      {/* Active Downloads — INLINE below URL input */}
      {activeTasks.length > 0 && (
        <Paper p="md" withBorder>
          <Text fw={600} size="sm" mb="xs">Active Downloads ({activeTasks.length})</Text>
          <Stack gap="sm">
            {activeTasks.map((task) => (
          <TaskCard
            key={task.id}
            task={task}
            showSpeedChart={config?.general.show_speed_chart ?? true}
          />
        ))}
          </Stack>
        </Paper>
      )}

      {/* Completed Downloads */}
      {doneTasks.length > 0 && (
        <Paper p="md" withBorder>
          <Text fw={600} size="sm" mb="xs">Completed ({doneTasks.length})</Text>
          <ScrollArea h={200}>
            <Stack gap="xs">
              {doneTasks.map((task) => (
                <Group key={task.id} gap="xs" wrap="nowrap">
                  <Badge
                    size="xs"
                    color={task.status === 'Done' ? 'green' : task.status === 'Cancelled' ? 'gray' : 'red'}
                  >
                    {typeof task.status === 'string' ? task.status : 'Failed'}
                  </Badge>
                  <Text size="xs" lineClamp={1} style={{ flex: 1 }}>
                    {task.title || task.url}
                  </Text>
                  <ActionIcon
                    variant="subtle"
                    size="xs"
                    color="gray"
                    onClick={() => removeTask(task.id)}
                  >
                    <IconTrash size={12} />
                  </ActionIcon>
                </Group>
              ))}
            </Stack>
          </ScrollArea>
        </Paper>
      )}

      {/* Empty state */}
      {activeTasks.length === 0 && doneTasks.length === 0 && (
        <Paper p="xl" withBorder>
          <Text c="dimmed" size="sm" ta="center">No downloads yet. Paste a URL and click Download.</Text>
        </Paper>
      )}

      {/* Format Selection Modal — Stacher7 style: opens immediately, fetches in background */}
      <Modal
        opened={formatDialogOpen}
        onClose={() => setFormatDialogOpen(false)}
        title={videoInfo ? videoInfo.title : 'Select Formats'}
        size="xl"
        closeOnClickOutside={false}
      >
        {/* Loading state: fetching info in background */}
        {fetchingInfo && !videoInfo && (
          <Center py="xl">
            <Stack gap="md" align="center">
              <Loader size="lg" />
              <Text c="dimmed" size="sm">Fetching video info...</Text>
              <Text c="dimmed" size="xs">This may take a few seconds</Text>
            </Stack>
          </Center>
        )}

        {/* Error state: fetch failed */}
        {fetchError && !fetchingInfo && !videoInfo && (
          <Stack gap="md" py="md">
            <Alert variant="light" color="red" title="Failed to fetch video info" icon={<IconAlertCircle />}>
              <Text size="sm">{fetchError}</Text>
            </Alert>
            <Button
              variant="outline"
              size="sm"
              leftSection={<IconRefresh size={16} />}
              onClick={handleDownloadClick}
            >
              Retry
            </Button>
          </Stack>
        )}

        {/* Success state: show format selection */}
        {videoInfo && (
          <>
            <Group gap="md" mb="md">
              {videoInfo.thumbnail ? (
                <Image src={videoInfo.thumbnail} w={160} h={90} radius="md" fit="cover" />
              ) : null}
              <Stack gap={4} style={{ flex: 1 }}>
                <Text fw={600} size="sm" lineClamp={2}>{videoInfo.title}</Text>
                <Text c="dimmed" size="xs">{videoInfo.uploader}</Text>
                {videoInfo.duration && (
                  <Badge variant="outline" size="xs">
                    {Math.floor(videoInfo.duration / 60)}:{String(Math.floor(videoInfo.duration % 60)).padStart(2, '0')}
                  </Badge>
                )}
              </Stack>
            </Group>

            <Divider my="sm" />

            {/* Quality Selector */}
            <Group gap="md" mb="sm">
              <Select
                label="Quality"
                value={quality}
                onChange={(v) => {
                  setQuality(v || 'best');
                  const q = v || 'best';
                  if (q === 'best') {
                    const combined = formats.filter((f) => f.is_combined);
                    if (combined.length > 0) {
                      combined.sort((a, b) => (b.height || 0) - (a.height || 0));
                      setSelectedFormats([combined[0].format_id]);
                    } else {
                      setSelectedFormats(['best']);
                    }
                  } else {
                    const maxH = parseInt(q);
                    const matching = formats.filter((f) => f.is_video && !f.is_combined && (f.height || 0) <= maxH);
                    if (matching.length > 0) {
                      matching.sort((a, b) => (b.height || 0) - (a.height || 0));
                      setSelectedFormats([matching[0].format_id]);
                    } else {
                      setSelectedFormats(['best']);
                    }
                  }
                }}
                data={qualityOptions}
                w={200}
                size="xs"
                allowDeselect={false}
              />
              <Badge variant="outline" size="sm">
                {selectedFormats.length > 0 ? '1 selected' : 'None selected'}
              </Badge>
            </Group>

            <Tabs value={formatFilter} onChange={(v) => setFormatFilter(v as any)}>
              <Tabs.List>
                <Tabs.Tab value="combined">Combined</Tabs.Tab>
                <Tabs.Tab value="video">Video</Tabs.Tab>
                <Tabs.Tab value="audio">Audio</Tabs.Tab>
                <Tabs.Tab value="all">All</Tabs.Tab>
              </Tabs.List>
            </Tabs>

            <TextInput
              placeholder="Search formats..."
              value={formatSearch}
              onChange={(e) => setFormatSearch(e.currentTarget.value)}
              mt="sm"
              size="xs"
              leftSection={<IconSearch size={14} />}
            />

            <ScrollArea h={250} mt="sm">
              <Stack gap={4}>
                {filteredFormats.map((fmt) => (
                  <FormatRow key={fmt.format_id} fmt={fmt} />
                ))}
                {filteredFormats.length === 0 && (
                  <Text c="dimmed" size="sm" ta="center" py="xl">No formats match</Text>
                )}
              </Stack>
            </ScrollArea>

            <Divider my="sm" />

            <Group gap="md" mb="sm">
              <Checkbox label="Audio only" checked={audioOnly} onChange={(e) => setAudioOnly(e.currentTarget.checked)} />
              {audioOnly && (
                <SegmentedControl value={audioFormat} onChange={setAudioFormat} data={audioFormats} size="xs" />
              )}
              <Checkbox label="Subtitles" checked={subtitlesEnabled} onChange={(e) => setSubtitlesEnabled(e.currentTarget.checked)} />
              {subtitlesEnabled && (
                <TextInput placeholder="zh-Hans,en" value={subtitleLangs} onChange={(e) => setSubtitleLangs(e.currentTarget.value)} w={150} size="xs" />
              )}
            </Group>

            <Group justify="flex-end" mt="md" gap="xs">
              <Button variant="outline" size="sm" onClick={() => setFormatDialogOpen(false)}>Cancel</Button>
              <Button
                size="sm"
                leftSection={<IconDownload size={16} />}
                onClick={handleConfirmDownload}
                disabled={selectedFormats.length === 0}
                loading={isDownloading}
              >
                {isDownloading ? 'Starting...' : 'Download'}
              </Button>
            </Group>
          </>
        )}
      </Modal>
    </Stack>
  );
}

function FormatRow({ fmt }: { fmt: FormatInfo }) {
  const { selectedFormats, toggleFormat } = useAppStore();
  const isSelected = selectedFormats.includes(fmt.format_id);
  const badge = fmt.is_combined ? 'Combined' : fmt.is_video ? 'Video' : 'Audio';
  const sizeStr = fmt.filesize
    ? formatSize(fmt.filesize)
    : fmt.filesize_approx ? `~${formatSize(fmt.filesize_approx)}`
    : fmt.approx_total_size ? `~${formatSize(fmt.approx_total_size)}`
    : 'N/A';

  return (
    <Paper
      p="xs"
      withBorder={isSelected}
      style={{ cursor: 'pointer', background: isSelected ? 'var(--mantine-primary-color-light)' : undefined, borderRadius: 6 }}
      onClick={() => toggleFormat(fmt.format_id)}
    >
      <Group gap="xs" wrap="nowrap">
        {isSelected ? <IconCircleFilled size={14} color="var(--mantine-primary-color-filled)" /> : <IconCircle size={14} color="gray" />}
        <Text size="xs" fw={600} w={70} c={fmt.is_combined ? 'blue' : fmt.is_video ? 'violet' : 'green'}>{badge}</Text>
        <Text size="xs" w={60} ta="right">{fmt.resolution}</Text>
        <Text size="xs" w={40} ta="right" c="dimmed">{fmt.ext}</Text>
        <Text size="xs" w={70} ta="right">{sizeStr}</Text>
        {fmt.fps && <Text size="xs" w={40} ta="right" c="dimmed">{fmt.fps}fps</Text>}
        <Text size="xs" style={{ flex: 1 }} c="dimmed">
          {fmt.is_combined ? `${fmt.vcodec}/${fmt.acodec}` : fmt.is_video ? fmt.vcodec : fmt.acodec}
        </Text>
      </Group>
    </Paper>
  );
}

function TaskCard({ task, showSpeedChart }: { task: DownloadTask; showSpeedChart: boolean }) {
  const { removeTask, cancelTask } = useAppStore();
  const isDownloading = task.status === 'Downloading';
  const isQueued = task.status === 'Queued';
  const isDone = task.status === 'Done';
  const isCancelled = task.status === 'Cancelled';
  const isFailed = typeof task.status === 'object' && 'Failed' in task.status;
  const speedHistory = (task.speed_history || []).map(([t, s]) => ({ time: Math.round(t), speed: s / 1024 / 1024 }));

  return (
    <Card withBorder padding="sm" radius="md">
      <Stack gap="xs">
        <Group justify="space-between">
          <Text size="xs" fw={500} lineClamp={1} style={{ flex: 1 }}>{task.title || task.url}</Text>
          <Badge size="xs" color={isDone ? 'green' : isCancelled ? 'gray' : isFailed ? 'red' : isDownloading ? 'blue' : 'yellow'}>
            {typeof task.status === 'string' ? task.status : 'Failed'}
          </Badge>
        </Group>
        {(isDownloading || isQueued) && (
          <>
            <Progress value={task.progress * 100} size="sm" animated radius="sm" />
            <Group gap="xs" style={{ fontSize: 11 }}>
              <Text c="dimmed">{formatBytes(task.downloaded_bytes)}{task.total_bytes ? ` / ${formatBytes(task.total_bytes)}` : ' / ???'}</Text>
              <Text c="dimmed">{task.speed}</Text>
              <Text c="dimmed">ETA {task.eta}</Text>
            </Group>
            {isDownloading && showSpeedChart && speedHistory.length > 2 && (
              <Box h={40}>
                <ResponsiveContainer width="100%" height="100%">
                  <AreaChart data={speedHistory}>
                    <defs>
                      <linearGradient id={`grad-${task.id}`} x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <Area type="monotone" dataKey="speed" stroke="#3b82f6" fill={`url(#grad-${task.id})`} strokeWidth={1} dot={false} />
                  </AreaChart>
                </ResponsiveContainer>
              </Box>
            )}
          </>
        )}
        <Group justify="flex-end" gap="xs">
          {(isDownloading || isQueued) && (
            <Tooltip label="Cancel">
              <ActionIcon variant="subtle" color="red" size="xs" onClick={() => cancelTask(task.id)}>
                <IconPlayerStop size={14} />
              </ActionIcon>
            </Tooltip>
          )}
          {(isDone || isCancelled || isFailed) && (
            <ActionIcon variant="subtle" color="gray" size="xs" onClick={() => removeTask(task.id)}>
              <IconTrash size={14} />
            </ActionIcon>
          )}
        </Group>
      </Stack>
    </Card>
  );
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${bytes} B`;
}
