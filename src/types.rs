use std::collections::HashMap;

use anyhow::{anyhow, bail};
use serde::{ Serialize, Deserialize };

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Transport {
    Belt,
    Pipe,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub belt_ipm: f64,
    pub pipe_ipm: f64,
    pub pref_multiple_constructor: f64,
    pub pref_multiple_assembler: f64,
    pub pref_multiple_manufacturer: f64,
    pub pref_multiple_refinery: f64,
    pub pref_multiple_foundry: f64,
    pub pref_multiple_packager: f64,
    pub pref_multiple_blender: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            belt_ipm: 780.0, // 480, ..
            pipe_ipm: 600.0,
            pref_multiple_assembler: 3.0,
            pref_multiple_blender: 4.0,
            pref_multiple_constructor: 3.0,
            pref_multiple_foundry: 3.0,
            pref_multiple_manufacturer: 2.0,
            pref_multiple_packager: 4.0,
            pref_multiple_refinery: 4.0,
        }
    }
}

impl State {
    pub fn prefered_building_multiple(&self, building: &str) -> Option<f64> {
        match building {
            "Constructor" => Some(self.pref_multiple_constructor),
            "Assembler" => Some(self.pref_multiple_assembler),
            "Manufacturer" => Some(self.pref_multiple_manufacturer),
            "Refinery" => Some(self.pref_multiple_refinery),
            "Foundry" => Some(self.pref_multiple_foundry),
            "Blender" => Some(self.pref_multiple_blender),
            "Packager" => Some(self.pref_multiple_packager),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipes {
    pub recipes: RecipeMap,
}
pub type RecipeMap = HashMap<String, Recipe>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub building: String,
    pub name: String,
    pub craft_time: f64,
    pub is_alt: bool,
    pub unlocks: String,
    pub is_unlocked: bool,
    pub in_1: Option<Ingredient>,
    pub in_2: Option<Ingredient>,
    pub in_3: Option<Ingredient>,
    pub in_4: Option<Ingredient>,
    pub out_1: Option<Ingredient>,
    pub out_2: Option<Ingredient>,
}

impl Recipe {
    pub fn max_outputs(&self) -> (f64, f64) {
        let mut belt = 0.0;
        let mut pipe = 0.0;
        let mut max_ing = |i: &Option<Ingredient>| {
            let i = match i {
                Some(i) => i,
                None => return,
            };
            let belt_pipe = match i.transport() {
                Transport::Belt => &mut belt,
                Transport::Pipe => &mut pipe,
            };
            if *belt_pipe < i.quantity {
                *belt_pipe = i.quantity;
            }
        };
        max_ing(&self.in_1);
        max_ing(&self.in_2);
        max_ing(&self.in_3);
        max_ing(&self.in_4);
        max_ing(&self.out_1);
        max_ing(&self.out_2);

        (belt, pipe)
    }

    pub fn calc(&self, state: &State) -> anyhow::Result<RecipeCalc> {
        let (max_belt, max_pipe) = self.max_outputs();
        let use_belt = max_belt >= 0.00001;
        let use_pipe = max_pipe >= 0.00001;
        let m_per_belt = state.belt_ipm / max_belt;
        let m_per_pipe = state.pipe_ipm / max_pipe;
        let m_per_transport = if use_belt && use_pipe {
            m_per_belt.min(m_per_pipe)
        } else if use_belt {
            m_per_belt
        } else {
            m_per_pipe
        };

        let pref_mult = state.prefered_building_multiple(self.building.as_str()).ok_or(anyhow!("Please state a prefered number of machines for {}", &self.building))?;
        let mut n_boxes = m_per_transport / pref_mult;
        let mut clock = 1.0;
        if n_boxes.fract().abs() > 0.0001 {
            // need to +1 the amount of boxes and adjust clocks
            let n_boxes_adjusted = n_boxes.ceil();
            clock = n_boxes / n_boxes_adjusted;
            n_boxes = n_boxes_adjusted;
        }

        let power_usage_mw = n_boxes * pref_mult * calc_power_usage_mw(self.building.as_str(), clock)?;

        Ok(RecipeCalc {
            use_belt,
            use_pipe,
            m_per_belt,
            m_per_pipe,
            n_boxes,
            pref_mult,
            clock,
            power_usage_mw,
        })
    }
}

pub struct RecipeCalc {
    pub use_belt: bool,
    pub use_pipe: bool,
    pub m_per_belt: f64,
    pub m_per_pipe: f64,
    pub n_boxes: f64,
    pub pref_mult: f64,
    pub clock: f64,
    pub power_usage_mw: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingredient {
    pub part: String,
    pub quantity: f64,
}

impl Ingredient {
    pub fn transport(&self) -> Transport {
        match self.part.as_str() {
            "Alumina Solution" => Transport::Pipe,
            "Fuel" => Transport::Pipe,
            "Heavy Oil Residue" => Transport::Pipe,
            "Ionised Fuel" => Transport::Pipe,
            "Liquid Biofuel" => Transport::Pipe,
            "Nitric Acid" => Transport::Pipe,
            "Nitrogen Gas" => Transport::Pipe,
            "Crude Oil" => Transport::Pipe,
            "Rocket Fuel" => Transport::Pipe,
            "Sulfuric Acid" => Transport::Pipe,
            "Turbofuel" => Transport::Pipe,
            "Water" => Transport::Pipe,
            _ => Transport::Belt,
        }
    }
}

/// Returns the power usage in MW if possible
fn calc_power_usage_mw(building: &str, clock: f64) -> anyhow::Result<f64> {
    let base_power_usage = match building {
        "Assembler" => 15.0,
        "Blender" => 75.0,
        "Constructor" => 4.0,
        "Foundry" => 16.0,
        "Manufacturer" => 55.0,
        "Packager" => 10.0,
        "Refinery" => 30.0,
        "Smelter" => 4.0,
        _ => bail!("Building {} has no defined base power usage.", building),
    };

    if clock <= 0.0 { bail!("Clock speed must no be less than 0"); }
    if clock >= 2.5 { bail!("Clock speed must no be more than 2.5"); }

    Ok(base_power_usage * clock.powf(1.321928))
}
