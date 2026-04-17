export interface FormatInfo {
  format_id: string;
  ext: string;
  resolution: string;
  fps: number | null;
  vcodec: string;
  acodec: string;
  filesize: number | null;
  filesize_approx: number | null;
  note: string;
  is_video: boolean;
  is_audio: boolean;
  is_combined: boolean;
  height: number | null;
  approx_total_size: number | null;
}

export interface VideoInfo {
  title: string;
  thumbnail: string;
  uploader: string;
  duration: number | null;
  description: string;
}

export interface DownloadTask {
  id: number;
  url: string;
  title: string;
  format: string;
  audio_only: boolean;
  status: TaskStatus;
  progress: number;
  speed: string;
  eta: string;
  log: string[];
  downloaded_bytes: number;
  total_bytes: number | null;
  speed_bytes: number | null;
  eta_seconds: number | null;
  elapsed_seconds: number;
  speed_history: [number, number][];
  file_path: string | null;
}

export type TaskStatus =
  | 'Queued'
  | 'Fetching'
  | 'Downloading'
  | 'Done'
  | 'Cancelled'
  | { Failed: string };

export interface HistoryEntry {
  id: number;
  title: string;
  url: string;
  format: string;
  status: string;
  date: string;
  file_path: string;
}

export interface Config {
  general: GeneralConfig;
  download: DownloadConfig;
  extractor: ExtractorConfig;
  post_processing: PostProcessingConfig;
  advanced: AdvancedConfig;
  defaults: DefaultConfig;
}

export interface GeneralConfig {
  download_dir: string;
  max_concurrent: number;
  theme: string;
  language: string;
  clipboard_monitor: boolean;
  output_template: string;
  merge_output_format: string;
  audio_only: boolean;
  show_speed_chart: boolean;
}

export interface DownloadConfig {
  concurrent_fragments: number;
  limit_rate: string;
  throttled_rate: string;
  retries: number;
  file_access_retries: number;
  download_archive: string;
  abort_on_error: boolean;
  ignore_errors: boolean;
  continue_downloads: boolean;
  no_overwrites: boolean;
  no_part: boolean;
  no_mtime: boolean;
}

export interface ExtractorConfig {
  extractor_args: string[];
  extractor_retries: number;
  force_generic_extractor: boolean;
  allow_unsafe_url: boolean;
  extract_flat: boolean;
  external_downloader: string;
  external_downloader_args: string;
}

export interface PostProcessingConfig {
  embed_thumbnail: boolean;
  embed_metadata: boolean;
  embed_subs: boolean;
  postprocessor_args: string[];
  keep_video: boolean;
  no_post_overwrites: boolean;
  convert_thumbnails: string;
  sponsorblock_remove: string;
  sponsorblock_api: string;
}

export interface AdvancedConfig {
  verbose: boolean;
  custom_headers: string[];
  user_agent: string;
  referer: string;
  proxy: string;
  geo_verification_proxy: string;
  geo_bypass: boolean;
  geo_bypass_country: string;
  sleep_interval: number;
  max_sleep_interval: number;
  prefer_free_formats: boolean;
  check_formats: boolean;
  simulate: boolean;
}

export interface DefaultConfig {
  video_quality: string;
  audio_format: string;
  ask_each_time: boolean;
  subtitles_enabled: boolean;
  subtitle_langs: string;
}

export type Page = 'download' | 'queue' | 'history' | 'settings' | 'test';

export type Browser = 'Chrome' | 'Edge' | 'Firefox' | 'NoCookies';

export interface DownloadParams {
  url: string;
  title: string;
  formatIds: string[];
  saveDir: string;
  audioOnly: boolean;
  audioFormat: string;
  cookieArgs: string[];
  subtitlesEnabled: boolean;
  subtitleLangs: string;
}

export interface ProgressEvent {
  id: number;
  progress: number;
  speed: number | null;
  eta: number | null;
  downloaded: number;
  total: number | null;
  speed_history: [number, number][];
}
