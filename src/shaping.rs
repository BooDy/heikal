use crate::error::RssyError;
use unicode_bidi::BidiInfo;

#[derive(Clone, Copy, Debug, PartialEq)]
enum JoiningType {
    Right,
    Dual,
    None,
}

struct ArabicChar {
    isolated: char,
    initial: char,
    medial: char,
    final_form: char,
    joining: JoiningType,
}

const ARABIC_TABLE: &[(u32, char, char, char, char, JoiningType)] = &[
    (0x0621, 'ء', 'ء', 'ء', 'ء', JoiningType::None),
    (0x0622, 'آ', 'آ', 'آ', 'ﺂ', JoiningType::Right),
    (0x0623, 'أ', 'أ', 'أ', 'ﺄ', JoiningType::Right),
    (0x0624, 'ؤ', 'ؤ', 'ؤ', 'ﺆ', JoiningType::Right),
    (0x0625, 'إ', 'إ', 'إ', 'ﺈ', JoiningType::Right),
    (0x0626, 'ئ', 'ﺋ', 'ﺌ', 'ﺊ', JoiningType::Dual),
    (0x0627, 'ا', 'ا', 'ا', 'ﺎ', JoiningType::Right),
    (0x0628, 'ب', 'ﺑ', 'ﺒ', 'ﺐ', JoiningType::Dual),
    (0x0629, 'ة', 'ة', 'ة', 'ﺔ', JoiningType::Right),
    (0x062A, 'ت', 'ﺗ', 'ﺘ', 'ﺖ', JoiningType::Dual),
    (0x062B, 'ث', 'ﺛ', 'ﺜ', 'ﺚ', JoiningType::Dual),
    (0x062C, 'ج', 'ﺟ', 'ﺠ', 'ﺞ', JoiningType::Dual),
    (0x062D, 'ح', 'ﺣ', 'ﺤ', 'ﺢ', JoiningType::Dual),
    (0x062E, 'خ', 'ﺧ', 'ﺨ', 'ﺦ', JoiningType::Dual),
    (0x062F, 'د', 'د', 'د', 'ﺪ', JoiningType::Right),
    (0x0630, 'ذ', 'ذ', 'ذ', 'ﺬ', JoiningType::Right),
    (0x0631, 'ر', 'ر', 'ر', 'ﺮ', JoiningType::Right),
    (0x0632, 'ز', 'ز', 'ز', 'ﺰ', JoiningType::Right),
    (0x0633, 'س', 'ﺳ', 'ﺴ', 'ﺲ', JoiningType::Dual),
    (0x0634, 'ش', 'ﺷ', 'ﺸ', 'ﺶ', JoiningType::Dual),
    (0x0635, 'ص', 'ﺻ', 'ﺼ', 'ﺺ', JoiningType::Dual),
    (0x0636, 'ض', 'ﺿ', 'ﻀ', 'ﺾ', JoiningType::Dual),
    (0x0637, 'ط', 'ﻃ', 'ﻄ', 'ﻂ', JoiningType::Dual),
    (0x0638, 'ظ', 'ﻇ', 'ﻈ', 'ﻆ', JoiningType::Dual),
    (0x0639, 'ع', 'ﻋ', 'ﻌ', 'ﻊ', JoiningType::Dual),
    (0x063A, 'غ', 'ﻏ', 'ﻐ', 'ﻎ', JoiningType::Dual),
    (0x0641, 'ف', 'ﻓ', 'ﻔ', 'ﻒ', JoiningType::Dual),
    (0x0642, 'ق', 'ﻗ', 'ﻘ', 'ﻖ', JoiningType::Dual),
    (0x0643, 'ك', 'ﻛ', 'ﻜ', 'ﻚ', JoiningType::Dual),
    (0x0644, 'ل', 'ﻟ', 'ﻠ', 'ﻞ', JoiningType::Dual),
    (0x0645, 'م', 'ﻣ', 'ﻤ', 'ﻢ', JoiningType::Dual),
    (0x0646, 'ن', 'ﻧ', 'ﻨ', 'ﻦ', JoiningType::Dual),
    (0x0647, 'ه', 'ﻫ', 'ﻬ', 'ﻪ', JoiningType::Dual),
    (0x0648, 'و', 'و', 'و', 'ﻮ', JoiningType::Right),
    (0x0649, 'ى', 'ى', 'ى', 'ﻰ', JoiningType::Right),
    (0x064A, 'ي', 'ﻳ', 'ﻴ', 'ﻲ', JoiningType::Dual),
];

fn get_arabic_char(c: char) -> Option<ArabicChar> {
    ARABIC_TABLE
        .iter()
        .find(|&&(_, isolated, _, _, _, _)| isolated == c)
        .map(
            |&(_, isolated, initial, medial, final_form, joining)| ArabicChar {
                isolated,
                initial,
                medial,
                final_form,
                joining,
            },
        )
}

pub struct TextShaper {}

impl TextShaper {
    pub fn new() -> Result<Self, RssyError> {
        Ok(Self {})
    }

    pub fn reshape(&self, input: &str) -> String {
        let mut chars: Vec<char> = input.chars().collect();

        // 1. Handle Lam-Alef ligatures
        let mut i = 0;
        while i + 1 < chars.len() {
            if chars[i] == 'ل' {
                let ligature = match chars[i + 1] {
                    'آ' => Some('ﻵ'),
                    'أ' => Some('ﻷ'),
                    'إ' => Some('ﻹ'),
                    'ا' => Some('ﻻ'),
                    _ => None,
                };
                if let Some(l) = ligature {
                    chars[i] = l;
                    chars.remove(i + 1);
                }
            }
            i += 1;
        }

        let mut result = Vec::with_capacity(chars.len());
        for i in 0..chars.len() {
            let c = chars[i];
            if let Some(ac) = get_arabic_char(c) {
                let prev = if i > 0 { Some(chars[i - 1]) } else { None };
                let next = if i < chars.len() - 1 {
                    Some(chars[i + 1])
                } else {
                    None
                };

                let connects_prev = prev
                    .and_then(get_arabic_char)
                    .map(|p| p.joining == JoiningType::Dual)
                    .unwrap_or(false);
                let connects_next = next
                    .and_then(get_arabic_char)
                    .map(|n| n.joining != JoiningType::None)
                    .unwrap_or(false);

                let shaped = match (connects_prev, connects_next) {
                    (true, true) if ac.joining == JoiningType::Dual => ac.medial,
                    (true, false) => ac.final_form,
                    (false, true) if ac.joining == JoiningType::Dual => ac.initial,
                    _ => ac.isolated,
                };
                result.push(shaped);
            } else {
                result.push(c);
            }
        }
        result.into_iter().collect()
    }
}

pub fn shape_text(shaper: Option<&TextShaper>, input: &str, width: usize) -> Vec<String> {
    let mut all_lines = Vec::new();

    for paragraph in input.lines() {
        if paragraph.trim().is_empty() {
            all_lines.push(String::new());
            continue;
        }

        // Reshape first to ensure connections
        let reshaped_para = if let Some(s) = shaper {
            s.reshape(paragraph)
        } else {
            paragraph.to_string()
        };

        let mut current_line = String::new();
        let mut current_width = 0;

        for word in reshaped_para.split_whitespace() {
            let word_width = word.chars().count(); // Approximation

            if current_width + word_width + (if current_line.is_empty() { 0 } else { 1 }) > width {
                if !current_line.is_empty() {
                    all_lines.push(finalize_line(&current_line, width));
                    current_line = String::new();
                    current_width = 0;
                }

                if word_width > width {
                    all_lines.push(finalize_line(word, width));
                    continue;
                }
            }

            if !current_line.is_empty() {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        }

        if !current_line.is_empty() {
            all_lines.push(finalize_line(&current_line, width));
        }
    }

    all_lines
}

fn finalize_line(line_text: &str, width: usize) -> String {
    // 1. BiDi reorder
    let bidi_info = BidiInfo::new(line_text, None);
    let mut visual_line = String::new();
    if !bidi_info.paragraphs.is_empty() {
        for paragraph in &bidi_info.paragraphs {
            let (_levels, runs) = bidi_info.visual_runs(paragraph, paragraph.range.clone());
            for run in runs {
                visual_line.push_str(&line_text[run]);
            }
        }
    } else {
        visual_line = line_text.to_string();
    }

    // 2. Alignment
    if is_mostly_rtl(line_text) {
        let current_width = visual_line.chars().count();
        let padding = width.saturating_sub(current_width);
        format!("{}{}", " ".repeat(padding), visual_line)
    } else {
        visual_line
    }
}

fn is_mostly_rtl(text: &str) -> bool {
    let rtl_count = text
        .chars()
        .filter(|&c| {
            let bidi_class = unicode_bidi::bidi_class(c);
            matches!(
                bidi_class,
                unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL
            )
        })
        .count();
    rtl_count * 2 > text.chars().count()
}

pub fn shape_preformatted_text(
    shaper: Option<&TextShaper>,
    input: &str,
    width: usize,
) -> Vec<String> {
    let mut all_lines = Vec::new();
    for line in input.lines() {
        let reshaped = if let Some(s) = shaper {
            s.reshape(line)
        } else {
            line.to_string()
        };
        all_lines.push(finalize_line(&reshaped, width));
    }
    all_lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reshaping() {
        let shaper = TextShaper::new().unwrap();
        let input = "مرحبا";
        let reshaped = shaper.reshape(input);
        assert_ne!(input, reshaped);
    }

    #[test]
    fn test_reshaping_empty() {
        let shaper = TextShaper::new().unwrap();
        let input = "";
        let reshaped = shaper.reshape(input);
        assert_eq!(reshaped, "");
    }
}
