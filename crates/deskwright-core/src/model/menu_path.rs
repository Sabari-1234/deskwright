use std::fmt;

use crate::error::{DeskwrightError, DeskwrightResult};

const MENU_PATH_SEPARATOR: &str = ">";
const DISPLAY_SEPARATOR: &str = " > ";

/// A menu-bar path such as `File > Export > PDF…`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuPath(Vec<String>);

impl MenuPath {
    pub fn parse(raw: &str) -> DeskwrightResult<Self> {
        let items: Vec<String> = raw
            .split(MENU_PATH_SEPARATOR)
            .map(|item| item.trim().to_owned())
            .collect();
        if items.iter().any(String::is_empty) {
            return Err(DeskwrightError::InvalidInput(format!(
                "'{raw}' is not a menu path; use the form 'File > Save As…'"
            )));
        }
        Ok(Self(items))
    }

    pub fn items(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Display for MenuPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.join(DISPLAY_SEPARATOR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_trims_items() {
        let path = MenuPath::parse("File >Export> PDF…").unwrap();
        assert_eq!(path.items(), ["File", "Export", "PDF…"]);
        assert_eq!(path.to_string(), "File > Export > PDF…");
    }

    #[test]
    fn rejects_empty_items() {
        assert!(MenuPath::parse("").is_err());
        assert!(MenuPath::parse("File > > Save").is_err());
    }
}
