use regex::Regex;

use crate::model::Calendar;

#[derive(Debug, Clone)]
struct LevelPatternMatcher {
    name: String,
    identifier_pattern: Option<String>,
    compiled: Option<Result<Regex, String>>,
}

#[derive(Debug, Clone)]
pub struct CalendarMatcher {
    levels: Vec<LevelPatternMatcher>,
}

impl CalendarMatcher {
    pub fn new(calendar: &Calendar) -> Self {
        let levels = calendar
            .levels
            .iter()
            .map(|level| LevelPatternMatcher {
                name: level.name.clone(),
                identifier_pattern: level.identifier_pattern.clone(),
                compiled: None,
            })
            .collect();

        Self { levels }
    }

    pub fn find_level_strict(&mut self, identifier: &str) -> Result<Option<String>, String> {
        for level in &mut self.levels {
            let Some(regex_result) = level.compile() else {
                continue;
            };

            let regex = regex_result.as_ref().map_err(|error| error.clone())?;
            if regex.is_match(identifier) {
                return Ok(Some(level.name.clone()));
            }
        }

        Ok(None)
    }

    pub fn find_level_lenient(&mut self, identifier: &str) -> Option<String> {
        for level in &mut self.levels {
            let Some(regex_result) = level.compile() else {
                continue;
            };

            if let Ok(regex) = regex_result {
                if regex.is_match(identifier) {
                    return Some(level.name.clone());
                }
            }
        }

        None
    }
}

impl LevelPatternMatcher {
    fn compile(&mut self) -> Option<&Result<Regex, String>> {
        let pattern = self.identifier_pattern.as_ref()?;

        if self.compiled.is_none() {
            self.compiled = Some(Regex::new(pattern).map_err(|error| {
                format!(
                    "invalid identifier_pattern for level '{}': {}",
                    self.name, error
                )
            }));
        }

        self.compiled.as_ref()
    }
}
