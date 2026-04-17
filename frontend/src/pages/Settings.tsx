import {
  Paper,
  Stack,
  Text,
  TextInput,
  Switch,
  Button,
  Tabs,
  NumberInput,
  Group,
  Select,
  Textarea,
  Divider,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { useAppStore } from '../store/appStore';
import * as api from '../api/tauri';
import { useState, useEffect } from 'react';
import type { Config } from '../types';

export function SettingsPage() {
  const { config, setConfig } = useAppStore();
  const [localConfig, setLocalConfig] = useState<Config | null>(null);

  useEffect(() => {
    if (config) {
      setLocalConfig(JSON.parse(JSON.stringify(config)));
    }
  }, [config]);

  const handleSave = async () => {
    if (!localConfig) return;
    try {
      await api.saveConfig(localConfig);
      setConfig(localConfig);
      notifications.show({
        title: 'Success',
        message: 'Settings saved!',
        color: 'green',
      });
    } catch (e) {
      notifications.show({
        title: 'Error',
        message: `Failed to save: ${e}`,
        color: 'red',
      });
    }
  };

  if (!localConfig) {
    return (
      <Paper p="xl" withBorder>
        <Text ta="center" c="dimmed">Loading settings...</Text>
      </Paper>
    );
  }

  const g = localConfig.general;
  const d = localConfig.download;
  const e = localConfig.extractor;
  const pp = localConfig.post_processing;
  const adv = localConfig.advanced;
  const def = localConfig.defaults;

  return (
    <Stack gap="md">
      <Tabs defaultValue="general">
        <Tabs.List>
          <Tabs.Tab value="general">General</Tabs.Tab>
          <Tabs.Tab value="download">Download</Tabs.Tab>
          <Tabs.Tab value="extractor">Extractor</Tabs.Tab>
          <Tabs.Tab value="postprocessing">Post-Processing</Tabs.Tab>
          <Tabs.Tab value="advanced">Advanced</Tabs.Tab>
          <Tabs.Tab value="defaults">Defaults</Tabs.Tab>
        </Tabs.List>

        {/* General Tab */}
        <Tabs.Panel value="general">
          <Paper p="md" withBorder mt="sm">
            <Stack gap="md">
              <TextInput
                label="Download Directory"
                value={g.download_dir}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, download_dir: ev.currentTarget.value },
                  })
                }
              />
              <NumberInput
                label="Max Concurrent Downloads"
                value={g.max_concurrent}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, max_concurrent: (v as number) || 1 },
                  })
                }
                min={1}
                max={10}
              />
              <Select
                label="Theme"
                value={g.theme}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, theme: v || 'Light' },
                  })
                }
                data={['Light', 'Dark']}
              />
              <Select
                label="Language"
                value={g.language}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, language: v || 'English' },
                  })
                }
                data={['English', 'Chinese']}
              />
              <Switch
                label="Monitor Clipboard for URLs"
                checked={g.clipboard_monitor}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, clipboard_monitor: ev.currentTarget.checked },
                  })
                }
              />
              <Divider />
              <TextInput
                label="Output Template"
                value={g.output_template}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, output_template: ev.currentTarget.value },
                  })
                }
              />
              <Select
                label="Merge Output Format"
                value={g.merge_output_format}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, merge_output_format: v || 'mp4' },
                  })
                }
                data={['mp4', 'mkv', 'webm']}
              />
              <Switch
                label="Audio Only Mode (Default)"
                checked={g.audio_only}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, audio_only: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Show Speed Chart in Download Cards"
                checked={g.show_speed_chart}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    general: { ...g, show_speed_chart: ev.currentTarget.checked },
                  })
                }
              />
            </Stack>
          </Paper>
        </Tabs.Panel>

        {/* Download Tab */}
        <Tabs.Panel value="download">
          <Paper p="md" withBorder mt="sm">
            <Stack gap="md">
              <NumberInput
                label="Concurrent Fragments (-N)"
                value={d.concurrent_fragments}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, concurrent_fragments: (v as number) || 1 },
                  })
                }
                min={1}
              />
              <TextInput
                label="Rate Limit (e.g. 50K, 4.6M)"
                value={d.limit_rate}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, limit_rate: ev.currentTarget.value },
                  })
                }
              />
              <TextInput
                label="Throttled Rate (--throttled-rate)"
                value={d.throttled_rate}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, throttled_rate: ev.currentTarget.value },
                  })
                }
              />
              <NumberInput
                label="Retries"
                value={d.retries}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, retries: (v as number) || 1 },
                  })
                }
                min={1}
              />
              <NumberInput
                label="File Access Retries"
                value={d.file_access_retries}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, file_access_retries: (v as number) || 1 },
                  })
                }
                min={0}
              />
              <TextInput
                label="Download Archive File"
                value={d.download_archive}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, download_archive: ev.currentTarget.value },
                  })
                }
              />
              <Switch
                label="Abort on Error"
                checked={d.abort_on_error}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, abort_on_error: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Ignore Errors (-i)"
                checked={d.ignore_errors}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, ignore_errors: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Continue Downloads (--continue)"
                checked={d.continue_downloads}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, continue_downloads: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="No Overwrites (--no-overwrites)"
                checked={d.no_overwrites}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, no_overwrites: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="No Part File (--no-part)"
                checked={d.no_part}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, no_part: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="No Mtime (--no-mtime)"
                checked={d.no_mtime}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    download: { ...d, no_mtime: ev.currentTarget.checked },
                  })
                }
              />
            </Stack>
          </Paper>
        </Tabs.Panel>

        {/* Extractor Tab */}
        <Tabs.Panel value="extractor">
          <Paper p="md" withBorder mt="sm">
            <Stack gap="md">
              <NumberInput
                label="Extractor Retries"
                value={e.extractor_retries}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, extractor_retries: (v as number) || 1 },
                  })
                }
                min={1}
              />
              <Textarea
                label="Extractor Args (one per line)"
                value={e.extractor_args.join('\n')}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, extractor_args: ev.currentTarget.value.split('\n').filter(Boolean) },
                  })
                }
                autosize
                minRows={2}
              />
              <Switch
                label="Force Generic Extractor"
                checked={e.force_generic_extractor}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, force_generic_extractor: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Allow Unsafe URL"
                checked={e.allow_unsafe_url}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, allow_unsafe_url: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Extract Flat Playlist"
                checked={e.extract_flat}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, extract_flat: ev.currentTarget.checked },
                  })
                }
              />
              <TextInput
                label="External Downloader"
                value={e.external_downloader}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, external_downloader: ev.currentTarget.value },
                  })
                }
              />
              <TextInput
                label="External Downloader Args"
                value={e.external_downloader_args}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    extractor: { ...e, external_downloader_args: ev.currentTarget.value },
                  })
                }
              />
            </Stack>
          </Paper>
        </Tabs.Panel>

        {/* Post-Processing Tab */}
        <Tabs.Panel value="postprocessing">
          <Paper p="md" withBorder mt="sm">
            <Stack gap="md">
              <Switch
                label="Embed Thumbnail"
                checked={pp.embed_thumbnail}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, embed_thumbnail: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Embed Metadata"
                checked={pp.embed_metadata}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, embed_metadata: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Embed Subtitles"
                checked={pp.embed_subs}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, embed_subs: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Keep Video (--keep-video)"
                checked={pp.keep_video}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, keep_video: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="No Post Overwrites"
                checked={pp.no_post_overwrites}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, no_post_overwrites: ev.currentTarget.checked },
                  })
                }
              />
              <TextInput
                label="Convert Thumbnails (e.g. jpg, png)"
                value={pp.convert_thumbnails}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, convert_thumbnails: ev.currentTarget.value },
                  })
                }
              />
              <Textarea
                label="Post-Processor Args (one per line)"
                value={pp.postprocessor_args.join('\n')}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, postprocessor_args: ev.currentTarget.value.split('\n').filter(Boolean) },
                  })
                }
                autosize
                minRows={2}
              />
              <Divider />
              <TextInput
                label="SponsorBlock Remove (e.g. sponsor,intro)"
                value={pp.sponsorblock_remove}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, sponsorblock_remove: ev.currentTarget.value },
                  })
                }
              />
              <TextInput
                label="SponsorBlock API URL"
                value={pp.sponsorblock_api}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    post_processing: { ...pp, sponsorblock_api: ev.currentTarget.value },
                  })
                }
              />
            </Stack>
          </Paper>
        </Tabs.Panel>

        {/* Advanced Tab */}
        <Tabs.Panel value="advanced">
          <Paper p="md" withBorder mt="sm">
            <Stack gap="md">
              <TextInput
                label="Proxy"
                value={adv.proxy}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, proxy: ev.currentTarget.value },
                  })
                }
              />
              <TextInput
                label="Geo Verification Proxy"
                value={adv.geo_verification_proxy}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, geo_verification_proxy: ev.currentTarget.value },
                  })
                }
              />
              <TextInput
                label="User Agent"
                value={adv.user_agent}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, user_agent: ev.currentTarget.value },
                  })
                }
              />
              <TextInput
                label="Referer"
                value={adv.referer}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, referer: ev.currentTarget.value },
                  })
                }
              />
              <Textarea
                label="Custom Headers (one per line)"
                value={adv.custom_headers.join('\n')}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, custom_headers: ev.currentTarget.value.split('\n').filter(Boolean) },
                  })
                }
                autosize
                minRows={2}
              />
              <Switch
                label="Verbose Mode"
                checked={adv.verbose}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, verbose: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Geo Bypass"
                checked={adv.geo_bypass}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, geo_bypass: ev.currentTarget.checked },
                  })
                }
              />
              <TextInput
                label="Geo Bypass Country"
                value={adv.geo_bypass_country}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, geo_bypass_country: ev.currentTarget.value },
                  })
                }
              />
              <NumberInput
                label="Sleep Interval (seconds)"
                value={adv.sleep_interval}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, sleep_interval: (v as number) || 0 },
                  })
                }
                min={0}
              />
              <NumberInput
                label="Max Sleep Interval"
                value={adv.max_sleep_interval}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, max_sleep_interval: (v as number) || 0 },
                  })
                }
                min={0}
              />
              <Switch
                label="Prefer Free Formats"
                checked={adv.prefer_free_formats}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, prefer_free_formats: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Check Formats"
                checked={adv.check_formats}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, check_formats: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Simulate"
                checked={adv.simulate}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    advanced: { ...adv, simulate: ev.currentTarget.checked },
                  })
                }
              />
            </Stack>
          </Paper>
        </Tabs.Panel>

        {/* Defaults Tab */}
        <Tabs.Panel value="defaults">
          <Paper p="md" withBorder mt="sm">
            <Stack gap="md">
              <Select
                label="Default Video Quality"
                value={def.video_quality}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    defaults: { ...def, video_quality: v || 'ask' },
                  })
                }
                data={['ask', 'best', '2160', '1440', '1080', '720', '480', '360']}
              />
              <Select
                label="Default Audio Format"
                value={def.audio_format}
                onChange={(v) =>
                  setLocalConfig({
                    ...localConfig,
                    defaults: { ...def, audio_format: v || 'm4a' },
                  })
                }
                data={['m4a', 'mp3', 'flac', 'opus']}
              />
              <Switch
                label="Ask Each Time"
                checked={def.ask_each_time}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    defaults: { ...def, ask_each_time: ev.currentTarget.checked },
                  })
                }
              />
              <Switch
                label="Subtitles Enabled by Default"
                checked={def.subtitles_enabled}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    defaults: { ...def, subtitles_enabled: ev.currentTarget.checked },
                  })
                }
              />
              <TextInput
                label="Default Subtitle Languages"
                value={def.subtitle_langs}
                onChange={(ev) =>
                  setLocalConfig({
                    ...localConfig,
                    defaults: { ...def, subtitle_langs: ev.currentTarget.value },
                  })
                }
              />
            </Stack>
          </Paper>
        </Tabs.Panel>
      </Tabs>

      <Group justify="flex-end">
        <Button onClick={handleSave}>Save Settings</Button>
      </Group>
    </Stack>
  );
}
