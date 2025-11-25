use gpui::{Context, FocusHandle};
use settings::SettingsStore;
use smol::Timer;
use std::time::Duration;
use ui::{App, Window};

pub struct BlinkManager {
    blink_interval: Duration,
    blink_epoch: usize,
    /// Whether the blinking is paused.
    blinking_paused: bool,
    /// Whether the cursor should be visibly rendered or not.
    visible: bool,
    /// The focus handle to use to determine if the cursor should be blinking.
    focus_handle: FocusHandle,
    /// Whether the blinking is enabled in the settings.
    is_enabled: Box<dyn Fn(&App) -> bool>,
}

impl BlinkManager {
    pub fn new(
        blink_interval: Duration,
        focus_handle: FocusHandle,
        is_enabled: impl Fn(&App) -> bool + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Make sure we blink the cursors if the setting is re-enabled
        cx.observe_global_in::<SettingsStore>(window, move |this, window, cx| {
            this.refresh(window, cx);
        })
        .detach();

        cx.on_focus(&focus_handle, window, move |this, window, cx| {
            this.visible = false;
            this.refresh(window, cx);
        })
        .detach();

        cx.on_blur(&focus_handle, window, move |this, _window, _cx| {
            this.visible = false;
        })
        .detach();

        Self {
            blink_interval,
            blink_epoch: 0,
            blinking_paused: false,
            visible: true,
            focus_handle,
            is_enabled: Box::new(is_enabled),
        }
    }

    pub fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.blink_cursors(self.blink_epoch, window, cx)
    }

    fn next_blink_epoch(&mut self) -> usize {
        self.blink_epoch += 1;
        self.blink_epoch
    }

    pub fn pause_blinking(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_cursor(cx);

        let epoch = self.next_blink_epoch();
        let interval = self.blink_interval;
        cx.spawn_in(window, async move |this, cx| {
            Timer::after(interval).await;
            this.update_in(cx, |this, window, cx| {
                this.resume_cursor_blinking(epoch, window, cx)
            })
        })
        .detach();
    }

    fn resume_cursor_blinking(
        &mut self,
        epoch: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if epoch == self.blink_epoch {
            self.blinking_paused = false;
            self.blink_cursors(epoch, window, cx);
        }
    }

    fn blink_cursors(&mut self, epoch: usize, window: &mut Window, cx: &mut Context<Self>) {
        if (self.is_enabled)(cx) {
            if epoch == self.blink_epoch
                && self.focus_handle.is_focused(window)
                && !self.blinking_paused
            {
                self.visible = !self.visible;
                cx.notify();

                let epoch = self.next_blink_epoch();
                let interval = self.blink_interval;
                cx.spawn_in(window, async move |this, cx| {
                    Timer::after(interval).await;
                    if let Some(this) = this.upgrade() {
                        this.update_in(cx, |this, window, cx| {
                            this.blink_cursors(epoch, window, cx)
                        })
                        .ok();
                    }
                })
                .detach();
            }
        } else {
            self.show_cursor(cx);
        }
    }

    pub fn show_cursor(&mut self, cx: &mut Context<BlinkManager>) {
        if !self.visible {
            self.visible = true;
            cx.notify();
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }
}
