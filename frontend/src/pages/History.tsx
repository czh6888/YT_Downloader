import {
  Paper,
  Stack,
  Text,
  TextInput,
  ActionIcon,
  Group,
  Table,
  Badge,
  ScrollArea,
} from '@mantine/core';
import { IconSearch, IconTrash } from '@tabler/icons-react';
import { useAppStore } from '../store/appStore';

export function HistoryPage() {
  const { history, historySearch, setHistorySearch, deleteHistoryEntry } = useAppStore();

  const filtered = historySearch
    ? history.filter(
        (h) =>
          h.title.toLowerCase().includes(historySearch.toLowerCase()) ||
          h.url.toLowerCase().includes(historySearch.toLowerCase())
      )
    : history;

  if (history.length === 0) {
    return (
      <Paper p="xl" withBorder>
        <Text ta="center" c="dimmed">No downloads yet.</Text>
      </Paper>
    );
  }

  return (
    <Stack gap="md">
      <TextInput
        placeholder="Search history..."
        value={historySearch}
        onChange={(e) => setHistorySearch(e.currentTarget.value)}
        leftSection={<IconSearch size={16} />}
      />

      <ScrollArea h={500}>
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Title</Table.Th>
              <Table.Th>Date</Table.Th>
              <Table.Th>Format</Table.Th>
              <Table.Th>Status</Table.Th>
              <Table.Th w={60}></Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {filtered.map((entry) => (
              <Table.Tr key={entry.id}>
                <Table.Td>
                  <Text size="sm" lineClamp={1} fw={500}>
                    {entry.title}
                  </Text>
                  <Text size="xs" c="dimmed" lineClamp={1}>
                    {entry.url}
                  </Text>
                </Table.Td>
                <Table.Td>{entry.date}</Table.Td>
                <Table.Td>
                  <Badge variant="outline" size="sm">{entry.format}</Badge>
                </Table.Td>
                <Table.Td>
                  <Badge
                    color={entry.status === 'completed' ? 'green' : entry.status === 'failed' ? 'red' : 'gray'}
                    size="sm"
                  >
                    {entry.status}
                  </Badge>
                </Table.Td>
                <Table.Td>
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    onClick={() => deleteHistoryEntry(entry.id)}
                  >
                    <IconTrash size={16} />
                  </ActionIcon>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </ScrollArea>

      <Text size="xs" c="dimmed">{filtered.length} entries</Text>
    </Stack>
  );
}
