use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: usize,
    pub height: usize,
}

impl Default for Dimensions {
    fn default() -> Self {
        Dimensions {
            width: 1920,
            height: 1080,
        }
    }
}

impl Dimensions {
    pub fn new(width: usize, height: usize) -> Dimensions {
        Dimensions { width, height }
    }
    pub fn parse_dimensions(dimensions: &str) -> Result<Dimensions, &str> {
        let d: Vec<&str> = dimensions.split('x').collect();
        if let [width, height] = d[..] {
            return Ok(Dimensions {
                width: width.parse::<usize>().expect("Invalid width!"),
                height: height.parse::<usize>().expect("Invalid height!"),
            });
        }
        Err("Invalid dimensions!")
    }

    pub fn as_string(&self) -> (String, String) {
        (self.width.to_string(), self.height.to_string())
    }

    pub fn save_dimensions_from_string(
        &mut self,
        dimensions: (String, String),
    ) -> Result<(), &str> {
        let (width, height) = dimensions;
        if let (Ok(width), Ok(height)) = (width.parse::<usize>(), height.parse::<usize>()) {
            if width > 8000 || height > 8000 {
                return Err("Dimensions to big!");
            }
            self.width = width;
            self.height = height;
            return Ok(());
        }
        Err("Failed to parse dimensions")
    }
}

pub fn round_percent(numerator: u64, denominator: u64) -> f32 {
    round_digits(numerator as f32, denominator as f32, 100, 2)
}

pub fn round_digits(
    numerator: f32,
    denominator: f32,
    multiplicator: u8,
    decimal_places: u8,
) -> f32 {
    if denominator == 0. {
        return 0.;
    }
    (((numerator / denominator) * (multiplicator as f32 * (decimal_places * 10) as f32)).round())
        / (decimal_places * 10) as f32
}
