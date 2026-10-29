use crate::text::Text;
pub fn label_without_ellipsis(label: &str) -> &str {
    label
        .strip_suffix("...")
        .or_else(|| label.strip_suffix('…'))
        .unwrap_or(label)
}

/// Remove navigation punctuation without losing coloured/styled label spans.
pub fn button_label(label: &Text) -> Text {
    let plain = label.plain();
    let clean = label_without_ellipsis(&plain);
    if clean.len() == plain.len() {
        return label.clone();
    }
    let mut remaining = clean.chars().count();
    let mut result = Text::empty();
    for span in &label.resolved().spans {
        let piece: String = span.text.chars().take(remaining).collect();
        remaining -= piece.chars().count();
        result.push(Text::styled(piece, span.style.clone()));
        if remaining == 0 {
            break;
        }
    }
    result
}
