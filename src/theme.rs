use cliclack::{Theme, ThemeState};
use console::{Style, style};

pub fn install() {
    cliclack::set_theme(AppTheme);
}

const VIOLET: (u8, u8, u8) = (0xFF, 0x87, 0xFF);
const VIOLET_LIGHT: (u8, u8, u8) = (0xFF, 0xB3, 0xFF);
const LIME: (u8, u8, u8) = (0xB1, 0xFF, 0x87);
const CORAL: (u8, u8, u8) = (0xFF, 0x8E, 0x87);
const BUTTER: (u8, u8, u8) = (0xFF, 0xE5, 0x87);
const DIM_GREY: (u8, u8, u8) = (0x70, 0x66, 0x77);

struct AppTheme;

impl Theme for AppTheme {
    fn bar_color(&self, state: &ThemeState) -> Style {
        match state {
            ThemeState::Active => {
                Style::new().true_color(VIOLET_LIGHT.0, VIOLET_LIGHT.1, VIOLET_LIGHT.2)
            }
            ThemeState::Submit => Style::new().true_color(VIOLET.0, VIOLET.1, VIOLET.2),
            ThemeState::Cancel => Style::new().red(),
            ThemeState::Error(_) => Style::new().true_color(CORAL.0, CORAL.1, CORAL.2),
        }
    }

    fn state_symbol_color(&self, state: &ThemeState) -> Style {
        match state {
            ThemeState::Active | ThemeState::Submit => {
                Style::new().true_color(VIOLET.0, VIOLET.1, VIOLET.2)
            }
            _ => self.bar_color(state),
        }
    }

    fn radio_symbol(&self, state: &ThemeState, selected: bool) -> String {
        match state {
            ThemeState::Active if selected => style("❯")
                .true_color(LIME.0, LIME.1, LIME.2)
                .to_string(),
            ThemeState::Active => " ".to_string(),
            _ => String::new(),
        }
    }

    fn error_symbol(&self) -> String {
        style("■")
            .true_color(CORAL.0, CORAL.1, CORAL.2)
            .to_string()
    }

    fn warning_symbol(&self) -> String {
        style("▲")
            .true_color(BUTTER.0, BUTTER.1, BUTTER.2)
            .to_string()
    }

    fn info_symbol(&self) -> String {
        style("●")
            .true_color(BUTTER.0, BUTTER.1, BUTTER.2)
            .to_string()
    }

    fn placeholder_style(&self, state: &ThemeState) -> Style {
        match state {
            ThemeState::Cancel => Style::new().hidden(),
            _ => Style::new().true_color(DIM_GREY.0, DIM_GREY.1, DIM_GREY.2),
        }
    }
}
