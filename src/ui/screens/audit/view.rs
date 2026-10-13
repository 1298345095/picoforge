//! Audit screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::models::device::audit;
use crate::ui::screens::audit::view_model::AuditViewModel;
use gpui::*;
use gpui_component::{ActiveTheme, Icon, StyledExt, Theme, h_flex, v_flex};

fn empty_state(heading: &str, body: String, theme: &Theme) -> AnyElement {
    v_flex()
        .items_center()
        .justify_center()
        .h_64()
        .gap_2()
        .border_1()
        .border_color(theme.border)
        .rounded_xl()
        .child(div().font_semibold().child(heading.to_string()))
        .child(
            div()
                .text_sm()
                .max_w(px(380.))
                .text_color(theme.muted_foreground)
                .child(body),
        )
        .into_any_element()
}

fn mono(theme: &Theme, s: String) -> AnyElement {
    div()
        .font_family("monospace")
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(s)
        .into_any_element()
}

fn short_hex(bytes: &[u8; 32]) -> String {
    let h = hex::encode(bytes);
    format!("{}…{}", &h[..8], &h[h.len() - 8..])
}

impl AuditViewModel {
    fn entry_row(entry: &audit::AuditEntry, theme: &Theme) -> AnyElement {
        h_flex()
            .gap_3()
            .py_1()
            .text_sm()
            .child(
                div()
                    .w(px(56.))
                    .text_color(theme.muted_foreground)
                    .child(entry.seq.to_string()),
            )
            .child(
                div()
                    .w(px(72.))
                    .text_color(theme.muted_foreground)
                    .child(format!("{:.1}s", entry.uptime_s())),
            )
            .child(div().w(px(160.)).font_medium().child(entry.event_label()))
            .child(
                div()
                    .w(px(40.))
                    .text_color(theme.muted_foreground)
                    .child(entry.aux.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(entry.detail_hex()),
            )
            .into_any_element()
    }

    fn journal_body(&self, theme: &Theme) -> AnyElement {
        let Some(j) = &self.journal else {
            return div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("读取日志以查看安全事件记录。")
                .into_any_element();
        };

        let header = h_flex()
            .gap_3()
            .pb_1()
            .text_xs()
            .font_semibold()
            .text_color(theme.muted_foreground)
            .child(div().w(px(56.)).child("seq"))
            .child(div().w(px(72.)).child("uptime"))
            .child(div().w(px(160.)).child("event"))
            .child(div().w(px(40.)).child("aux"))
            .child(div().flex_1().child("detail"));

        let mut rows = vec![header.into_any_element()];
        for e in &j.entries {
            rows.push(Self::entry_row(e, theme));
        }
        if j.entries.is_empty() {
            rows.push(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("当前范围内没有实时记录。")
                    .into_any_element(),
            );
        }

        v_flex()
            .gap_2()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child(format!(
                        "范围 [{}, {}) — {} 条记录，其中 {} 条已合并到纪元",
                        j.start,
                        j.seq_next,
                        j.entries.len(),
                        j.start,
                    )))
                    .child(mono(theme, format!("纪元  {}", short_hex(&j.epoch))))
                    .child(mono(
                        theme,
                        format!("链头   {}  （链验证通过）", short_hex(&j.head)),
                    )),
            )
            .child(div().h(px(1.)).bg(theme.border))
            .child(v_flex().gap_0p5().children(rows))
            .into_any_element()
    }

    fn verify_body(&self, theme: &Theme) -> AnyElement {
        let Some(v) = &self.verification else {
            return div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("验证签名检查点，以确认日志真实且设备可信。")
                .into_any_element();
        };

        let (label, color) = if v.authentic() {
            ("验证通过 ✓", theme.green)
        } else {
            ("不可信 ✗", theme.danger)
        };

        let kv = |k: &str, val: String| {
            v_flex()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(k.to_string()),
                )
                .child(div().font_family("monospace").text_xs().child(val))
        };

        let expected_line = match v.expected_match {
            Some(true) => Some(("已固定的密钥", "匹配 ✓".to_string())),
            Some(false) => Some(("已固定的密钥", "不匹配 ✗".to_string())),
            None => None,
        };

        let mut col = v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(10.)).h(px(10.)).rounded_full().bg(color))
                    .child(div().font_semibold().text_color(color).child(label)),
            )
            .child(div().text_sm().child(format!(
                "签名 {} · 链头 {} · 检查点序号 {}",
                if v.signature_ok { "有效" } else { "无效" },
                if v.head_matches { "bound" } else { "不匹配" },
                v.seq_signed,
            )))
            .child(kv("证明密钥", v.pubkey_hex.clone()))
            .child(kv(
                "指纹（稍后可通过“预期密钥”固定）",
                v.fingerprint.clone(),
            ));
        if let Some((k, val)) = expected_line {
            col = col.child(kv(k, val));
        }
        col.into_any_element()
    }
}

impl Render for AuditViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const TITLE: &str = "安全审计";
        const SUBTITLE: &str = "防篡改安全日志。";

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(TITLE, SUBTITLE, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let read_btn = PFButton::new("读取日志")
            .id("audit-read")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_read(window, cx)));
        let verify_btn = PFButton::new("验证")
            .id("audit-verify")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_verify(window, cx)));
        let toggle_btn = match self.enabled {
            Some(true) => Some(
                PFButton::new("禁用")
                    .id("audit-disable")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .disabled(self.loading)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_toggle(false, window, cx)),
                    ),
            ),
            Some(false) => Some(
                PFButton::new("启用")
                    .id("audit-enable")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .disabled(self.loading)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_toggle(true, window, cx)),
                    ),
            ),
            None => None,
        };

        let theme = cx.theme();
        let journal_body = self.journal_body(theme);
        let verify_body = self.verify_body(theme);

        let (dot, status_text) = match self.enabled {
            Some(true) => (
                theme.green,
                "已启用——安全事件将记录到密钥的闪存中。",
            ),
            Some(false) => (
                theme.muted_foreground,
                "已关闭——安全日志需主动启用，目前不会记录事件。",
            ),
            None => (theme.muted_foreground, "正在读取状态…"),
        };
        let status_body = h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(10.)).h(px(10.)).rounded_full().bg(dot))
            .child(div().text_sm().child(status_text.to_string()));
        let status_card = {
            let mut c = Card::new()
                .title("日志记录")
                .description("开启或关闭防篡改日志（需要 PIN 和触摸确认）")
                .icon(Icon::default().path("icons/book-open.svg"));
            if let Some(btn) = toggle_btn {
                c = c.header_right(btn);
            }
            c.child(status_body)
        };

        let journal_card = Card::new()
            .title("审计日志")
            .description("采用哈希链记录安全事件（启动、FIDO 操作、PIN、配置）")
            .icon(Icon::default().path("icons/scroll-text.svg"))
            .header_right(read_btn)
            .child(journal_body);

        let verify_card = Card::new()
            .title("检查点验证")
            .description("由 DEVK 签名的真实性与设备身份凭证")
            .icon(Icon::default().path("icons/shield-check.svg"))
            .header_right(verify_btn)
            .child(verify_body);

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .child(journal_card)
            .child(verify_card);
        PageView::build(TITLE, SUBTITLE, content, theme).into_any_element()
    }
}
