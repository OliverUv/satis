use std::fmt::Display;

use anyhow::{anyhow, bail, Result};
use serde::{ Serialize, Deserialize };

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Transport {
    Belt,
    Pipe,
}

impl Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transport::Belt => write!(f, "Belt"),
            Transport::Pipe => write!(f, "Pipe"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub belt_ipm: f64,
    pub pipe_ipm: f64,
    pub pref_multiple_assembler: f64,
    pub pref_multiple_blender: f64,
    pub pref_multiple_constructor: f64,
    pub pref_multiple_converter: f64,
    pub pref_multiple_foundry: f64,
    pub pref_multiple_manufacturer: f64,
    pub pref_multiple_nuclear_power_plant: f64,
    pub pref_multiple_packager: f64,
    pub pref_multiple_particle_accelerator: f64,
    pub pref_multiple_refinery: f64,
    pub pref_multiple_smelter: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            belt_ipm: 780.0,
            // belt_ipm: 480.0,
            pipe_ipm: 600.0,
            // pipe_ipm: 300.0,
            pref_multiple_assembler: 3.0,
            pref_multiple_blender: 2.0,
            pref_multiple_constructor: 3.0,
            pref_multiple_converter: 1.0,
            pref_multiple_foundry: 3.0,
            pref_multiple_manufacturer: 2.0,
            pref_multiple_nuclear_power_plant: 1.0,
            pref_multiple_packager: 4.0,
            pref_multiple_particle_accelerator: 1.0,
            pref_multiple_smelter: 4.0,
            pref_multiple_refinery: 4.0,
        }
    }
}

impl State {
    pub fn prefered_building_multiple(&self, building: &str) -> Option<f64> {
        match building {
            "Assembler" => Some(self.pref_multiple_assembler),
            "Blender" => Some(self.pref_multiple_blender),
            "Constructor" => Some(self.pref_multiple_constructor),
            "Converter" => Some(self.pref_multiple_converter),
            "Foundry" => Some(self.pref_multiple_foundry),
            "Manufacturer" => Some(self.pref_multiple_manufacturer),
            "Packager" => Some(self.pref_multiple_packager),
            "Particle Accelerator" => Some(self.pref_multiple_particle_accelerator),
            "Smelter" => Some(self.pref_multiple_smelter),
            "Refinery" => Some(self.pref_multiple_refinery),
            "Nuclear Power Plant" => Some(self.pref_multiple_nuclear_power_plant),
            _ => None,
        }
    }
}

pub type RecipeCollection = Vec<Recipe>;

pub fn recipe_by_name_mut<'a, 'b>(col: &'a mut RecipeCollection, name: &'b str) -> Option<&'a mut Recipe> {
    col.iter_mut().find(|r| r.name == name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub building: String,
    pub name: String,
    pub craft_time_s: f64,
    pub is_alt: bool,
    pub unlocks: String,
    pub is_unlocked: bool,
    // Quanitites of ingredients is specified per minute
    pub in_1: Option<Ingredient>,
    pub in_2: Option<Ingredient>,
    pub in_3: Option<Ingredient>,
    pub in_4: Option<Ingredient>,
    pub out_1: Option<Ingredient>,
    pub out_2: Option<Ingredient>,
}

impl Recipe {

    pub fn inputs(&self) -> impl Iterator<Item=&Ingredient> {
        [
            self.in_1.as_ref(),
            self.in_2.as_ref(),
            self.in_3.as_ref(),
            self.in_4.as_ref(),
        ].into_iter().filter_map(|i| i)
    }

    pub fn outputs(&self) -> impl Iterator<Item=&Ingredient> {
        [
            self.out_1.as_ref(),
            self.out_2.as_ref(),
        ].into_iter().filter_map(|i| i)
    }

    pub fn ingredients(&self) -> impl Iterator<Item=&Ingredient> {
        [
            self.in_1.as_ref(),
            self.in_2.as_ref(),
            self.in_3.as_ref(),
            self.in_4.as_ref(),
            self.out_1.as_ref(),
            self.out_2.as_ref(),
        ].into_iter().filter_map(|i| i)
    }

    pub fn max_outputs(&self) -> (f64, f64) {
        let mut belt = 0.0;
        let mut pipe = 0.0;
        let max_ing = |i: &Ingredient| {
            let belt_pipe = match i.transport() {
                Transport::Belt => &mut belt,
                Transport::Pipe => &mut pipe,
            };
            if *belt_pipe < i.quantity {
                *belt_pipe = i.quantity;
            }
        };

        self.ingredients().for_each(max_ing);

        (belt, pipe)
    }

    pub fn suggest_blueprint(&self, state: &State) -> Result<BlueprintSuggestion> {
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

        Ok(BlueprintSuggestion {
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

pub struct BlueprintSuggestion {
    pub use_belt: bool,
    pub use_pipe: bool,
    pub m_per_belt: f64,
    pub m_per_pipe: f64,
    pub n_boxes: f64,
    pub pref_mult: f64,
    pub clock: f64,
    pub power_usage_mw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub part: String,
    pub quantity: f64,
}

impl Ingredient {

    #[must_use]
    pub fn same_type_as(&self, other: &Ingredient) -> bool {
        self.part == other.part
    }

    pub fn same_type(&self, other_part: &str) -> bool {
        self.part.to_lowercase() == other_part.to_lowercase()
    }

    #[must_use]
    pub fn neg(&self) -> Self {
        Ingredient {
            part: self.part.clone(),
            quantity: -self.quantity,
        }
    }

    #[must_use]
    pub fn scale(&self, scalar: f64) -> Ingredient {
        Ingredient {
            part: self.part.clone(),
            quantity: self.quantity * scalar,
        }
    }

    pub fn merge(&mut self, other: &Ingredient) {
        if self.part != other.part { panic!("Tried to merge {} with {}", self.part, other.part) }
        self.quantity = self.quantity + other.quantity;
    }

    pub fn merge_with(&self, others: &mut Vec<Ingredient>) {
        let has_same = others.iter_mut().find(|o| o.part == self.part);
        if let Some(o) = has_same {
            o.merge(self);
        } else {
            others.push(self.clone());
        }
    }

    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.quantity <= 0.0
    }

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
            "Excited Photonic Matter" => Transport::Pipe,
            "Dark Matter Residue" => Transport::Pipe,
            _ => Transport::Belt,
        }
    }

}

/// Returns the power usage in MW if possible.
/// TODO support variable power usage of Particle Accelerator and Converter
fn calc_power_usage_mw(building: &str, clock: f64) -> Result<f64> {
    let base_power_usage = match building {
        "Assembler" => 15.0,
        "Blender" => 75.0,
        "Constructor" => 4.0,
        "Converter" => 1.0, // fake
        "Foundry" => 16.0,
        "Manufacturer" => 55.0,
        "Packager" => 10.0,
        "Refinery" => 30.0,
        "Smelter" => 4.0,
        "Particle Accelerator" => 1.0, // fake
        "Nuclear Power Plant" => -2500.0, // fake ish
        _ => bail!("Building {} has no defined base power usage.", building),
    };

    if clock <= 0.0 { bail!("Clock speed must no be less than 0"); }
    if clock >= 2.5 { bail!("Clock speed must no be more than 2.5"); }

    Ok(base_power_usage * clock.powf(1.321928))
}
