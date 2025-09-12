use inquire::ui::{Color, RenderConfig, Styled};

// TODO: MAKE FUNCTIONS FOR STYLING INTERACTIVE ELEMENTS
pub fn base_config(prefix: &'static str) -> RenderConfig<'static> {
    RenderConfig::default()
        .with_prompt_prefix(Styled::new(prefix))
        .with_answered_prompt_prefix(Styled::new("✓").with_fg(Color::LightGreen))
}
