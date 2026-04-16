use iced::widget::{
    button, checkbox, container, row, scrollable, text_input, vertical_space, Column,
};
use iced::{Alignment, Element, Length};
use std::collections::HashSet;

use crate::downloader::FormatInfo;

/// 格式筛选类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatFilter {
    #[default]
    Video,
    Audio,
    Combined,
    All,
}

impl FormatFilter {
    pub fn label(&self) -> &'static str {
        match self {
            FormatFilter::Video => "视频",
            FormatFilter::Audio => "音频",
            FormatFilter::Combined => "音视频",
            FormatFilter::All => "全部",
        }
    }
}

/// 格式选择弹窗状态。
#[derive(Debug, Clone, Default)]
pub struct FormatDialog {
    pub open: bool,
    pub selected_format_id: String,
    pub filter: FormatFilter,
    pub search: String,
    pub download_mode: bool,
    /// Selected format IDs for multi-download mode
    pub selected_formats: HashSet<String>,
}

impl FormatDialog {
    pub fn open(&mut self, current_format_id: &str, download_mode: bool) {
        self.open = true;
        self.selected_format_id = current_format_id.to_string();
        self.search.clear();
        self.filter = FormatFilter::Video;
        self.download_mode = download_mode;
        // In download mode, start with all video formats selected
        if download_mode {
            self.selected_formats.clear();
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.search.clear();
    }

    /// 渲染弹窗覆盖层。
    pub fn view<'a, Message>(
        &self,
        formats: &'a [FormatInfo],
        on_select: impl Fn(String) -> Message + 'static,
        on_close: impl Fn() -> Message + 'static,
        on_filter: impl Fn(FormatFilter) -> Message + 'static,
        on_search: impl Fn(String) -> Message + 'static,
        on_download: Option<impl Fn(Vec<String>) -> Message + 'static>,
        on_toggle_format: Option<impl Fn(String, bool) -> Message + 'static>,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'static,
    {
        let backdrop = container(vertical_space())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color {
                    r: 0.0, g: 0.0, b: 0.0, a: 0.5,
                })),
                ..Default::default()
            });

        let dialog = self.dialog_content(formats, on_select, on_close, on_filter, on_search, on_download, on_toggle_format);

        let overlay = container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        iced::widget::stack![backdrop, overlay].into()
    }

    fn dialog_content<'a, Message>(
        &self,
        formats: &'a [FormatInfo],
        on_select: impl Fn(String) -> Message + 'static,
        on_close: impl Fn() -> Message + 'static,
        on_filter: impl Fn(FormatFilter) -> Message + 'static,
        on_search: impl Fn(String) -> Message + 'static,
        on_download: Option<impl Fn(Vec<String>) -> Message + 'static>,
        on_toggle_format: Option<impl Fn(String, bool) -> Message + 'static>,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'static,
    {
        let bg = iced::Color { r: 0.12, g: 0.12, b: 0.14, a: 1.0 };

        let mut col = Column::new().spacing(12).padding(20);

        // Title bar
        let close_msg = on_close();
        let title_text = if self.download_mode { "选择格式并下载" } else { "选择格式" };
        col = col.push(
            row![
                cjk_text(title_text).size(18).color(iced::Color::WHITE),
                iced::widget::horizontal_space(),
                button(cjk_text("\u{2715}").size(14))
                    .padding(6)
                    .on_press(close_msg),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );

        // Filter tabs
        let filters = [FormatFilter::Video, FormatFilter::Audio, FormatFilter::Combined, FormatFilter::All];
        let mut filter_row = row![].spacing(8);
        for f in filters {
            let is_active = self.filter == f;
            let filter_msg = on_filter(f);
            let btn = button(cjk_text(f.label()).size(13).color(if is_active { iced::Color::WHITE } else { iced::Color { r: 0.85, g: 0.85, b: 0.85, a: 1.0 } }))
                .padding(6)
                .on_press(filter_msg);
            let btn = if is_active {
                btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        r: 0.2, g: 0.55, b: 1.0, a: 1.0,
                    })),
                    text_color: iced::Color::WHITE,
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            } else {
                btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        r: 0.25, g: 0.25, b: 0.28, a: 1.0,
                    })),
                    text_color: iced::Color { r: 0.85, g: 0.85, b: 0.85, a: 1.0 },
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            };
            filter_row = filter_row.push(btn);
        }
        col = col.push(filter_row);

        // Search
        col = col.push(
            text_input("搜索格式...", &self.search)
                .on_input(on_search)
                .padding(8)
                .size(13)
                .style(|_, _| text_input::Style {
                    background: iced::Background::Color(iced::Color { r: 0.2, g: 0.2, b: 0.22, a: 1.0 }),
                    border: iced::border::rounded(4),
                    icon: iced::Color::WHITE,
                    placeholder: iced::Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
                    value: iced::Color::WHITE,
                    selection: iced::Color { r: 0.3, g: 0.5, b: 1.0, a: 0.4 },
                }),
        );

        // Format list
        let filtered = Self::filter_formats(self, formats);
        let mut list = Column::new().spacing(4);

        if filtered.is_empty() {
            list = list.push(cjk_text("没有匹配的格式").size(13).color(iced::Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 }));
        } else if self.download_mode {
            // Multi-select mode with checkboxes
            for fmt in &filtered {
                let label = self.format_row_label(fmt);
                let is_selected = self.selected_formats.contains(&fmt.format_id);
                let fmt_id = fmt.format_id.clone();

                let toggle_msg = on_toggle_format.as_ref().map(|cb| {
                    cb(fmt_id.clone(), !is_selected)
                });

                let row = if let Some(msg) = toggle_msg {
                    row![
                        checkbox("", is_selected)
                            .on_toggle(move |_| msg.clone())
                            .size(14),
                        cjk_text(&label).size(12).color(iced::Color::WHITE),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                } else {
                    row![
                        checkbox("", is_selected).size(14),
                        cjk_text(&label).size(12).color(iced::Color::WHITE),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                };

                list = list.push(
                    container(row)
                        .padding(6)
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: if is_selected {
                                Some(iced::Background::Color(iced::Color {
                                    r: 0.0, g: 0.47, b: 1.0, a: 0.15,
                                }))
                            } else {
                                None
                            },
                            border: iced::border::rounded(6),
                            ..Default::default()
                        }),
                );
            }

            // Show selection count
            if !self.selected_formats.is_empty() {
                list = list.push(
                    cjk_text(format!("已选 {} 个格式", self.selected_formats.len()))
                        .size(12).color(iced::Color { r: 0.4, g: 0.8, b: 0.4, a: 1.0 }),
                );
            }
        } else {
            // Single-select mode
            for fmt in &filtered {
                let label = self.format_row_label(fmt);
                let is_selected = self.selected_format_id == fmt.format_id;
                let fmt_id = fmt.format_id.clone();
                let select_msg = on_select(fmt_id);
                let btn = button(cjk_text(&label).size(12).color(iced::Color::WHITE))
                    .padding(8)
                    .width(Length::Fill)
                    .on_press(select_msg);
                let btn = if is_selected {
                    btn.style(|_, _| button::Style {
                        background: Some(iced::Background::Color(iced::Color {
                            r: 0.0, g: 0.47, b: 1.0, a: 0.3,
                        })),
                        border: iced::border::rounded(6),
                        ..Default::default()
                    })
                } else {
                    btn
                };
                list = list.push(btn);
            }
        }

        col = col.push(
            container(scrollable(list).height(Length::Fixed(300.0)))
                .padding(4),
        );

        // Selected format display (single-select mode only)
        if !self.download_mode {
            if !self.selected_format_id.is_empty() && self.selected_format_id != "best" {
                if let Some(fmt) = formats.iter().find(|f| f.format_id == self.selected_format_id) {
                    col = col.push(
                        cjk_text(format!("已选: {} ({})", fmt.resolution, fmt.format_id))
                            .size(12).color(iced::Color { r: 0.4, g: 0.8, b: 0.4, a: 1.0 }),
                    );
                }
            } else if self.selected_format_id == "best" {
                col = col.push(cjk_text("已选: 最佳质量").size(12).color(iced::Color { r: 0.4, g: 0.8, b: 0.4, a: 1.0 }));
            }
        }

        // Action buttons
        col = col.push(
            if self.download_mode {
                // Download mode: Cancel + Download buttons
                let selected: Vec<String> = self.selected_formats.iter().cloned().collect();
                let download_msg = on_download.as_ref().map(|cb| cb(selected));
                let cancel_msg = on_close();
                row![
                    button(cjk_text("取消").size(13))
                        .padding(10)
                        .on_press(cancel_msg),
                    iced::widget::horizontal_space(),
                    button(cjk_text(format!("下载 ({} 个格式)", self.selected_formats.len())).size(13))
                        .padding(10)
                        .style(|_, _| button::Style {
                            background: Some(iced::Background::Color(iced::Color {
                                r: 0.0, g: 0.55, b: 0.0, a: 1.0,
                            })),
                            text_color: iced::Color::WHITE,
                            border: iced::border::rounded(6),
                            ..Default::default()
                        })
                        .on_press(download_msg.unwrap_or(on_close())),
                ]
                .spacing(12)
            } else {
                // Normal mode: Cancel + Confirm buttons
                let confirm_msg = on_close();
                row![
                    button(cjk_text("取消").size(13))
                        .padding(10)
                        .on_press(on_close()),
                    iced::widget::horizontal_space(),
                    button(cjk_text("确认").size(13))
                        .padding(10)
                        .style(|_, _| button::Style {
                            background: Some(iced::Background::Color(iced::Color {
                                r: 0.0, g: 0.47, b: 1.0, a: 1.0,
                            })),
                            text_color: iced::Color::WHITE,
                            border: iced::border::rounded(6),
                            ..Default::default()
                        })
                        .on_press(confirm_msg),
                ]
                .spacing(12)
            },
        );

        container(col)
            .width(Length::Fixed(500.0))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(12),
                ..Default::default()
            })
            .into()
    }

    fn filter_formats<'a>(&'a self, formats: &'a [FormatInfo]) -> Vec<&'a FormatInfo> {
        formats
            .iter()
            .filter(|fmt| match self.filter {
                FormatFilter::Video => fmt.is_video && !fmt.is_combined,
                FormatFilter::Audio => fmt.is_audio,
                FormatFilter::Combined => fmt.is_combined,
                FormatFilter::All => true,
            })
            .filter(|fmt| {
                if self.search.is_empty() {
                    return true;
                }
                let lower = self.search.to_lowercase();
                fmt.resolution.to_lowercase().contains(&lower)
                    || fmt.ext.to_lowercase().contains(&lower)
                    || fmt.format_id.to_lowercase().contains(&lower)
                    || fmt.note.to_lowercase().contains(&lower)
                    || fmt.vcodec.to_lowercase().contains(&lower)
                    || fmt.acodec.to_lowercase().contains(&lower)
            })
            .collect()
    }

    fn format_row_label(&self, fmt: &FormatInfo) -> String {
        let size_str = if let Some(size) = fmt.filesize {
            format_size(size)
        } else if let Some(size) = fmt.filesize_approx {
            format!("~{}", format_size(size))
        } else if let Some(size) = fmt.approx_total_size {
            format!("~{}", format_size(size))
        } else {
            "N/A".to_string()
        };

        let type_badge = if fmt.is_combined {
            "[音视频]"
        } else if fmt.is_video {
            "[视频]"
        } else {
            "[音频]"
        };

        let codec_str = if fmt.is_combined {
            format!("{}/{}", fmt.vcodec, fmt.acodec)
        } else if fmt.is_video {
            format!("{}/{}", fmt.vcodec, fmt.acodec)
        } else {
            fmt.acodec.clone()
        };

        format!(
            "{}  {:>6}  {:>5}  {:>9}  {}",
            type_badge, fmt.resolution, fmt.ext, size_str, codec_str
        )
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn cjk_text(content: impl std::fmt::Display) -> iced::widget::Text<'static, iced::Theme> {
    iced::widget::text(content.to_string())
}
