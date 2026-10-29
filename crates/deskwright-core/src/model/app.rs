use serde::Serialize;

use crate::model::visible_text::visible_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ProcessId(pub i32);

/// A running application as seen by the desktop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    pub process_id: ProcessId,
    pub is_active: bool,
}

impl AppInfo {
    /// True when `name_or_bundle_id` equals this app's display name or bundle ID, ignoring case
    /// and invisible direction marks (WhatsApp reports its name as "\u{200e}WhatsApp").
    pub fn is_identified_by(&self, name_or_bundle_id: &str) -> bool {
        visible_text(&self.name).eq_ignore_ascii_case(&visible_text(name_or_bundle_id))
            || self
                .bundle_id
                .as_deref()
                .is_some_and(|bundle_id| bundle_id.eq_ignore_ascii_case(name_or_bundle_id))
    }
}

/// Which app an operation targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppSelector {
    Frontmost,
    NameOrBundleId(String),
}

impl AppSelector {
    /// A missing or blank name selects the frontmost app.
    pub fn from_optional_name(name_or_bundle_id: Option<&str>) -> Self {
        match name_or_bundle_id.map(str::trim) {
            Some(trimmed) if !trimmed.is_empty() => Self::NameOrBundleId(trimmed.to_owned()),
            _ => Self::Frontmost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, bundle_id: Option<&str>) -> AppInfo {
        AppInfo {
            name: name.to_owned(),
            bundle_id: bundle_id.map(str::to_owned),
            process_id: ProcessId(1),
            is_active: false,
        }
    }

    #[test]
    fn matches_name_or_bundle_id_ignoring_case_and_direction_marks() {
        let whatsapp = app("\u{200e}WhatsApp", Some("net.whatsapp.WhatsApp"));

        assert!(whatsapp.is_identified_by("whatsapp"));
        assert!(whatsapp.is_identified_by("NET.WHATSAPP.WHATSAPP"));
        assert!(!whatsapp.is_identified_by("WhatsApp Beta"));
    }

    #[test]
    fn a_blank_name_selects_the_frontmost_app() {
        assert_eq!(
            AppSelector::from_optional_name(Some("  ")),
            AppSelector::Frontmost
        );
        assert_eq!(
            AppSelector::from_optional_name(Some(" Notes ")),
            AppSelector::NameOrBundleId("Notes".to_owned())
        );
    }
}
