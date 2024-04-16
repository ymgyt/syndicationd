use std::{collections::HashMap, path::Path};

use anyhow::Context;
use ratatui::style::Color;
use serde::Deserialize;
use synd_feed::types::Category;

#[derive(Deserialize)]
pub struct Categories {
    categories: HashMap<String, Entry>,
    #[serde(skip)]
    aliases: HashMap<String, String>,
}

impl Categories {
    pub fn default_toml() -> Self {
        let s = include_str!("../../../../categories.toml");
        let mut c: Self = toml::from_str(s).unwrap();
        c.update_aliases();
        c
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let buf =
            std::fs::read_to_string(path).with_context(|| format!("path: {}", path.display()))?;
        let mut c: Self = toml::from_str(&buf)?;
        c.update_aliases();
        Ok(c)
    }

    pub fn icon(&self, category: &Category<'_>) -> Option<&Icon> {
        self.categories
            .get(category.as_str())
            .map(|entry| &entry.icon)
    }

    pub fn normalize(&self, category: Category<'static>) -> Category<'static> {
        match self.aliases.get(category.as_str()) {
            Some(normalized) => Category::new(normalized.to_owned()).unwrap_or(category),
            None => category,
        }
    }

    fn update_aliases(&mut self) {
        let new_map = self.categories.iter().fold(
            HashMap::with_capacity(self.categories.len()),
            |mut m, (category, entry)| {
                entry.aliases.iter().for_each(|alias| {
                    m.insert(alias.to_lowercase(), category.to_lowercase());
                });
                m
            },
        );

        self.aliases = new_map;
    }
}

#[derive(Deserialize)]
struct Entry {
    icon: Icon,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Icon {
    symbol: String,
    color: Option<IconColor>,
}

impl Icon {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            color: None,
        }
    }

    #[must_use]
    pub fn with_color(self, color: IconColor) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }

    pub fn symbol(&self) -> &str {
        self.symbol.as_str()
    }
    pub fn color(&self) -> Option<Color> {
        self.color.as_ref().and_then(IconColor::color)
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct IconColor {
    rgb: Option<u32>,
    // https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html#variant.Red
    name: Option<String>,
    #[serde(skip)]
    color: Option<Color>,
}

impl IconColor {
    pub fn new(color: Color) -> Self {
        Self {
            rgb: None,
            name: None,
            color: Some(color),
        }
    }
}

impl IconColor {
    fn color(&self) -> Option<Color> {
        self.color.or(self
            .rgb
            .as_ref()
            .map(|rgb| Color::from_u32(*rgb))
            .or(self.name.as_ref().and_then(|s| s.parse().ok())))
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
