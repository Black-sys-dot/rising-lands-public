// Included in memory's module; undo storage and instrumentation are scratch only.
impl MemoriaJogo {
    pub(crate) fn save_boundary(&self) -> bool {
        !self.journal.active && self.player.is_none()
    }
}
