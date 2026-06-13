use egui::Rect;

#[derive(Default)]
pub struct FallbackEmbedder {
    enabled: bool,
    last_rect: Option<Rect>,
}

impl FallbackEmbedder {
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn sync_rect(&mut self, rect: Rect) {
        self.last_rect = Some(rect);
    }
}
