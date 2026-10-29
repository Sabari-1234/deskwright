use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, AppSelector};
use crate::model::key_chord::KeyChord;
use crate::ports::InputSynthesizer;
use crate::services::app_resolver::AppResolver;

/// Sends keyboard input (key chords and typed text), but only when the intended app is
/// frontmost: keys always go to the frontmost app, so sending them while another app is in
/// front would act on the wrong app.
pub struct KeyboardService {
    apps: AppResolver,
    input: Arc<dyn InputSynthesizer>,
}

impl KeyboardService {
    pub fn new(apps: AppResolver, input: Arc<dyn InputSynthesizer>) -> Self {
        Self { apps, input }
    }

    pub fn press(&self, chord: &KeyChord, target: &AppSelector) -> DeskwrightResult<AppInfo> {
        let app = self.frontmost_target(target)?;
        self.input.press_chord(chord)?;
        Ok(app)
    }

    /// Types text into whatever has keyboard focus in the target app.
    pub fn type_text(&self, text: &str, target: &AppSelector) -> DeskwrightResult<AppInfo> {
        if text.is_empty() {
            return Err(DeskwrightError::InvalidInput(
                "there is no text to type".to_owned(),
            ));
        }
        let app = self.frontmost_target(target)?;
        self.input.type_text(text)?;
        Ok(app)
    }

    fn frontmost_target(&self, target: &AppSelector) -> DeskwrightResult<AppInfo> {
        let app = self.apps.resolve(target)?;
        if app.is_active {
            Ok(app)
        } else {
            Err(DeskwrightError::AppNotFrontmost(app.name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDesktop, RecordedAction};

    fn keyboard_service(desktop: &Arc<FakeDesktop>) -> KeyboardService {
        KeyboardService::new(AppResolver::new(desktop.clone()), desktop.clone())
    }

    #[test]
    fn presses_the_chord_when_the_target_app_is_frontmost() {
        let desktop = Arc::new(FakeDesktop::new());
        let chrome = desktop.add_app("Google Chrome", None);
        desktop.make_frontmost(chrome);
        let chord = KeyChord::parse("Enter").unwrap();
        let target = AppSelector::NameOrBundleId("google chrome".to_owned());

        let app = keyboard_service(&desktop).press(&chord, &target).unwrap();

        assert_eq!(app.name, "Google Chrome");
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::PressChord(chord)]
        );
    }

    #[test]
    fn refuses_when_the_target_app_is_in_the_background() {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.add_app("Google Chrome", None);
        let claude = desktop.add_app("Claude", None);
        desktop.make_frontmost(claude);
        let target = AppSelector::NameOrBundleId("Google Chrome".to_owned());

        let error = keyboard_service(&desktop)
            .press(&KeyChord::parse("Cmd+Q").unwrap(), &target)
            .unwrap_err();

        assert_eq!(
            error,
            DeskwrightError::AppNotFrontmost("Google Chrome".to_owned())
        );
        assert!(desktop.recorded_actions().is_empty());
    }

    #[test]
    fn presses_into_the_frontmost_app_when_no_app_is_named() {
        let desktop = Arc::new(FakeDesktop::new());
        let notes = desktop.add_app("Notes", None);
        desktop.make_frontmost(notes);

        let app = keyboard_service(&desktop)
            .press(&KeyChord::parse("Escape").unwrap(), &AppSelector::Frontmost)
            .unwrap();

        assert_eq!(app.name, "Notes");
    }

    #[test]
    fn types_text_into_the_frontmost_target_app() {
        let desktop = Arc::new(FakeDesktop::new());
        let whatsapp = desktop.add_app("WhatsApp", None);
        desktop.make_frontmost(whatsapp);
        let target = AppSelector::NameOrBundleId("WhatsApp".to_owned());

        let app = keyboard_service(&desktop).type_text("hi", &target).unwrap();

        assert_eq!(app.name, "WhatsApp");
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::TypeText("hi".to_owned())]
        );
    }

    #[test]
    fn refuses_to_type_into_a_background_app_or_type_nothing() {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.add_app("WhatsApp", None);
        let target = AppSelector::NameOrBundleId("WhatsApp".to_owned());

        let background = keyboard_service(&desktop)
            .type_text("hi", &target)
            .unwrap_err();
        let empty = keyboard_service(&desktop)
            .type_text("", &AppSelector::Frontmost)
            .unwrap_err();

        assert_eq!(
            background,
            DeskwrightError::AppNotFrontmost("WhatsApp".to_owned())
        );
        assert!(empty.to_string().contains("no text to type"));
        assert!(desktop.recorded_actions().is_empty());
    }
}
