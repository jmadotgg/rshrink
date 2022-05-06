use std::num::ParseIntError;
#[derive(Debug, Clone)]
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
        let d: Vec<&str> = dimensions.split("x").collect();
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
    ) -> Result<(), ParseIntError> {
        let (width, height) = dimensions;
        self.width = width.parse::<usize>()?;
        self.height = height.parse::<usize>()?;
        Ok(())
    }
}
