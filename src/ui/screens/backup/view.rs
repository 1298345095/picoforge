//! Backup screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::backup::view_model::BackupViewModel;
use gpui::*;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::{ActiveTheme, Disableable, Icon, StyledExt, Theme, h_flex, v_flex};

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

impl BackupViewModel {
    fn action_row(
        &self,
        title: &'static str,
        subtitle: &'static str,
        btn: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        h_flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_0p5()
                    .child(div().font_medium().child(title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(subtitle),
                    ),
            )
            .child(btn)
    }

    fn exported_card(&self, phrase: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let copy = {
            let p = phrase.to_string();
            Button::new("bk-copy")
                .label("复制")
                .ghost()
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(p.clone()));
                }))
        };
        let clear = Button::new("bk-clear")
            .label("从屏幕清除")
            .ghost()
            .on_click(cx.listener(|this, _, _, cx| this.clear_exported(cx)));

        Card::new()
            .title("恢复助记词")
            .description("仅显示一次——请立即抄写并关闭导出窗口")
            .icon(Icon::default().path("icons/key-round.svg"))
            .header_right(h_flex().gap_2().child(copy).child(clear))
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .text_color(rgb(0xf59e0b))
                            .text_sm()
                            .child("任何获得此助记词的人都可以克隆你的 FIDO 身份。请离线保存，切勿粘贴到网站上。"),
                    )
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .font_family("monospace")
                            .text_sm()
                            .child(phrase.to_string()),
                    ),
            )
            .into_any_element()
    }
}

impl Render for BackupViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const TITLE: &str = "备份";
        const SUBTITLE: &str = "使用助记词备份与恢复 FIDO 种子。";

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(TITLE, SUBTITLE, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let status = self.status;
        let exported = self.exported.clone();

        let exported_card = exported.map(|p| self.exported_card(&p, cx));
        let theme = cx.theme();

        let refresh_btn = Button::new("bk-refresh")
            .icon(Icon::default().path("icons/refresh-cw.svg"))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(rgb(0x1b1b1d).into())
                    .hover(rgb(0x232325).into())
                    .active(rgb(0x3f3f46).into())
                    .border(theme.border),
            )
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)));
        let export_btn = Button::new("bk-export")
            .label("导出种子")
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_export(window, cx)));
        let seal_btn = PFButton::new("关闭导出窗口")
            .id("bk-seal")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_finalize(window, cx)));
        let restore_btn = Button::new("bk-restore")
            .label("恢复种子")
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_restore(window, cx)));

        let status_card = {
            let body = match status {
                Some(s) => {
                    let yn = |b: bool| if b { "yes" } else { "no" };
                    let export_state = if s.sealed {
                        "已封闭（恢复出厂设置前禁止导出）"
                    } else if s.has_seed {
                        "已开放——种子可导出一次"
                    } else {
                        "不存在种子"
                    };
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .child(format!("存在种子：{}", yn(s.has_seed))),
                        )
                        .child(div().text_sm().child(format!("导出窗口：{export_state}")))
                        .into_any_element()
                }
                None => div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("正在读取备份状态…")
                    .into_any_element(),
            };
            Card::new()
                .title("备份状态")
                .description("查看是否存在种子以及导出窗口是否开放")
                .icon(Icon::default().path("icons/cpu.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let export_card = Card::new()
            .title("导出")
            .description("将种子显示为 24 个助记词，然后关闭导出窗口")
            .icon(Icon::default().path("icons/lock-open.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        "导出种子",
                        "显示恢复助记词（请离线操作；需要 PIN 或触摸确认）",
                        export_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "关闭导出窗口",
                        "在恢复出厂设置之前禁止再次导出",
                        seal_btn,
                        theme,
                    )),
            );

        let restore_card = Card::new()
            .title("恢复")
            .description("通过 24 个助记词导入种子")
            .icon(Icon::default().path("icons/lock.svg"))
            .child(self.action_row(
                "恢复种子",
                "使用恢复助记词替换 FIDO 身份",
                restore_btn,
                theme,
            ));

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .children(exported_card)
            .child(export_card)
            .child(restore_card);

        PageView::build(TITLE, SUBTITLE, content, theme).into_any_element()
    }
}
