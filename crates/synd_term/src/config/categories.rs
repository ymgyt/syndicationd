use std::collections::HashMap;

use ratatui::style::Color;
use serde::Deserialize;
use synd_feed::types::Category;

#[derive(Deserialize)]
pub struct Categories {
    categories: HashMap<String, Entry>,
}

impl Categories {
    pub fn default_toml() -> Self {
        let s = include_str!("../../../../categories.toml");
        toml::from_str(s).unwrap()
    }

    pub fn icon(&self, category: &Category<'_>) -> Option<&Icon> {
        self.categories
            .get(category.as_str())
            .map(|entry| &entry.icon)
    }
}

#[derive(Deserialize)]
struct Entry {
    icon: Icon,
}

#[derive(Deserialize)]
pub struct Icon {
    symbol: String,
    color: Option<IconColor>,
}

impl Icon {
    pub fn symbol(&self) -> &str {
        self.symbol.as_str()
    }
    pub fn color(&self) -> Option<Color> {
        self.color.as_ref().and_then(IconColor::color)
    }
}

#[derive(Deserialize, Default)]
struct IconColor {
    rgb: Option<u32>,
    // https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html#variant.Red
    name: Option<String>,
}

impl IconColor {
    fn color(&self) -> Option<Color> {
        self.rgb
            .as_ref()
            .map(|rgb| Color::from_u32(*rgb))
            .or(self.name.as_ref().and_then(|s| s.parse().ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_default_toml() {
        let c = Categories::default_toml();
        let icon = c.icon(&Category::new("rust").unwrap()).unwrap();

        assert_eq!(icon.symbol(), "");
        assert_eq!(icon.color(), Some(Color::Rgb(247, 76, 0)));
    }
}
