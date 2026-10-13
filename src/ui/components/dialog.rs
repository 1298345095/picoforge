//! Modal dialog components for PIN prompts, confirmations, and status display.

use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable, Sizable, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};

type PinPromptCallback = std::rc::Rc<dyn Fn(String, WeakEntity<PinPromptContent>, &mut App)>;
type ConfirmCallback = std::rc::Rc<dyn Fn(WeakEntity<ConfirmContent>, &mut Window, &mut App)>;
type ChangePinCallback =
    std::rc::Rc<dyn Fn(String, String, WeakEntity<ChangePinContent>, &mut App)>;
type SetPinCallback = std::rc::Rc<dyn Fn(String, WeakEntity<SetPinContent>, &mut App)>;

#[derive(Clone)]
enum DialogPhase {
    Input,
    Loading,
    /// Indicates the dialog is blocked on an asynchronous background task,
    /// presenting a specific dynamic status message to guide the user (e.g. "Waiting for touch...").
    LoadingWithMessage(String),
    Success(String),
    Error(String),
}

/// Dialog content for collecting the FIDO PIN from the user.
pub struct PinPromptContent {
    phase: DialogPhase,
    title: SharedString,
    description: SharedString,
    warning: Option<SharedString>,
    confirm_label: SharedString,
    pin_input: Entity<InputState>,
    on_confirm: PinPromptCallback,
    _subscription: Subscription,
}

impl PinPromptContent {
    /// Transition the dialog to a loading state with the given message.
    pub fn set_loading_msg(&mut self, msg: impl Into<String>, cx: &mut Context<Self>) {
        self.phase = DialogPhase::LoadingWithMessage(msg.into());
        cx.notify();
    }

    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    /// Transition the dialog to a success state.
    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    /// Transition the dialog to an error state with the given message.
    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }

    fn trigger_confirm(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, DialogPhase::Loading | DialogPhase::Success(_)) {
            return;
        }
        let pin = self.pin_input.read(cx).text().to_string();
        if !pin.is_empty() {
            let handle = cx.entity().downgrade();
            self.set_loading(cx);
            (self.on_confirm)(pin, handle, cx);
        }
    }
}

impl Render for PinPromptContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("完成")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading | DialogPhase::LoadingWithMessage(_) => {
                let text = match &self.phase {
                    DialogPhase::LoadingWithMessage(msg) => msg.clone(),
                    _ => self.description.to_string(),
                };
                v_flex()
                    .gap_4()
                    .child(text)
                    .child(Input::new(&self.pin_input).disabled(true))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("cancel").label("取消").disabled(true))
                            .child(
                                Button::new("confirm")
                                    .primary()
                                    .label("加载中…")
                                    .loading(true),
                            ),
                    )
                    .into_any_element()
            }

            DialogPhase::Error(err_msg) => {
                let pin_input = self.pin_input.clone();
                let confirm_label = self.confirm_label.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                let mut container = v_flex().gap_4().child(self.description.clone());

                if let Some(warning) = &self.warning {
                    container = container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .items_start()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .child(
                                div().flex_shrink_0().child(
                                    gpui_component::Icon::new(
                                        gpui_component::IconName::TriangleAlert,
                                    )
                                    .text_color(rgb(0xef4444))
                                    .with_size(gpui_component::Size::Medium),
                                ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(rgb(0xef4444))
                                    .text_sm()
                                    .child(warning.clone()),
                            ),
                    );
                }

                container
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .text_color(rgb(0xef4444))
                            .text_sm()
                            .child(render_error_message(err_msg.clone())),
                    )
                    .child(Input::new(&pin_input))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| {
                                        window.close_dialog(cx);
                                    }),
                            )
                            .child(
                                Button::new("confirm")
                                    .primary()
                                    .label(confirm_label)
                                    .on_click(move |_, _, cx| {
                                        let pin = pin_input.read(cx).text().to_string();
                                        if !pin.is_empty() {
                                            if let Some(h) = handle.upgrade() {
                                                h.update(cx, |this, cx| this.set_loading(cx));
                                            }
                                            on_confirm(pin, handle.clone(), cx);
                                        }
                                    }),
                            ),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let pin_input = self.pin_input.clone();
                let confirm_label = self.confirm_label.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                let mut container = v_flex().gap_4().child(self.description.clone());

                if let Some(warning) = &self.warning {
                    container = container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .items_start()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .child(
                                div().flex_shrink_0().child(
                                    gpui_component::Icon::new(
                                        gpui_component::IconName::TriangleAlert,
                                    )
                                    .text_color(rgb(0xef4444))
                                    .with_size(gpui_component::Size::Medium),
                                ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(rgb(0xef4444))
                                    .text_sm()
                                    .child(warning.clone()),
                            ),
                    );
                }

                container
                    .child(Input::new(&pin_input))
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| {
                                        window.close_dialog(cx);
                                    }),
                            )
                            .child(
                                Button::new("confirm")
                                    .primary()
                                    .label(confirm_label)
                                    .on_click(move |_, _, cx| {
                                        let pin = pin_input.read(cx).text().to_string();
                                        if !pin.is_empty() {
                                            if let Some(h) = handle.upgrade() {
                                                h.update(cx, |this, cx| this.set_loading(cx));
                                            }
                                            on_confirm(pin, handle.clone(), cx);
                                        }
                                    }),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}

/// Open a PIN prompt dialog and return the submitted PIN.
#[allow(clippy::too_many_arguments)]
pub fn open_pin_prompt(
    title: &str,
    description: &str,
    placeholder: &str,
    warning: Option<&str>,
    confirm_label: &str,
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(String, WeakEntity<PinPromptContent>, &mut App) + 'static,
) {
    let title_str = SharedString::from(title.to_string());
    let description = SharedString::from(description.to_string());
    let placeholder_str = SharedString::from(placeholder.to_string());
    let warning = warning.map(|w| SharedString::from(w.to_string()));
    let confirm_label = SharedString::from(confirm_label.to_string());

    let pin_input = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(placeholder_str)
            .masked(true)
    });

    let dialog_title = title_str.clone();
    let pin_for_sub = pin_input.clone();

    let content = cx.new(|cx| {
        let sub = cx.subscribe(&pin_for_sub, |this: &mut PinPromptContent, _, event, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.trigger_confirm(cx);
            }
        });

        PinPromptContent {
            phase: DialogPhase::Input,
            title: title_str,
            description,
            warning,
            confirm_label,
            pin_input: pin_for_sub,
            on_confirm: std::rc::Rc::new(on_confirm),
            _subscription: sub,
        }
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(dialog_title.clone())
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}

/// Dialog content for confirming a destructive or irreversible action.
pub struct ConfirmContent {
    phase: DialogPhase,
    title: SharedString,
    message: String,
    ok_label: SharedString,
    ok_variant: ButtonVariant,
    on_ok: ConfirmCallback,
}

impl ConfirmContent {
    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    /// Transition the dialog to a success state.
    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    /// Transition the dialog to an error state with the given message.
    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }
}

impl Render for ConfirmContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("完成")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading | DialogPhase::LoadingWithMessage(_) => v_flex()
                .gap_4()
                .child(self.message.clone())
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("取消").disabled(true))
                        .child(
                            Button::new("ok")
                                .with_variant(self.ok_variant)
                                .label("加载中…")
                                .loading(true),
                        ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                let ok_label = self.ok_label.clone();
                let ok_variant = self.ok_variant;
                let on_ok = self.on_ok.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child(self.message.clone())
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .text_color(rgb(0xef4444))
                            .text_sm()
                            .child(render_error_message(err_msg.clone())),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| {
                                        window.close_dialog(cx);
                                    }),
                            )
                            .child(
                                Button::new("ok")
                                    .with_variant(ok_variant)
                                    .label(ok_label)
                                    .on_click(move |_, window, cx| {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| this.set_loading(cx));
                                        }
                                        on_ok(handle.clone(), window, cx);
                                    }),
                            ),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let ok_label = self.ok_label.clone();
                let ok_variant = self.ok_variant;
                let on_ok = self.on_ok.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child(self.message.clone())
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| {
                                        window.close_dialog(cx);
                                    }),
                            )
                            .child(
                                Button::new("ok")
                                    .with_variant(ok_variant)
                                    .label(ok_label)
                                    .on_click(move |_, window, cx| {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| this.set_loading(cx));
                                        }
                                        on_ok(handle.clone(), window, cx);
                                    }),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}

/// Open a confirmation dialog and return whether the user accepted.
pub fn open_confirm(
    title: &str,
    message: String,
    ok_label: &str,
    ok_variant: ButtonVariant,
    window: &mut Window,
    cx: &mut App,
    on_ok: impl Fn(WeakEntity<ConfirmContent>, &mut Window, &mut App) + 'static,
) {
    let title_str = SharedString::from(title.to_string());
    let dialog_title = title_str.clone();

    let content = cx.new(|_cx| ConfirmContent {
        phase: DialogPhase::Input,
        title: title_str,
        message,
        ok_label: SharedString::from(ok_label.to_string()),
        ok_variant,
        on_ok: std::rc::Rc::new(on_ok),
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(dialog_title.clone())
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}

/// Dialog content for changing an existing FIDO PIN.
pub struct ChangePinContent {
    phase: DialogPhase,
    current_pin: Entity<InputState>,
    new_pin: Entity<InputState>,
    confirm_pin: Entity<InputState>,
    on_confirm: ChangePinCallback,
    _subscriptions: Vec<Subscription>,
}

impl ChangePinContent {
    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    /// Transition the dialog to a success state.
    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    /// Transition the dialog to an error state with the given message.
    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }

    fn trigger_confirm(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, DialogPhase::Loading | DialogPhase::Success(_)) {
            return;
        }

        let current_pin_text = self.current_pin.read(cx).text().to_string();
        let new_pin_text = self.new_pin.read(cx).text().to_string();
        let confirm_pin_text = self.confirm_pin.read(cx).text().to_string();

        if current_pin_text.is_empty() {
            return;
        }

        if new_pin_text != confirm_pin_text {
            self.set_error("两次输入的 PIN 不一致".to_string(), cx);
            return;
        }

        if new_pin_text.len() < 4 {
            self.set_error("PIN 至少需要 4 个字符".to_string(), cx);
            return;
        }

        let handle = cx.entity().downgrade();
        self.set_loading(cx);
        (self.on_confirm)(current_pin_text, new_pin_text, handle, cx);
    }
}

impl Render for ChangePinContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child("修改 PIN"),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("完成")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading | DialogPhase::LoadingWithMessage(_) => v_flex()
                .gap_4()
                .child("请输入当前 PIN 并设置新的 PIN。")
                .child(
                    v_flex()
                        .gap_4()
                        .child("当前 PIN")
                        .child(Input::new(&self.current_pin).disabled(true))
                        .child("新 PIN")
                        .child(Input::new(&self.new_pin).disabled(true))
                        .child("确认新 PIN")
                        .child(Input::new(&self.confirm_pin).disabled(true)),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("取消").disabled(true))
                        .child(
                            Button::new("confirm")
                                .primary()
                                .label("正在修改 PIN…")
                                .loading(true),
                        ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                let current_pin_entity = self.current_pin.clone();
                let new_pin_entity = self.new_pin.clone();
                let confirm_pin_entity = self.confirm_pin.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child("请输入当前 PIN 并设置新的 PIN。")
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .text_color(rgb(0xef4444))
                            .text_sm()
                            .child(render_error_message(err_msg.clone())),
                    )
                    .child(
                        v_flex()
                            .gap_4()
                            .child("当前 PIN")
                            .child(Input::new(&current_pin_entity))
                            .child("新 PIN")
                            .child(Input::new(&new_pin_entity))
                            .child("确认新 PIN")
                            .child(Input::new(&confirm_pin_entity)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child(Button::new("confirm").primary().label("确认").on_click(
                                move |_, _, cx| {
                                    let current_pin_text =
                                        current_pin_entity.read(cx).text().to_string();
                                    let new_pin_text = new_pin_entity.read(cx).text().to_string();
                                    let confirm_pin_text =
                                        confirm_pin_entity.read(cx).text().to_string();

                                    if current_pin_text.is_empty() {
                                        return;
                                    }

                                    if new_pin_text != confirm_pin_text {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "两次输入的 PIN 不一致".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if new_pin_text.len() < 4 {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "PIN 至少需要 4 个字符".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if let Some(h) = handle.upgrade() {
                                        h.update(cx, |this, cx| this.set_loading(cx));
                                    }
                                    on_confirm(current_pin_text, new_pin_text, handle.clone(), cx);
                                },
                            )),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let current_pin_entity = self.current_pin.clone();
                let new_pin_entity = self.new_pin.clone();
                let confirm_pin_entity = self.confirm_pin.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child("请输入当前 PIN 并设置新的 PIN。")
                    .child(
                        v_flex()
                            .gap_4()
                            .child("当前 PIN")
                            .child(Input::new(&current_pin_entity))
                            .child("新 PIN")
                            .child(Input::new(&new_pin_entity))
                            .child("确认新 PIN")
                            .child(Input::new(&confirm_pin_entity)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child(Button::new("confirm").primary().label("确认").on_click(
                                move |_, _, cx| {
                                    let current_pin_text =
                                        current_pin_entity.read(cx).text().to_string();
                                    let new_pin_text = new_pin_entity.read(cx).text().to_string();
                                    let confirm_pin_text =
                                        confirm_pin_entity.read(cx).text().to_string();

                                    if current_pin_text.is_empty() {
                                        return;
                                    }

                                    if new_pin_text != confirm_pin_text {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "两次输入的 PIN 不一致".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if new_pin_text.len() < 4 {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "PIN 至少需要 4 个字符".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if let Some(h) = handle.upgrade() {
                                        h.update(cx, |this, cx| this.set_loading(cx));
                                    }
                                    on_confirm(current_pin_text, new_pin_text, handle.clone(), cx);
                                },
                            )),
                    )
                    .into_any_element()
            }
        }
    }
}

/// Open a dialog to change the FIDO PIN.
pub fn open_change_pin(
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(String, String, WeakEntity<ChangePinContent>, &mut App) + 'static,
) {
    let current_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("输入当前 PIN")
            .masked(true)
    });
    let new_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("输入新 PIN")
            .masked(true)
    });
    let confirm_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("确认新 PIN")
            .masked(true)
    });

    let confirm_for_sub = confirm_pin.clone();

    let content = cx.new(|cx| {
        let sub = cx.subscribe(
            &confirm_for_sub,
            |this: &mut ChangePinContent, _, event, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.trigger_confirm(cx);
                }
            },
        );

        ChangePinContent {
            phase: DialogPhase::Input,
            current_pin,
            new_pin,
            confirm_pin: confirm_for_sub,
            on_confirm: std::rc::Rc::new(on_confirm),
            _subscriptions: vec![sub],
        }
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("修改 PIN")
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}

/// Dialog content for setting an initial FIDO PIN.
pub struct SetPinContent {
    phase: DialogPhase,
    new_pin: Entity<InputState>,
    confirm_pin: Entity<InputState>,
    on_confirm: SetPinCallback,
    _subscriptions: Vec<Subscription>,
}

impl SetPinContent {
    fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Loading;
        cx.notify();
    }

    /// Transition the dialog to a success state.
    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    /// Transition the dialog to an error state with the given message.
    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }

    fn trigger_confirm(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, DialogPhase::Loading | DialogPhase::Success(_)) {
            return;
        }

        let new_pin_text = self.new_pin.read(cx).text().to_string();
        let confirm_pin_text = self.confirm_pin.read(cx).text().to_string();

        if new_pin_text != confirm_pin_text {
            self.set_error("两次输入的 PIN 不一致".to_string(), cx);
            return;
        }

        if new_pin_text.len() < 4 {
            self.set_error("PIN 至少需要 4 个字符".to_string(), cx);
            return;
        }

        let handle = cx.entity().downgrade();
        self.set_loading(cx);
        (self.on_confirm)(new_pin_text, handle, cx);
    }
}

impl Render for SetPinContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child("设置 PIN"),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("完成")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Loading | DialogPhase::LoadingWithMessage(_) => v_flex()
                .gap_4()
                .child("请为 pico-key 设置 PIN。")
                .child(
                    v_flex()
                        .gap_4()
                        .child("新 PIN")
                        .child(Input::new(&self.new_pin).disabled(true))
                        .child("确认新 PIN")
                        .child(Input::new(&self.confirm_pin).disabled(true)),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("取消").disabled(true))
                        .child(
                            Button::new("confirm")
                                .primary()
                                .label("正在设置 PIN…")
                                .loading(true),
                        ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => {
                let new_pin_entity = self.new_pin.clone();
                let confirm_pin_entity = self.confirm_pin.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child("请为 pico-key 设置 PIN。")
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x18181b))
                            .text_color(rgb(0xef4444))
                            .text_sm()
                            .child(render_error_message(err_msg.clone())),
                    )
                    .child(
                        v_flex()
                            .gap_4()
                            .child("新 PIN")
                            .child(Input::new(&new_pin_entity))
                            .child("确认新 PIN")
                            .child(Input::new(&confirm_pin_entity)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child(Button::new("confirm").primary().label("确认").on_click(
                                move |_, _, cx| {
                                    let new_pin_text = new_pin_entity.read(cx).text().to_string();
                                    let confirm_pin_text =
                                        confirm_pin_entity.read(cx).text().to_string();

                                    if new_pin_text != confirm_pin_text {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "两次输入的 PIN 不一致".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if new_pin_text.len() < 4 {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "PIN 至少需要 4 个字符".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if let Some(h) = handle.upgrade() {
                                        h.update(cx, |this, cx| this.set_loading(cx));
                                    }
                                    on_confirm(new_pin_text, handle.clone(), cx);
                                },
                            )),
                    )
                    .into_any_element()
            }

            DialogPhase::Input => {
                let new_pin_entity = self.new_pin.clone();
                let confirm_pin_entity = self.confirm_pin.clone();
                let on_confirm = self.on_confirm.clone();
                let handle = cx.entity().downgrade();

                v_flex()
                    .gap_4()
                    .child("请为 pico-key 设置 PIN。")
                    .child(
                        v_flex()
                            .gap_4()
                            .child("新 PIN")
                            .child(Input::new(&new_pin_entity))
                            .child("确认新 PIN")
                            .child(Input::new(&confirm_pin_entity)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel")
                                    .label("取消")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            )
                            .child(Button::new("confirm").primary().label("确认").on_click(
                                move |_, _, cx| {
                                    let new_pin_text = new_pin_entity.read(cx).text().to_string();
                                    let confirm_pin_text =
                                        confirm_pin_entity.read(cx).text().to_string();

                                    if new_pin_text != confirm_pin_text {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "两次输入的 PIN 不一致".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if new_pin_text.len() < 4 {
                                        if let Some(h) = handle.upgrade() {
                                            h.update(cx, |this, cx| {
                                                this.set_error(
                                                    "PIN 至少需要 4 个字符".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                        return;
                                    }

                                    if let Some(h) = handle.upgrade() {
                                        h.update(cx, |this, cx| this.set_loading(cx));
                                    }
                                    on_confirm(new_pin_text, handle.clone(), cx);
                                },
                            )),
                    )
                    .into_any_element()
            }
        }
    }
}

/// Open a dialog to set an initial FIDO PIN.
pub fn open_setup_pin(
    window: &mut Window,
    cx: &mut App,
    on_confirm: impl Fn(String, WeakEntity<SetPinContent>, &mut App) + 'static,
) {
    let new_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("输入新 PIN")
            .masked(true)
    });
    let confirm_pin = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("确认新 PIN")
            .masked(true)
    });

    let confirm_for_sub = confirm_pin.clone();

    let content = cx.new(|cx| {
        let sub = cx.subscribe(
            &confirm_for_sub,
            |this: &mut SetPinContent, _, event, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.trigger_confirm(cx);
                }
            },
        );

        SetPinContent {
            phase: DialogPhase::Input,
            new_pin,
            confirm_pin: confirm_for_sub,
            on_confirm: std::rc::Rc::new(on_confirm),
            _subscriptions: vec![sub],
        }
    });

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("设置 PIN")
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });
}
/// Dialog content for showing operation progress, success, or error.
pub struct StatusContent {
    phase: DialogPhase,
    title: SharedString,
}

impl StatusContent {
    /// Transitions the dialog into a loading state while displaying a custom, dynamic status message.
    /// Useful for multi-step background operations where user context needs to be updated.
    pub fn set_loading(&mut self, msg: impl Into<String>, cx: &mut Context<Self>) {
        self.phase = DialogPhase::LoadingWithMessage(msg.into());
        cx.notify();
    }

    pub fn set_success(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Success(msg);
        cx.notify();
    }

    pub fn set_error(&mut self, msg: String, cx: &mut Context<Self>) {
        self.phase = DialogPhase::Error(msg);
        cx.notify();
    }
}

impl Render for StatusContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let phase = self.phase.clone();

        match &phase {
            DialogPhase::Success(msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleCheck)
                                .text_color(cx.theme().green)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(msg.clone())
                .child(
                    h_flex().justify_end().child(
                        Button::new("done")
                            .primary()
                            .label("完成")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
                )
                .into_any_element(),

            DialogPhase::Error(err_msg) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            gpui_component::Icon::new(gpui_component::IconName::CircleX)
                                .text_color(cx.theme().danger)
                                .with_size(gpui_component::Size::Large),
                        )
                        .child(self.title.clone()),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(0x18181b))
                        .text_color(rgb(0xef4444))
                        .text_sm()
                        .child(render_error_message(err_msg.clone())),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .child(
                            Button::new("close")
                                .label("关闭")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        ),
                )
                .into_any_element(),

            DialogPhase::LoadingWithMessage(msg) => v_flex()
                .gap_4()
                .items_center()
                .child(msg.clone())
                .child(
                    Button::new("loading")
                        .primary()
                        .label("正在应用…")
                        .loading(true),
                )
                .into_any_element(),

            _ => v_flex()
                .gap_4()
                .items_center()
                .child("正在应用配置…")
                .child(
                    Button::new("loading")
                        .primary()
                        .label("正在应用…")
                        .loading(true),
                )
                .into_any_element(),
        }
    }
}

/// Open a status dialog showing progress, success, or error state.
pub fn open_status_dialog(
    title: &str,
    window: &mut Window,
    cx: &mut App,
) -> WeakEntity<StatusContent> {
    let title_str = SharedString::from(title.to_string());
    let dialog_title = title_str.clone();

    let content = cx.new(|_cx| StatusContent {
        phase: DialogPhase::Loading,
        title: title_str,
    });

    let handle = content.downgrade();

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title(dialog_title.clone())
            .child(content.clone())
            .overlay_closable(false)
            .close_button(false)
    });

    handle
}

fn render_error_message(msg: String) -> impl IntoElement {
    let troubleshooting_phrase = "故障排查指南";
    let url = "https://github.com/librekeys/picoforge/wiki/Troubleshooting#1-my-key-is-not-detected-by-picoforge-or-picoforge-displays-a-device-status-of-online---fido-and-there-are-some-settings-that-i-cannot-configure";

    if msg.contains(troubleshooting_phrase) {
        v_flex()
            .child("设备固件不支持在仅 FIDO 通信模式下进行配置。")
            .child(
                h_flex()
                    .gap_1()
                    .child("请参阅")
                    .child(
                        div()
                            .text_color(rgb(0x3b82f6))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.open_url(url);
                            })
                            .child(troubleshooting_phrase.to_string()),
                    )
                    .child("解决此问题"),
            )
    } else {
        div().child(msg)
    }
}
