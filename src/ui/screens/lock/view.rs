//! Lock screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::screens::lock::view_model::LockViewModel;
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

impl LockViewModel {
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

    fn lock_key_card(&self, phrase: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let copy = {
            let p = phrase.to_string();
            PFButton::new("复制")
                .id("lk-copy")
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(p.clone()));
                }))
        };
        let clear = PFButton::new("从屏幕清除")
            .id("lk-clear")
            .on_click(cx.listener(|this, _, _, cx| this.clear_lock_key(cx)));

        Card::new()
            .title("锁定密钥")
            .description("仅显示一次——每次重新上电后均需使用它解锁")
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
                            .child("若丢失此密钥，只能通过 FIDO 恢复出厂设置来恢复使用，且会销毁当前身份。请离线妥善保管。"),
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

impl Render for LockViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const TITLE: &str = "锁定";
        const SUBTITLE: &str = "对存储中的 FIDO 种子进行软件锁定。";

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(TITLE, SUBTITLE, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let status = self.status;
        let locked = status.map(|s| s.locked).unwrap_or(false);
        let lock_key = self.lock_key.clone();
        let lock_key_card = lock_key.map(|p| self.lock_key_card(&p, cx));
        let theme = cx.theme();

        let refresh_btn = Button::new("lk-refresh")
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
        let enable_btn = Button::new("lk-enable")
            .label("启用锁定")
            .danger()
            .disabled(self.loading || locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_enable(window, cx)));
        let unlock_btn = PFButton::new("解锁")
            .id("lk-unlock")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading || !locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_unlock(window, cx)));
        let disable_btn = PFButton::new("禁用锁定")
            .id("lk-disable")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .disabled(self.loading || !locked)
            .on_click(cx.listener(|this, _, window, cx| this.open_disable(window, cx)));

        let status_card = {
            let body =
                match status {
                    Some(s) => {
                        let state = if s.locked {
                            if s.unlocked {
                                "已锁定，本次上电周期已解锁"
                            } else {
                                "已锁定——使用 FIDO 登录前必须解锁"
                            }
                        } else {
                            "未锁定（明文种子）"
                        };
                        let (dot, color) = if s.locked && !s.unlocked {
                            ("●", theme.danger)
                        } else if s.locked {
                            ("●", theme.green)
                        } else {
                            ("●", theme.muted_foreground)
                        };
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(div().text_color(color).child(dot))
                                    .child(div().text_sm().child(format!("状态：{state}"))),
                            )
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                format!("存在种子：{}", if s.has_seed { "yes" } else { "no" }),
                            ))
                            .into_any_element()
                    }
                    None => div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("正在读取锁定状态…")
                        .into_any_element(),
                };
            Card::new()
                .title("锁定状态")
                .description("检查存储中的种子是否受到加密保护")
                .icon(Icon::default().path("icons/lock.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let actions_card = Card::new()
            .title("操作")
            .description("启用、解锁或禁用存储锁定")
            .icon(Icon::default().path("icons/lock-open.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        "启用锁定",
                        "加密种子并显示新的锁定密钥（需要 PIN 和触摸确认）",
                        enable_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "解锁",
                        "在本次上电周期中加载种子（需锁定密钥）",
                        unlock_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "禁用锁定",
                        "恢复明文种子（需锁定密钥和 PIN）",
                        disable_btn,
                        theme,
                    )),
            );

        let content = v_flex()
            .gap_6()
            .child(status_card)
            .children(lock_key_card)
            .child(actions_card);

        PageView::build(TITLE, SUBTITLE, content, theme).into_any_element()
    }
}
