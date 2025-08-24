use std::error::Error;

use rusttype::{Font, Scale, VMetrics};

pub enum FontStyle {
    Roman,
    Italic,
}

pub enum FontWeight {
    Normal,
    Bold,
}

pub struct FontAndMetadata<'a> {
    pub font: Font<'a>,
    pub v_metrics: VMetrics,
    pub space_width: i32,
}

pub struct BrowserFont<'a> {
    pub roman: FontAndMetadata<'a>,
    pub italic: FontAndMetadata<'a>,
    pub bold: FontAndMetadata<'a>,
    pub bold_italic: FontAndMetadata<'a>,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
}

impl BrowserFont<'static> {
    pub fn get_font_and_metadata(&self) -> &FontAndMetadata<'static> {
        match (&self.font_style, &self.font_weight) {
            (FontStyle::Roman, FontWeight::Normal) => &self.roman,
            (FontStyle::Roman, FontWeight::Bold) => &self.bold,
            (FontStyle::Italic, FontWeight::Normal) => &self.italic,
            (FontStyle::Italic, FontWeight::Bold) => &self.bold_italic,
        }
    }

    pub fn set_weight(&mut self, weight: FontWeight) {
        self.font_weight = weight;
    }

    pub fn set_style(&mut self, style: FontStyle) {
        self.font_style = style;
    }
    
    pub fn current_height(&self) -> i32 {
        self.get_font_and_metadata().v_metrics.ascent.floor() as i32

    }

    pub fn load(scale: Scale) -> Result<Self, Box<dyn Error>> {
        let roman_data = include_bytes!("../../assets/b612/B612-Regular.ttf");
        let roman = Font::try_from_bytes(roman_data).ok_or("Error loading regular")?;

        let italic_data = include_bytes!("../../assets/b612/B612-Italic.ttf");
        let italic = Font::try_from_bytes(italic_data).ok_or("Error loading italic")?;

        let bold_data = include_bytes!("../../assets/b612/B612-Bold.ttf");
        let bold = Font::try_from_bytes(bold_data).ok_or("Error loading bold")?;

        let bold_italic_data = include_bytes!("../../assets/b612/B612-BoldItalic.ttf");
        let bold_italic =
            Font::try_from_bytes(bold_italic_data).ok_or("Error loading bold italic")?;

        return Ok(BrowserFont {
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Roman,
            roman: FontAndMetadata {
                v_metrics: roman.v_metrics(scale).clone(),
                space_width: roman
                    .glyph(' ')
                    .scaled(scale)
                    .h_metrics()
                    .advance_width
                    .floor() as i32,
                font: roman,
            },
            italic: FontAndMetadata {
                v_metrics: italic.v_metrics(scale).clone(),
                space_width: italic
                    .glyph(' ')
                    .scaled(scale)
                    .h_metrics()
                    .advance_width
                    .floor() as i32,
                font: italic,
            },
            bold: FontAndMetadata {
                v_metrics: bold.v_metrics(scale).clone(),
                space_width: bold
                    .glyph(' ')
                    .scaled(scale)
                    .h_metrics()
                    .advance_width
                    .floor() as i32,
                font: bold,
            },
            bold_italic: FontAndMetadata {
                v_metrics: bold_italic.v_metrics(scale).clone(),
                space_width: bold_italic
                    .glyph(' ')
                    .scaled(scale)
                    .h_metrics()
                    .advance_width
                    .floor() as i32,
                font: bold_italic,
            },
        });
    }
}
