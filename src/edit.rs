use crate::store::Replacement;
use unicode_segmentation::UnicodeSegmentation;
#[derive(Debug, PartialEq, Eq)]
pub struct Patch {
    pub backspaces: usize,
    pub insert: String,
}
/// Changes only the text inserted by the current dictation session. The platform
/// must separately prove unchanged focus and no intervening physical input.
pub fn patch(old: &str, new: &str) -> Patch {
    let a = old.graphemes(true).collect::<Vec<_>>();
    let b = new.graphemes(true).collect::<Vec<_>>();
    let same = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    Patch {
        backspaces: a.len() - same,
        insert: b[same..].concat(),
    }
}
pub fn join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn dictionary(text: &str, rules: &[Replacement]) -> String {
    let mut rules = rules
        .iter()
        .filter(|r| !r.from.trim().is_empty())
        .collect::<Vec<_>>();
    rules.sort_by_key(|r| std::cmp::Reverse(r.from.len()));
    if rules.is_empty() {
        return text.into();
    }
    let pattern = rules
        .iter()
        .map(|r| regex::escape(&r.from))
        .collect::<Vec<_>>()
        .join("|");
    let Ok(re) = regex::RegexBuilder::new(&pattern)
        .case_insensitive(true)
        .build()
    else {
        return text.into();
    };
    let mut result = String::new();
    let mut pos = 0;
    for m in re.find_iter(text) {
        let s = m.as_str();
        let word = |c: char| c.is_alphanumeric() || c == '_';
        if s.chars().next().is_some_and(word) && text[..m.start()].chars().last().is_some_and(word)
        {
            continue;
        }
        if s.chars().last().is_some_and(word) && text[m.end()..].chars().next().is_some_and(word) {
            continue;
        }
        let Some(rule) = rules
            .iter()
            .find(|r| r.from.to_lowercase() == s.to_lowercase())
        else {
            continue;
        };
        result.push_str(&text[pos..m.start()]);
        result.push_str(&rule.to);
        pos = m.end();
    }
    result.push_str(&text[pos..]);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correction_and_unicode() {
        assert_eq!(
            patch("Hallo Welt", "Hallo Welt!"),
            Patch {
                backspaces: 0,
                insert: "!".into()
            }
        );
        assert_eq!(
            patch("Eine Katz", "Eine Katze."),
            Patch {
                backspaces: 0,
                insert: "e.".into()
            }
        );
        assert_eq!(
            patch("Hallo, Welt", "Hallo Welt!"),
            Patch {
                backspaces: 6,
                insert: " Welt!".into()
            }
        );
        assert_eq!(
            patch("a👩‍💻", "aö"),
            Patch {
                backspaces: 1,
                insert: "ö".into()
            }
        );
    }
    #[test]
    fn replacements_are_single_pass() {
        let r = vec![
            Replacement {
                from: "Voice Inc".into(),
                to: "Silbenlot".into(),
            },
            Replacement {
                from: "cat".into(),
                to: "dog".into(),
            },
            Replacement {
                from: "dog".into(),
                to: "wolf".into(),
            },
        ];
        assert_eq!(
            dictionary("VOICE INC: cat concatenate dog!", &r),
            "Silbenlot: dog concatenate wolf!"
        );
    }
}
