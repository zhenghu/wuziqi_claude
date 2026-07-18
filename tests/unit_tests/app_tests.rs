use super::super::app::compact_text;

#[test]
fn compact_text_preserves_short_model_ids_and_truncates_long_ones() {
    assert_eq!(compact_text("openai/gpt-5-mini", 24), "openai/gpt-5-mini");
    assert_eq!(
        compact_text("provider/a-very-long-model-name", 16),
        "provider/a-ve..."
    );
}
