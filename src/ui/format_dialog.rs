use iced::widget::{
    button, container, row, scrollable, text_input, vertical_space, Column,
};
use iced::{Alignment, Element, Font, Length};

use crate::downloader::FormatInfo;

/// 格式筛选类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatFilter {
    #[default]
    Video,
    Audio,
    All,
}

impl FormatFilter {
    pub fn label(&self) -> &'static str {
        match self {
            FormatFilter::Video => "视频",
            FormatFilter::Audio => "音频",
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
}

impl FormatDialog {
    pub fn open(&mut self, current_format_id: &str) {
        self.open = true;
        self.selected_format_id = current_format_id.to_string();
        self.search.clear();
        self.filter = FormatFilter::Video;
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

        let dialog = self.dialog_content(formats, on_select, on_close, on_filter, on_search);

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
    ) -> Element<'a, Message>
    where
        Message: Clone + 'static,
    {
        let bg = iced::Color { r: 0.18, g: 0.18, b: 0.20, a: 1.0 };

        let mut col = Column::new().spacing(12).padding(20);

        // Title bar
        let close_msg = on_close();
        col = col.push(
            row![
                cjk_text("选择格式").size(18),
                iced::widget::horizontal_space(),
                button(cjk_text("\u{2715}").size(14))
                    .padding(6)
                    .on_press(close_msg),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );

        // Filter tabs
        let filters = [FormatFilter::Video, FormatFilter::Audio, FormatFilter::All];
        let mut filter_row = row![].spacing(8);
        for f in filters {
            let is_active = self.filter == f;
            let filter_msg = on_filter(f);
            let btn = button(cjk_text(f.label()).size(13))
                .padding(6)
                .on_press(filter_msg);
            let btn = if is_active {
                btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        r: 0.0, g: 0.47, b: 1.0, a: 1.0,
                    })),
                    text_color: iced::Color::WHITE,
                    border: iced::border::rounded(6),
                    ..Default::default()
                })
            } else {
                btn
            };
            filter_row = filter_row.push(btn);
        }
        col = col.push(filter_row);

        // Search
        col = col.push(
            text_input("搜索格式...", &self.search)
                .on_input(on_search)
                .padding(8)
                .size(13),
        );

        // Format list
        let filtered = Self::filter_formats(self, formats);
        let mut list = Column::new().spacing(4);

        if filtered.is_empty() {
            list = list.push(cjk_text("没有匹配的格式").size(13));
        } else {
            for fmt in &filtered {
                let label = self.format_row_label(fmt);
                let is_selected = self.selected_format_id == fmt.format_id;
                let fmt_id = fmt.format_id.clone();
                let select_msg = on_select(fmt_id);
                let btn = button(cjk_text(&label).size(12))
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

        // Selected format display
        if !self.selected_format_id.is_empty() && self.selected_format_id != "best" {
            if let Some(fmt) = formats.iter().find(|f| f.format_id == self.selected_format_id) {
                col = col.push(
                    cjk_text(format!("已选: {} ({})", fmt.resolution, fmt.format_id))
                        .size(12),
                );
            }
        } else if self.selected_format_id == "best" {
            col = col.push(cjk_text("已选: 最佳质量").size(12));
        }

        // Action buttons
        let cancel_msg = on_close();
        let confirm_msg = on_close();
        col = col.push(
            row![
                button(cjk_text("取消").size(13))
                    .padding(10)
                    .on_press(cancel_msg),
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
            .spacing(12),
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
                FormatFilter::Video => fmt.is_video,
                FormatFilter::Audio => fmt.is_audio,
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
        let size_str = fmt
            .filesize
            .map(|s| format_size(s))
            .unwrap_or_else(|| "N/A".to_string());

        let codec_str = if fmt.is_video {
            format!("{}/{}", fmt.vcodec, fmt.acodec)
        } else {
            fmt.acodec.clone()
        };

        format!(
            "{:>6}  {:>5}  {:>8}  {}",
            fmt.resolution, fmt.ext, size_str, codec_str
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
    iced::widget::text(content.to_string()).font(Font::with_name("Microsoft YaHei"))
}
