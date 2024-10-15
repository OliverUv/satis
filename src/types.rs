use std::collections::HashMap;

use anyhow::anyhow;
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            belt_ipm: 480.0,
            pipe_ipm: 600.0,
            pref_multiple_constructor: 2.0,
            pref_multiple_assembler: 3.0,
            pref_multiple_manufacturer: 2.0,
            pref_multiple_refinery: 4.0,
            pref_multiple_foundry: 3.0,
            pref_multiple_packager: 4.0,
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
            // "Packager" => Some(self.pref_multiple_packager),
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
    pub fn print_calc(&self, state: &State) -> anyhow::Result<()> {
        println!("\n{:12}{:>39}", self.building, self.name);
        println!("\n  --  IN  --");
        print_ingredient(&self.in_1, None);
        print_ingredient(&self.in_2, None);
        print_ingredient(&self.in_3, None);
        print_ingredient(&self.in_4, None);
        println!("\n  -- OUT  --");
        print_ingredient(&self.out_1, None);
        print_ingredient(&self.out_2, None);
        println!("\n  -- CALC --");
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
        let in_out_modifier = clock * n_boxes * pref_mult;
        clock *= 100.0;

        if use_belt {
            println!("Max belt use: {:8}", max_belt);
        }
        if use_pipe {
            println!("Max pipe use: {:8}", max_pipe);
        }
        if use_belt {
            println!(
                "Num of {} per belt: {:8.4}",
                &self.building,
                m_per_belt,
            );
        }
        if use_pipe {
            println!(
                "Num of {} per pipe: {:8.4}",
                &self.building,
                m_per_pipe,
            );
        }

        println!("\n  --  BP  --");
        println!("{} [{:.0}]", self.name, n_boxes);
        println!("Num {} per instance: {}", self.building, pref_mult);
        println!("Clock: {:5.2} %", clock);
        println!("Out:");
        print_ingredient(&self.out_1, Some(in_out_modifier));
        print_ingredient(&self.out_2, Some(in_out_modifier));
        println!("In:");
        print_ingredient(&self.in_1, Some(in_out_modifier));
        print_ingredient(&self.in_2, Some(in_out_modifier));
        print_ingredient(&self.in_3, Some(in_out_modifier));
        print_ingredient(&self.in_4, Some(in_out_modifier));

        Ok(())
    }

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
}

fn print_ingredient(i: &Option<Ingredient>, modify: Option<f64>) {
    let i = match i {
        Some(i) => i,
        None => return,
    };
    let t = match i.transport() {
        Transport::Belt => "Belt",
        Transport::Pipe => "Pipe",
    };
    match modify {
        None => println!("({:4})  {:27} {:15.4}", t, i.part, i.quantity),
        Some(m) => println!("  {:24} {:7.2}", i.part, m * i.quantity),
    }
    
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
