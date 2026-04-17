import {
  Paper,
  Stack,
  Text,
  Progress,
  Group,
  ActionIcon,
  Badge,
  Box,
} from '@mantine/core';
import {
  IconPlayerPause,
  IconPlayerPlay,
  IconX,
  IconTrash,
} from '@tabler/icons-react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { useAppStore } from '../store/appStore';
import * as api from '../api/tauri';
import type { DownloadTask } from '../types';

export function QueuePage() {
  const { tasks, removeTask, cancelTask } = useAppStore();

  if (tasks.length === 0) {
    return (
      <Paper p="xl" withBorder>
        <Text ta="center" c="dimmed">
          No downloads in queue. Fetch a video and click 'Download Now' to get started.
        </Text>
      </Paper>
    );
  }

  return (
    <Stack gap="md">
      {tasks.map((task) => (
        <TaskCard key={task.id} task={task} />
      ))}
    </Stack>
  );
}

function TaskCard({ task }: { task: DownloadTask }) {
  const { removeTask, cancelTask } = useAppStore();

  const isDone = task.status === 'Done';
  const isCancelled = task.status === 'Cancelled';
  const isFailed = typeof task.status === 'object' && 'Failed' in task.status;
  const isDownloading = task.status === 'Downloading';

  const statusColor = isDone
    ? 'green'
    : isCancelled
    ? 'gray'
    : isFailed
    ? 'red'
    : isDownloading
    ? 'blue'
    : 'yellow';

  const statusLabel = typeof task.status === 'string' ? task.status : 'Failed';

  const speedHistory = task.speed_history.map(([t, s]) => ({
    time: Math.round(t),
    speed: s / 1024 / 1024, // MB/s
  }));

  return (
    <Paper p="md" withBorder>
      <Stack gap="xs">
        <Group justify="space-between">
          <Text fw={600} size="sm" lineClamp={1}>
            {task.title || task.url}
          </Text>
          <Badge color={statusColor}>{statusLabel}</Badge>
        </Group>

        {/* Progress Bar */}
        {isDownloading && (
          <Stack gap={2}>
            <Progress value={task.progress * 100} size="sm" animated />
            <Group gap="xs" style={{ fontSize: 12 }}>
              <Text size="xs" c="dimmed">
                {formatBytes(task.downloaded_bytes)}
                {task.total_bytes ? ` / ${formatBytes(task.total_bytes)}` : ' / ???'}
              </Text>
              <Text size="xs" c="dimmed">{task.speed}</Text>
              <Text size="xs" c="dimmed">ETA: {task.eta}</Text>
            </Group>
          </Stack>
        )}

        {/* Speed Chart */}
        {task.speed_history.length > 2 && isDownloading && (
          <Box h={60}>
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={speedHistory}>
                <defs>
                  <linearGradient id={`grad-${task.id}`} x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <Area
                  type="monotone"
                  dataKey="speed"
                  stroke="#3b82f6"
                  fill={`url(#grad-${task.id})`}
                  strokeWidth={1}
                  dot={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </Box>
        )}

        {/* Actions */}
        <Group justify="flex-end" gap="xs">
          {isDownloading && (
            <ActionIcon
              variant="subtle"
              color="yellow"
              onClick={() => api.cancelDownload(task.id)}
            >
              <IconPlayerPause size={16} />
            </ActionIcon>
          )}
          {(isDone || isCancelled || isFailed) && (
            <ActionIcon
              variant="subtle"
              color="gray"
              onClick={() => removeTask(task.id)}
            >
              <IconTrash size={16} />
            </ActionIcon>
          )}
        </Group>
      </Stack>
    </Paper>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${bytes} B`;
}
